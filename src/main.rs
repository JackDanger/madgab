//! `madgab` — Mad Gab puzzle generator CLI.
//!
//! Reads a target English phrase, prints ranked Mad-Gab-style clue
//! candidates: phrases whose IPA matches the target but whose words
//! re-syllabify the phoneme stream.
//!
//! Usage:
//!
//!     madgab "It's just a stupid game"
//!     madgab --top 20 --max-rarity 50000 "Coors light"
//!     madgab --transcribe "It's just a stupid game"   # IPA-only debug
//!
//! Build is a single binary that embeds the 15 MB transcription
//! corpus, so `madgab` runs anywhere without a separate data file.

use std::process::ExitCode;

use madgab::{Generator, GeneratorConfig, SearchMode};

/// The IPA transcription corpus, embedded at compile time.
/// 15 MB; expands the release binary correspondingly.
const CORPUS_JSON: &str = include_str!("../data/common_ipa_transcriptions.json");

const USAGE: &str = "\
madgab — Mad Gab puzzle generator

Usage:
  madgab [options] <target phrase>

Options:
  --top N            Return top N candidates (default 10).
  --max-rarity R     Drop corpus words rarer than R (default 20000).
  --beam K           Beam width during search (default 64).
  --min-word-len N   Skip clue words with fewer than N IPA chars (default 2).
  --transcribe       Print the target's IPA stream and exit.
  --help             This message.
";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).collect::<Vec<String>>();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let mut config = GeneratorConfig {
        mode: SearchMode::Exact,
        ..GeneratorConfig::default()
    };
    let mut transcribe_only = false;

    while let Some(flag) = args.first().cloned() {
        match flag.as_str() {
            "--top" => {
                args.remove(0);
                let v = args.remove(0).parse::<usize>().unwrap_or(config.top_n);
                config.top_n = v;
            }
            "--max-rarity" => {
                args.remove(0);
                let v = args.remove(0).parse::<f64>().ok();
                config.max_rarity = v;
            }
            "--beam" => {
                args.remove(0);
                config.beam_width = args.remove(0).parse::<usize>().unwrap_or(config.beam_width);
            }
            "--min-word-len" => {
                args.remove(0);
                config.min_word_ipa_chars = args.remove(0).parse::<usize>().unwrap_or(config.min_word_ipa_chars);
            }
            "--transcribe" => {
                args.remove(0);
                transcribe_only = true;
            }
            other if other.starts_with("--") => {
                eprintln!("madgab: unknown flag {other:?}");
                return ExitCode::from(2);
            }
            _ => break,
        }
    }

    if args.is_empty() {
        eprintln!("madgab: missing <target phrase>\n\n{USAGE}");
        return ExitCode::from(2);
    }

    let target = args.join(" ");

    // Loading the corpus dominates startup (~1s). Don't do it in the
    // --transcribe path if we don't need to — but we do need it
    // because transcribe() uses the corpus, so just press on.
    let started_load = std::time::Instant::now();
    let generator = match Generator::from_json(CORPUS_JSON, config.clone()) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("madgab: failed to load corpus: {e}");
            return ExitCode::from(1);
        }
    };
    let load_ms = started_load.elapsed().as_millis();

    if transcribe_only {
        match generator.corpus().transcribe(&target) {
            Some(ipa) => {
                println!("{ipa}");
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("madgab: at least one word in {target:?} has no transcription");
                ExitCode::from(1)
            }
        }
    } else {
        let started_search = std::time::Instant::now();
        let clues = generator.generate(&target);
        let search_ms = started_search.elapsed().as_millis();

        if clues.is_empty() {
            eprintln!("madgab: no clue coverings found for {target:?}");
            eprintln!("  (try lowering --min-word-len or raising --max-rarity)");
            return ExitCode::from(1);
        }

        let ipa = generator
            .corpus()
            .transcribe(&target)
            .unwrap_or_else(|| "?".to_string());
        eprintln!("target: {target}");
        eprintln!("IPA:    /{ipa}/");
        eprintln!("(corpus loaded in {load_ms}ms; search {search_ms}ms)");
        eprintln!();

        for (i, clue) in clues.iter().enumerate() {
            println!("{:2}. [{:.3}] {}", i + 1, clue.score, clue.phrase);
        }
        ExitCode::SUCCESS
    }
}
