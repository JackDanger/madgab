# Plan — open-licensed English IPA transcription corpus

## Goal

A reproducible, permissively-licensed dataset of English words → IPA
pronunciations that:

1. **Starts from the best public-domain source** (CMU Pronouncing
   Dictionary) and *fixes* the ASCII-IPA encoding loss the
   current `common_ipa_transcriptions.json` inherited.
2. **Supersets every record** with cross-validated alternative
   transcriptions from other public IPA sources.
3. **Generates fallback transcriptions** for words missing from all
   curated sources, via G2P (grapheme-to-phoneme) tooling.
4. **Carries frequency information** so consumers can prefer common
   vocabulary (essential for downstream Mad Gab clue quality).
5. **Has a fully reproducible build** — anyone can re-derive the
   corpus from versioned upstream inputs with a single `make build`.

The dataset replaces the existing 15 MB JSON for both `phonetics-rs`
consumers and the `madgab` generator. Downstream switches via a single
file replacement.

## Why this matters

The current corpus's 90% CMU coverage uses an ASCII-only encoding that
collapses /i/-/ɪ/, /u/-/ʊ/, and stress information. Every consequent
search in `madgab` operates on a mangled phoneme stream. The new
corpus restores the precision the original ARPABET notation always
had — re-deriving from CMU's source data does the work.

## Source inventory

| Tier | Source | Coverage | License | Format |
|---|---|---|---|---|
| 1 | **[CMUdict](https://github.com/cmusphinx/cmudict)** | ~134k words | Public domain | ARPABET + stress |
| 2 | **[WikiPron](https://github.com/CUNY-CL/wikipron)** | ~1.2M lines (en-us, en-uk, en-au) | CC-BY-SA 4.0 | IPA |
| 2 | **[Wiktionary](https://kaikki.org/dictionary/English/)** | 100k+ entries | CC-BY-SA 4.0 | IPA, parsed by Tatu Ylönen |
| 3 | **[eSpeak-NG](https://github.com/espeak-ng/espeak-ng)** | unlimited (G2P) | GPL-3.0 (tool); output is data | IPA |
| 3 | **g2p-en** | unlimited (model) | MIT | ARPABET |
| 4 | **[SUBTLEX-US](https://www.ugent.be/pp/experimentele-psychologie/en/research/documents/subtlexus)** | 74k frequencies | Free for research | CSV |
| 4 | **[Google Books Ngram](https://storage.googleapis.com/books/ngrams/books/datasetsv3.html)** | millions | CC-BY | TSV |

**License decision: dataset ships under CC-BY-SA 4.0**, inherited from
WikiPron and Wiktionary. CMU+eSpeak-only subset can be released under
CC0 as a "minimal" build if someone needs full public domain.

## Build pipeline

```
┌─────────────┐     ┌──────────────────┐     ┌────────────────┐
│ CMUdict     │────►│ ARPABET → IPA    │────►│                │
│ (raw)       │     │ canonical mapper │     │                │
└─────────────┘     └──────────────────┘     │                │
                                              │                │
┌─────────────┐     ┌──────────────────┐     │                │
│ WikiPron    │────►│ validate + dedupe│────►│ merger         │────► corpus.json
│ (raw .tsv)  │     │                  │     │ (with provenance)
└─────────────┘     └──────────────────┘     │                │
                                              │                │
┌─────────────┐     ┌──────────────────┐     │                │
│ vocabulary  │────►│ eSpeak-NG G2P    │────►│                │
│ gap list    │     │ (for misses)     │     │                │
└─────────────┘     └──────────────────┘     │                │
                                              │                │
┌─────────────┐     ┌──────────────────┐     │                │
│ SUBTLEX-US  │────►│ frequency join   │────►│                │
│             │     │                  │     │                │
└─────────────┘     └──────────────────┘     └────────────────┘
                                                       │
                                                       ▼
                                              ┌────────────────┐
                                              │ validate +     │
                                              │ compile to bin │
                                              └────────────────┘
                                                       │
                                                       ▼
                                              corpus.bin (compact)
```

### Stage 1 — CMUdict, properly converted

Parse CMUdict's ARPABET. Apply the canonical mapping table:

```
AA → ɑ      EH → ɛ      L  → l       SH → ʃ
AE → æ      ER → ɝ/ɚ    M  → m       T  → t
AH → ʌ/ə    EY → eɪ     N  → n       TH → θ
AO → ɔ      F  → f      NG → ŋ       UH → ʊ
AW → aʊ     G  → g      OW → oʊ      UW → u
AY → aɪ     HH → h      OY → ɔɪ      V  → v
B  → b      IH → ɪ      P  → p       W  → w
CH → tʃ     IY → i      R  → ɹ       Y  → j
D  → d      JH → dʒ     S  → s       Z  → z
DH → ð      K  → k                   ZH → ʒ
```

Stress markers (0, 1, 2) appear suffixed on ARPABET vowels. Use them to:

- Disambiguate `AH` → /ʌ/ (stressed: 1 or 2) vs /ə/ (unstressed: 0)
- Disambiguate `ER` → /ɝ/ (stressed) vs /ɚ/ (unstressed)
- Emit primary-stress markers (`ˈ`) on the syllable carrying `1` stress

CMUdict ships variants like `HOMER(1)` for alternative pronunciations.
Each becomes a separate variant in the output.

**Expected size at stage 1: ~140k entries (134k words × ~1.05 variants average)**

### Stage 2 — WikiPron supplements

WikiPron is the cleanest large-scale IPA source. Already parsed,
deduplicated, multi-dialect. For each word:

- If CMU has it, add WikiPron's transcriptions as *additional variants*
  tagged with `source: wikipron-en-us` etc.
- If CMU lacks it, add the WikiPron entry as the primary.

Validate each transcription before insertion:

- Every character must be in the IPA inventory `phonetics-rs` recognizes
- Strip surrounding `/…/`, `[…]`
- Reject entries with non-IPA characters or unusual length

**Expected size at stage 2: ~280k entries**

### Stage 3 — eSpeak-NG fallback

For target vocabulary (top 100k SUBTLEX-US words) that has no curated
transcription, shell out to eSpeak-NG:

```
echo "$word" | espeak-ng -q --ipa
```

Tag the entry with `source: espeak-ng-1.51` and `confidence: synthetic`
so consumers can downweight. eSpeak-NG's English G2P is good but not
human-level; downweighting is honest.

**Expected size at stage 3: ~320k entries**

### Stage 4 — Frequency data

Join SUBTLEX-US (movie-subtitle frequencies, more conversational than
book frequencies) and emit `frequency_per_million`. For words not in
SUBTLEX, fall back to Google Books Ngram (less conversational but
broader vocabulary).

Compute a derived `rarity` rank (lower = more common) for backward
compatibility with the current `phonetics-rs` `max_rarity` API.

### Stage 5 — Validate and compile

- Round-trip every entry through `phonetics::tokens(ipa)` to confirm
  it tokenizes cleanly
- Drop entries with unrecognized IPA chars after best-effort cleanup
- Emit warnings (not errors) for entries where multiple sources
  disagree by more than a confusion-distance threshold
- Write `corpus.json` (pretty-printed) and `corpus.bin`
  (length-prefixed binary, ~5 MB for fast load)

## Output schema (JSON)

```json
{
  "cat": {
    "transcriptions": [
      {
        "ipa": "kæt",
        "source": "cmudict-0.7b",
        "stress_pattern": "1",
        "variant_index": 0
      },
      {
        "ipa": "kʰæt",
        "source": "wikipron-en-us",
        "variant_index": 1
      }
    ],
    "frequency_per_million": 50.3,
    "rarity": 100,
    "alt_display": "CAT"
  }
}
```

## Output schema (binary)

Compact format for fast load (~5 MB target):

```
[u32 word_count]
[for each word:
   [u8 word_len] [bytes word]
   [u8 transcription_count]
   [for each transcription:
       [u8 ipa_len] [bytes ipa]
       [u8 source_id]      (lookup into source table)
       [u8 stress_pattern_len] [bytes stress_pattern]
   ]
   [f32 frequency_per_million]
]
```

## Repository structure

The corpus build lives in its own repo so it can be released and
versioned independently of consumers:

```
github.com/JackDanger/phonetic-corpus/
├── README.md
├── LICENSE                       CC-BY-SA 4.0
├── Makefile                      `make` invokes the whole pipeline
├── build/
│   ├── 01_cmudict.py             ARPABET → IPA conversion
│   ├── 02_wikipron.py            WikiPron ingest + validation
│   ├── 03_espeak.py              G2P fallback for gaps
│   ├── 04_frequency.py           SUBTLEX-US join
│   ├── 05_validate.py            phonetics-rs tokenizer round-trip
│   └── 06_compile.py             emit corpus.{json,bin}
├── inputs/                       gitignored
│   ├── cmudict-0.7b.txt          downloaded by Makefile
│   ├── wikipron-en-us.tsv
│   └── SUBTLEX-US.csv
├── data/
│   ├── corpus.json               released artifact
│   └── corpus.bin                released artifact
└── tests/
    ├── golden/                   100 hand-picked words for diff testing
    └── invariants.py             every IPA must round-trip through tokenizer
```

Releases (corpus.json + corpus.bin) are uploaded as GitHub Release
assets. Consumers either commit the artifact (small) or download it
during `cargo build` / package install.

## Migration plan for `madgab` / `phonetics-rs`

1. Build phonetic-corpus v0.1 (CMU-only, clean ARPABET conversion)
   and verify output quality on a hand-picked basket of words
   that currently break in madgab (`pie`, `it`, `stupid`).
2. Add WikiPron supplements; re-run madgab basket.
3. Decide whether to drop `phonetics-rs`'s
   `transcriptions::Corpus::from_json` in favor of a dedicated
   `phonetic_corpus` crate that ships the binary form.
4. Replace the 15 MB embedded JSON in `madgab` with the smaller
   binary form, fed through the new loader.
5. Update CORPUS_PLAN.md → CORPUS.md (or move it to the new repo)
   once the dataset exists.

## Quality validation

A "Mad Gab basket" of ~30 known target → clue pairs (from puzzle
books and crowdsourced lists) becomes the empirical test. After each
corpus iteration, re-run `madgab` against the basket and report:

- target IPA parses cleanly
- canonical clue is found in top-N
- discrimination ratio (clue similarity / decoy similarity)

The basket lives in this `madgab` repo (`spec/madgab_basket.json`)
and is the regression suite for both the corpus and the generator.

## Non-goals (for v0.1)

- **Stress mark precision beyond primary stress** — secondary stress
  is corpus-dependent and useful but not blocking
- **Multi-dialect output** — start with American English (CMU baseline);
  WikiPron's en-uk variants stay in the data but the default consumer
  picks en-us
- **IPA → English** reverse lookups beyond what
  `phonetics::transcriptions::Trie` already supports
- **Sub-second incremental rebuilds** — the full build is allowed
  to take ~30 minutes
- **Manual curation of every WikiPron entry** — we filter, we don't
  hand-edit
