# madgab

Mad Gab puzzle generator in Rust. Given an English target phrase, produces
ranked candidate **clues** — phrases whose IPA matches the target's but
whose word boundaries re-syllabify the phoneme stream into different
lexical sequences.

```
$ madgab "It's just a stupid game"
target: It's just a stupid game
IPA:    /itsdʒəstəstupidgeɪm/

 1. [0.520] its justa stu peed game
 2. [0.520] its jeet stir stupid game
 …
```

(*"Hits Justice Dupe Hid Came"* is the canonical Mad Gab clue. Whether the
generator can land on that exact one depends on the corpus — see *Quality
notes* below.)

## How it works

1. **Transcribe** the target into IPA via the 140k-word corpus that ships
   with the binary (preferred-source order: CMU → phonemicchart →
   Wiktionary).
2. **Beam-DP search** over the IPA stream: at each phoneme position, look up
   every English word in the trie that starts at this position, extend
   partial coverings, prune to the top K beams.
3. **Score** completed coverings by:
   - **boundary novelty** — how much the clue's word boundaries differ
     from the target's
   - **word novelty** — penalty for reusing words from the target
   - **length signal** — prefer fewer, longer clue words over many tiny
     ones (avoids "a a a a" coverings)

The phonetic similarity is by construction 1.0 because we require exact
phoneme coverage. The discrimination is all on the *lexical* axis.

## Build

```
cargo build --release
./target/release/madgab "I love you"
```

## CLI

```
madgab [options] <target phrase>

  --top N                  top N candidates (default 10)
  --max-rarity R           drop corpus words rarer than R (default 20000)
  --beam K                 beam width during search (default 64)
  --min-word-len N         skip clue words shorter than N IPA chars (default 2)
  --approximate            allow small phonetic substitutions (/t/→/d/,
                           /ɪ/→/i/, etc.) — finds homophone-style clues that
                           don't exactly cover the target's phonemes
  --per-word-budget COST   approximate mode: max sub cost per clue word
                           (default 0.5)
  --total-budget COST      approximate mode: max sub cost across the clue
                           (default 1.5)
  --transcribe             print target IPA and exit
  --help
```

### Search modes

**Exact** (default) — each clue word's IPA must exactly tile a span of
the target's IPA. Same phonemes, different words.

**Approximate** (`--approximate`) — each clue word's IPA may differ from
its span by a budgeted amount of phonetic-substitution cost. Finds
homophone-style clues: for "I love you" exact mode produces
`"i. le vue"`, `"eye le view"`; approximate mode finds `"i'll have yu"`.
See `src/lib.rs::SearchMode` for the underlying mechanism.

## Architecture

```
madgab/                                 ← this repo
├── data/common_ipa_transcriptions.json (15 MB, embedded into the binary)
├── src/lib.rs                          Generator + scoring
├── src/main.rs                         CLI
└── Cargo.toml                          → phonetics-rs (with transcriptions feature)

phonetics-rs (separate crate)           the per-phoneme distance metric and
                                        the trie/corpus types used here
```

Everything phonetic — the IPA tokenizer, vowel/consonant distance, listener-
confusion metric — lives in
[`phonetics-rs`](https://crates.io/crates/phonetics-rs). This crate is the
search layer on top.

## Quality notes

The embedded corpus has known limitations:

- **CMU's transcriptions use ASCII-only IPA** (writing `i` for both tense /i/
  and lax /ɪ/, `u` for both /u/ and /ʊ/, etc.). 90% of the corpus comes from
  CMU. Target transcriptions look "simplified" as a result.
- **Wiktionary has occasional broken entries** (e.g. "pie" → `piaɪi`).
- **The corpus only has 6,010 phonemicchart entries** of ~142k total, so for
  most words we're stuck with the CMU style.

This affects which clues the generator can find. Future work: re-derive
the corpus from a single high-quality source (epitran? phonemizer?) and
retire the multi-source heuristic.

## License

MIT.
