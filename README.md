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
madgab/                          ← this repo: search + scoring + CLI
├── src/lib.rs                   Generator, SearchMode, beam DP
├── src/main.rs                  CLI front-end
└── Cargo.toml                   → phonetics, open-english-pronouncing-dictionary

phonetics-rs                     the per-phoneme distance metric and the
                                 trie / Corpus / Pronunciation types

open-english-pronouncing-dictionary  the ~280k-word IPA dictionary this
("OpenEPD")                          generator searches; CC-BY-SA 4.0
```

Three crates, three concerns:

- **[phonetics-rs](https://crates.io/crates/phonetics-rs)** — MIT-licensed,
  language-neutral phonetic-distance math (Bark-space vowel distance, 2D
  consonant place embedding, Gotoh affine-gap listener-confusion DP) plus
  the corpus / trie data types. Stable.
- **[open-english-pronouncing-dictionary](https://crates.io/crates/open-english-pronouncing-dictionary)**
  (OpenEPD) — CC-BY-SA-4.0 IPA dictionary fused from Misaki US gold/silver,
  CMUdict 0.7b (ARPABET→IPA at build time), and WikiPron en_us broad, with
  per-word frequency-derived rarity. Lives in its own repo so the data has
  its own refresh cadence and other consumers (TTS, ASR, etc.) can depend
  on it without taking the Mad Gab generator along for the ride.
- **madgab** (this crate) — MIT-licensed, the beam-DP search that turns an
  English target phrase into ranked clue candidates by tiling its IPA
  stream with words from OpenEPD.

## License

MIT. The embedded IPA dictionary it consumes from OpenEPD is CC-BY-SA 4.0
— see that crate's README for the source attribution and the reasoning
behind the license.
