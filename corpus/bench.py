#!/usr/bin/env python3
"""
Quick benchmark comparing the new corpus against the prior one.

Reads `/tmp/old_corpus.json` (the previous build, extracted from git
history) and `data/common_ipa_transcriptions.json` (the new build),
then reports coverage, per-word-source breakdown, and quality metrics
on a fixed hand-curated set of Mad Gab target words.

Run after `build.py`:

    git show main:data/common_ipa_transcriptions.json > /tmp/old_corpus.json
    corpus/venv/bin/python corpus/bench.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

NEW_PATH = Path(__file__).resolve().parent.parent / "data" / "common_ipa_transcriptions.json"
OLD_PATH = Path("/tmp/old_corpus.json")

# Hand-curated check set: words from canonical Mad Gab puzzles.
# Each entry is (target_word, expected_substring_of_canonical_ipa).
# We don't insist on full IPA equality because IPA conventions differ
# across sources — we just want to know the corpus carries a
# transcription that contains the right vowel/consonant cues.
CHECK_SET = [
    ("the",     "ð"),
    ("a",       "ə"),       # weak form — CMU has this, Misaki may not
    ("just",    "dʒ"),
    ("stupid",  "stup"),    # "stoop-id" not "styoop-id" in US English
    ("game",    "ɡeɪ"),
    ("love",    "lʌv"),
    ("you",     "ju"),
    ("water",   "wɔ"),      # /wɑ/ or /wɔ/ acceptable
    ("hits",    "hɪts"),
    ("justice", "dʒʌs"),
    ("dupe",    "dup"),
    ("hid",     "hɪd"),
    ("came",    "keɪ"),
    ("phonetics", "fəˈne"),  # canonical primary stress on second syllable
    ("california", "kæl"),
    ("supercalifragilisticexpialidocious", ""),  # just want it to exist
]


def cover_stats(corpus: dict) -> dict:
    """Coverage by source."""
    stats: dict[str, int] = {}
    for w, entry in corpus.items():
        for src in entry.get("ipa", {}):
            stats[src] = stats.get(src, 0) + 1
    return stats


def best_ipa(entry: dict, preference: list[str]) -> str | None:
    """Mirror Corpus::preferred_ipa: exact-match-then-prefix per preference."""
    ipa_map = entry.get("ipa", {})
    if not ipa_map:
        return None
    for pref in preference:
        if pref in ipa_map:
            return ipa_map[pref]
        for src, ipa in ipa_map.items():
            if src.startswith(pref):
                return ipa
    return next(iter(ipa_map.values()))


# Mirror the phonetics-rs SOURCE_PREFERENCE for the post-fix release.
NEW_PREFERENCE = ["cmu", "misaki_gold", "misaki_silver", "phonemicchart", "wiktionary", "wikipron"]
OLD_PREFERENCE = ["cmu", "phonemicchart", "wiktionary"]


def main() -> int:
    new = json.loads(NEW_PATH.read_text())
    old = json.loads(OLD_PATH.read_text())

    print(f"old corpus: {len(old):>7} unique words, {OLD_PATH.stat().st_size/1e6:.1f} MB")
    print(f"new corpus: {len(new):>7} unique words, {NEW_PATH.stat().st_size/1e6:.1f} MB")
    print()

    print("source coverage (entries with that source present):")
    print("  old:")
    for src, n in sorted(cover_stats(old).items(), key=lambda kv: -kv[1]):
        print(f"    {src:18s} {n:>7}")
    print("  new:")
    for src, n in sorted(cover_stats(new).items(), key=lambda kv: -kv[1]):
        print(f"    {src:18s} {n:>7}")
    print()

    print("coverage delta:")
    only_new = len(new.keys() - old.keys())
    only_old = len(old.keys() - new.keys())
    common = len(new.keys() & old.keys())
    print(f"  in both:           {common:>7}")
    print(f"  only in new:       {only_new:>7}")
    print(f"  only in old:       {only_old:>7}")
    print()

    # Normalize for the substring check: strip stress marks, fold
    # ASCII `g` to IPA `ɡ` (some sources use one, some the other),
    # collapse the unstressed `ɝ`/`ɚ` pair to `ɚ`.
    def norm(s: str) -> str:
        return (s.replace("ˈ", "")
                  .replace("ˌ", "")
                  .replace("g", "ɡ"))

    print("check-set spot check (expected substring in preferred IPA, "
          "after stripping stress marks and folding g→ɡ):")
    print(f"  {'word':<35s} {'old':<25s} {'new':<25s} match?")
    matches_old = matches_new = 0
    for word, expected in CHECK_SET:
        old_ipa = best_ipa(old[word], OLD_PREFERENCE) if word in old else None
        new_ipa = best_ipa(new[word], NEW_PREFERENCE) if word in new else None
        ok_old = old_ipa is not None and (not expected or expected in norm(old_ipa))
        ok_new = new_ipa is not None and (not expected or expected in norm(new_ipa))
        matches_old += int(ok_old)
        matches_new += int(ok_new)
        flag_old = "✓" if ok_old else ("✗" if old_ipa else "—")
        flag_new = "✓" if ok_new else ("✗" if new_ipa else "—")
        print(f"  {word:<35s} {(old_ipa or '—'):<25s} {(new_ipa or '—'):<25s} "
              f"old:{flag_old} new:{flag_new}  exp:{expected!r}")
    print()
    print(f"check-set score: old={matches_old}/{len(CHECK_SET)}, "
          f"new={matches_new}/{len(CHECK_SET)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
