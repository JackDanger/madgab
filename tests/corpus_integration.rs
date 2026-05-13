//! Integration tests against the real shipped corpus.
//!
//! These complement the unit tests in `src/lib.rs` (which use a tiny
//! inline fixture) by exercising the full ~28 MB corpus. They run a
//! handful of canonical Mad Gab targets and assert specific
//! pronunciations are present — the kind of regression a corpus
//! rebuild could silently break.

use madgab::{Generator, GeneratorConfig, SearchMode};
use open_english_pronouncing_dictionary::CORPUS_JSON;

/// Generator with the full corpus loaded — no rarity cap. The
/// integration tests run probes against rare words like "dupe"
/// (rarity ~40k) that would be dropped under the CLI default of
/// 20k.
fn gen_unfiltered() -> Generator {
    Generator::from_json(
        CORPUS_JSON,
        GeneratorConfig {
            max_rarity: None,
            ..GeneratorConfig::default()
        },
    )
    .expect("corpus should parse")
}

#[test]
fn canonical_words_have_expected_narrow_ipa() {
    let g = gen_unfiltered();
    let c = g.corpus();
    // Each pair is (word, must-contain-substring-after-stress-strip).
    // We strip stress marks before checking because callers shouldn't
    // depend on whether `cmu` or `misaki_gold` wins preferred_ipa.
    let checks: &[(&str, &str)] = &[
        // Narrow vowel distinctions that the old ARPABET-derived
        // corpus collapsed:
        ("stupid",  "stup"),   // /u/, not "ASCII u" for both /u/ and /ʊ/
        ("hid",     "hɪd"),    // /ɪ/, not the collapsed /i/
        ("hits",    "hɪts"),
        ("justice", "dʒʌs"),   // full /ʌ/, not reduced to schwa
        ("love",    "lʌv"),
        // Affricates: dʒ/tʃ should appear as the two-character
        // sequence everywhere.
        ("just",    "dʒ"),
        ("chair",   "tʃ"),
        // Diphthongs:
        ("game",    "eɪ"),
        ("came",    "eɪ"),
        ("dupe",    "dup"),
        // Function-word weak forms — important because the target's
        // IPA stream is what the search must tile:
        ("a",       "ə"),
        ("the",     "ðə"),
    ];
    for (word, expected) in checks {
        let ipa = c.preferred_ipa(word)
            .unwrap_or_else(|| panic!("corpus must know {:?}", word));
        let stripped: String = ipa.chars()
            .filter(|&ch| ch != 'ˈ' && ch != 'ˌ')
            .collect();
        // Fold ASCII g → IPA ɡ so the check is independent of which
        // source happened to win preferred_ipa.
        let stripped = stripped.replace('g', "ɡ");
        assert!(
            stripped.contains(expected),
            "expected {:?} in preferred IPA of {:?}, got {:?}",
            expected, word, ipa,
        );
    }
}

#[test]
fn known_madgab_pair_is_searchable() {
    // The classic Mad Gab puzzle: "It's just a stupid game" → "Hits
    // Justice Dupe Hid Came". This test doesn't require the
    // generator to *prefer* the canonical clue — it just asserts the
    // corpus has every word the clue needs, so a player who guesses
    // it can be told they're right.
    let g = gen_unfiltered();
    let c = g.corpus();
    for w in ["it's", "just", "a", "stupid", "game",
              "hits", "justice", "dupe", "hid", "came"] {
        assert!(
            c.preferred_ipa(w).is_some(),
            "corpus missing canonical Mad Gab word {:?}", w,
        );
    }
}

#[test]
fn generates_for_a_real_phrase() {
    // End-to-end: the generator produces *some* candidate clues for
    // a small phrase. We don't assert what's at the top — scoring
    // tuning is its own concern — only that we get non-empty output.
    let g = Generator::from_json(
        CORPUS_JSON,
        GeneratorConfig {
            top_n: 5,
            ..GeneratorConfig::default()
        },
    ).unwrap();
    let clues = g.generate("I love you");
    assert!(!clues.is_empty(), "expected at least one clue for 'I love you'");
    // Every clue should have a non-empty phrase and a valid score.
    for c in &clues {
        assert!(!c.phrase.is_empty());
        assert!(c.score.is_finite() && (0.0..=1.0).contains(&c.score),
                "score out of [0,1]: {}", c.score);
    }
}

#[test]
fn approximate_mode_runs_end_to_end() {
    // Smoke test for approximate mode against the real corpus —
    // assert it produces well-formed clues, not that it produces
    // strictly more than exact (the beam pruning operates on a
    // larger candidate space, so the top-K returned can intersect
    // exact's top-K imperfectly).
    let g = Generator::from_json(
        CORPUS_JSON,
        GeneratorConfig {
            mode: SearchMode::approximate(),
            top_n: 10,
            ..GeneratorConfig::default()
        },
    ).unwrap();
    let clues = g.generate("I love you");
    assert!(!clues.is_empty(), "approximate mode found no clues");
    // Each clue has at least one word with a non-zero substitution
    // cost or the clue is identical to an exact-mode hit; either is
    // fine, we just want output.
    for c in &clues {
        assert!(!c.words.is_empty());
    }
}
