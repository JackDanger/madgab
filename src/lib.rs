//! Mad Gab puzzle generator.
//!
//! Mad Gab takes an English phrase and re-presents it as a sequence of
//! different English words whose concatenated pronunciation is similar
//! to the original. "It's just a stupid game" becomes "Hits Justice
//! Dupe Hid Came" — same phoneme stream, totally different lexical
//! parse.
//!
//! This crate's [`Generator`] takes a target phrase and returns a
//! ranked list of candidate clues. It leans on three capabilities
//! from `phonetics-rs`:
//!
//!   * `Corpus::transcribe` to turn the target into an IPA string
//!   * `Corpus::trie::words_starting_at` to enumerate every English
//!     word whose IPA matches a given prefix of the target
//!   * `phonetics::similarity` to score how close a candidate clue
//!     sounds to the target
//!
//! The search itself is a beam-DP over phoneme positions: at each
//! position we keep the best K coverings reachable so far, and at
//! each step we extend each beam entry by every word in the trie
//! that fits. Polynomial time in the phoneme stream length.

use std::collections::HashSet;

use phonetics::transcriptions::{Corpus, Pronunciation};

/// One candidate Mad Gab clue.
#[derive(Debug, Clone)]
pub struct Clue {
    /// The clue as a space-joined English phrase.
    pub phrase: String,
    /// The IPA stream the clue covers. Always equal to the target's
    /// IPA in [`SearchMode::Exact`] mode.
    pub ipa: String,
    /// Per-word components, in order.
    pub words: Vec<ClueWord>,
    /// Composite score in [0, 1] — higher is a better Mad Gab clue.
    pub score: f64,
}

/// One word inside a candidate clue.
#[derive(Debug, Clone)]
pub struct ClueWord {
    /// English headword.
    pub word: String,
    /// IPA span this word's transcription covers (which may differ
    /// from the target's span in Approximate mode).
    pub ipa: String,
    /// Frequency rank from the corpus, if known.
    pub rarity: Option<f64>,
    /// Accumulated substitution cost relative to the target's span
    /// of the IPA stream this word covers. Always 0.0 in Exact mode.
    pub sub_cost: f64,
}

/// Search behavior.
#[derive(Debug, Clone, Copy)]
pub enum SearchMode {
    /// Each clue word's IPA must exactly match its span of the
    /// target stream. The clue is a re-syllabification of the same
    /// phonemes; phonetic similarity is by construction 1.0.
    Exact,
    /// Each clue word's IPA is allowed to differ from its span of
    /// the target by up to `per_word_budget` of accumulated
    /// phonetic-distance cost; the whole clue's accumulated cost is
    /// capped at `total_budget`. Lets the generator find clues
    /// whose phonemes don't exactly match — /t/→/d/, /ɪ/→/i/, etc.
    Approximate {
        /// Maximum substitution cost per single trie-walked word.
        per_word_budget: f64,
        /// Maximum substitution cost summed across the whole clue.
        total_budget: f64,
    },
}

impl SearchMode {
    /// A sensible Approximate default — small per-word slack with a
    /// total cap that still keeps the clue recognizable.
    pub fn approximate() -> Self {
        Self::Approximate {
            per_word_budget: 0.5,
            total_budget: 1.5,
        }
    }
}

/// Search configuration. Defaults are tuned for an interactive
/// `madgab "..."` invocation.
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Number of clue candidates kept per beam position. Higher =
    /// better quality but quadratically more memory and time.
    pub beam_width: usize,
    /// How many candidates to return.
    pub top_n: usize,
    /// Maximum corpus rarity (= least common word allowed). Lower
    /// values mean a smaller, faster-to-build trie and clues that
    /// use more familiar vocabulary. None loads everything.
    pub max_rarity: Option<f64>,
    /// Search mode (only Exact is implemented for v0.1.0).
    pub mode: SearchMode,
    /// Reject clue words shorter than this many IPA characters.
    /// Without a floor, the beam fills with degenerate
    /// single-vowel paths like "a a a a a a a".
    pub min_word_ipa_chars: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            beam_width: 64,
            top_n: 10,
            max_rarity: Some(20_000.0),
            mode: SearchMode::Exact,
            min_word_ipa_chars: 2,
        }
    }
}

/// A reusable Mad Gab generator. Build once from a corpus; ask for
/// many phrases.
pub struct Generator {
    corpus: Corpus,
    config: GeneratorConfig,
}

impl Generator {
    /// Build a generator from raw corpus JSON. The JSON shape is the
    /// one `phonetics::transcriptions::Corpus` expects.
    pub fn from_json(json: &str, config: GeneratorConfig) -> Result<Self, phonetics::transcriptions::Error> {
        let corpus = Corpus::from_json(json, config.max_rarity)?;
        Ok(Self { corpus, config })
    }

    /// Access the underlying corpus (handy for transcription
    /// debugging and tests).
    pub fn corpus(&self) -> &Corpus {
        &self.corpus
    }

    /// Configuration in effect.
    pub fn config(&self) -> &GeneratorConfig {
        &self.config
    }

    /// Generate ranked clue candidates for `target`.
    ///
    /// Returns an empty Vec if the target's IPA stream has no
    /// complete coverings under the current corpus / config.
    pub fn generate(&self, target: &str) -> Vec<Clue> {
        let Some((target_ipa, target_boundaries)) = transcribe_with_boundaries(&self.corpus, target) else {
            return Vec::new();
        };
        let target_words: HashSet<String> = target
            .split_whitespace()
            .map(|w| w.to_lowercase().trim_end_matches(['.', ',', '!', '?']).to_string())
            .collect();
        let chars: Vec<char> = target_ipa.chars().collect();
        let n = chars.len();
        if n == 0 {
            return Vec::new();
        }

        // beam[p] = best K partial coverings of [0..p).
        let mut beam: Vec<Vec<Partial>> = vec![Vec::new(); n + 1];
        beam[0].push(Partial::empty());

        // Total-cost cap for Approximate mode; serves as a hard
        // prune on partials that have already overshot the budget.
        let total_budget = match self.config.mode {
            SearchMode::Exact => 0.0,
            SearchMode::Approximate { total_budget, .. } => total_budget,
        };

        for p in 0..n {
            if beam[p].is_empty() {
                continue;
            }
            // Take ownership of the beam-at-p so we can mutate beam[p..] freely.
            let here = std::mem::take(&mut beam[p]);
            for partial in &here {
                let remaining_budget = total_budget - partial.sub_cost_total;
                match self.config.mode {
                    SearchMode::Exact => {
                        for (consumed, pronunciation) in self.corpus.trie.words_starting_at(&chars, p) {
                            if consumed < self.config.min_word_ipa_chars {
                                continue;
                            }
                            let next = partial.extend(pronunciation, consumed, 0.0);
                            insert_top_k(&mut beam[p + consumed], next, self.config.beam_width);
                        }
                    }
                    SearchMode::Approximate { per_word_budget, .. } => {
                        let budget = per_word_budget.min(remaining_budget.max(0.0));
                        if budget <= 0.0 {
                            // Falling back to exact-only walk when the
                            // remaining budget is exhausted.
                            for (consumed, pronunciation) in self.corpus.trie.words_starting_at(&chars, p) {
                                if consumed < self.config.min_word_ipa_chars {
                                    continue;
                                }
                                let next = partial.extend(pronunciation, consumed, 0.0);
                                insert_top_k(&mut beam[p + consumed], next, self.config.beam_width);
                            }
                            continue;
                        }
                        let matches = self.corpus.trie.words_approximately_starting_at(
                            &chars,
                            p,
                            budget,
                            |target_c, trie_c| {
                                phonetics::distance(&target_c.to_string(), &trie_c.to_string())
                            },
                        );
                        for (consumed, pronunciation, word_cost) in matches {
                            if consumed < self.config.min_word_ipa_chars {
                                continue;
                            }
                            let next = partial.extend(pronunciation, consumed, word_cost);
                            insert_top_k(&mut beam[p + consumed], next, self.config.beam_width);
                        }
                    }
                }
            }
        }

        let mut completed = std::mem::take(&mut beam[n]);
        let mut clues: Vec<Clue> = completed
            .drain(..)
            .map(|p| p.into_clue(&target_ipa, &target_boundaries, &target_words))
            .collect();

        clues.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        clues.dedup_by(|a, b| a.phrase == b.phrase);
        clues.truncate(self.config.top_n);
        clues
    }
}

// -----------------------------------------------------------------
// Internals
// -----------------------------------------------------------------

/// Like `Corpus::transcribe` but also returns the running set of
/// char offsets at each word boundary, so we can later score how
/// much a candidate clue rearranges them.
fn transcribe_with_boundaries(corpus: &Corpus, phrase: &str) -> Option<(String, Vec<usize>)> {
    let mut out = String::new();
    let mut boundaries: Vec<usize> = Vec::new();
    for word in phrase.split_whitespace() {
        let key = word
            .to_lowercase()
            .trim_end_matches(['.', ',', '!', '?', ';', ':'])
            .to_string();
        let ipa = corpus.preferred_ipa(&key)?;
        out.push_str(ipa);
        boundaries.push(out.chars().count());
    }
    Some((out, boundaries))
}

/// A partially-constructed clue: the words chosen so far plus the
/// running cheap score used to prune the beam. Boundaries (the
/// per-word char-offset cuts) are derived from `words` at scoring
/// time.
#[derive(Debug, Clone)]
struct Partial {
    words: Vec<ClueWord>,
    /// Accumulated substitution cost across all words so far.
    /// Always zero in Exact mode.
    sub_cost_total: f64,
    /// Running cheap score (per-word rarity penalties plus word-length
    /// bonuses) used only to prune the beam. The final Clue score
    /// replaces this with the full novelty-aware computation.
    cheap_score: f64,
}

impl Partial {
    fn empty() -> Self {
        Self {
            words: Vec::new(),
            sub_cost_total: 0.0,
            cheap_score: 0.0,
        }
    }

    fn extend(&self, p: &Pronunciation, _consumed: usize, word_sub_cost: f64) -> Self {
        let len = p.ipa.chars().count();
        let word_bonus = (len as f64).min(6.0) / 6.0;
        let rarity_penalty = match p.rarity {
            Some(r) if r > 5_000.0 => -((r / 50_000.0).min(1.0)),
            _ => 0.0,
        };
        // Approximate-mode penalty: each unit of substitution cost
        // shaves cheap_score so the beam prefers closer matches.
        let approx_penalty = word_sub_cost;
        Self {
            words: {
                let mut w = self.words.clone();
                w.push(ClueWord {
                    word: p.word.clone(),
                    ipa: p.ipa.clone(),
                    rarity: p.rarity,
                    sub_cost: word_sub_cost,
                });
                w
            },
            sub_cost_total: self.sub_cost_total + word_sub_cost,
            cheap_score: self.cheap_score + word_bonus + rarity_penalty - approx_penalty,
        }
    }

    fn into_clue(self, target_ipa: &str, target_boundaries: &[usize], target_words: &HashSet<String>) -> Clue {
        // Reconstruct the clue's boundary set.
        let mut cum = 0_usize;
        let mut clue_boundaries: Vec<usize> = Vec::with_capacity(self.words.len());
        for w in &self.words {
            cum += w.ipa.chars().count();
            clue_boundaries.push(cum);
        }

        // Novelty: how few of the target's word boundaries the clue
        // also has. Boundary at the end of the phrase is shared by
        // construction, so exclude it.
        let target_inner: HashSet<usize> = target_boundaries
            .iter()
            .copied()
            .filter(|b| *b < cum)
            .collect();
        let clue_inner: HashSet<usize> = clue_boundaries
            .iter()
            .copied()
            .filter(|b| *b < cum)
            .collect();
        let shared = target_inner.intersection(&clue_inner).count() as f64;
        let denom = target_inner.len().max(1) as f64;
        let novelty = 1.0 - (shared / denom);

        // Word-novelty: penalty if the clue reuses any target word.
        let reused = self
            .words
            .iter()
            .filter(|w| target_words.contains(&w.word.to_lowercase()))
            .count() as f64;
        let word_novelty = 1.0 - (reused / self.words.len().max(1) as f64);

        // Word-length signal: prefer fewer/longer words, the
        // signature of a real Mad Gab clue.
        let avg_word_ipa_len = self
            .words
            .iter()
            .map(|w| w.ipa.chars().count())
            .sum::<usize>() as f64
            / self.words.len().max(1) as f64;
        let length_signal = (avg_word_ipa_len / 4.0).min(1.0);

        // Approximate-mode similarity: penalize total substitution
        // cost. In Exact mode sub_cost_total is 0, so similarity is
        // exactly 1.0 and this term is constant — the discrimination
        // remains on the novelty/length axes as before.
        let similarity = (1.0 - self.sub_cost_total / 4.0).clamp(0.0, 1.0);

        let score =
              0.40 * similarity
            + 0.35 * novelty
            + 0.15 * word_novelty
            + 0.10 * length_signal;

        Clue {
            phrase: self.words.iter().map(|w| w.word.as_str()).collect::<Vec<_>>().join(" "),
            ipa: target_ipa.to_string(),
            words: self.words,
            score,
        }
    }
}

/// Insert `candidate` into a top-K beam, keeping the K highest-cheap-
/// score entries. Stable enough for our purposes.
fn insert_top_k(beam: &mut Vec<Partial>, candidate: Partial, k: usize) {
    if beam.len() < k {
        beam.push(candidate);
        return;
    }
    // Find the weakest entry; replace if the candidate is stronger.
    let (worst_idx, worst_score) = beam
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.cheap_score.partial_cmp(&b.cheap_score).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, p)| (i, p.cheap_score))
        .unwrap();
    if candidate.cheap_score > worst_score {
        beam[worst_idx] = candidate;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY: &str = r#"{
        "cat":  { "rarity": 100, "ipa": { "cmu": "kæt" }, "alt_display": "CAT" },
        "kit":  { "rarity": 200, "ipa": { "cmu": "kɪt" } },
        "at":   { "rarity": 80,  "ipa": { "cmu": "æt" } },
        "ka":   { "rarity": 5000,"ipa": { "cmu": "kæ" } }
    }"#;

    #[test]
    fn transcribes_known_phrase() {
        let g = Generator::from_json(TINY, GeneratorConfig::default()).unwrap();
        assert_eq!(g.corpus().transcribe("cat"), Some("kæt".to_string()));
    }

    #[test]
    fn generates_completions_for_a_tiny_corpus() {
        let cfg = GeneratorConfig {
            min_word_ipa_chars: 2,
            ..GeneratorConfig::default()
        };
        let g = Generator::from_json(TINY, cfg).unwrap();
        // "cat" IPA = "kæt". Completions whose word IPAs concatenate
        // to "kæt": just the single word "cat" itself (in our tiny
        // corpus). "ka" + "t" doesn't work — "t" alone isn't a word.
        let clues = g.generate("cat");
        assert!(!clues.is_empty(), "expected at least one covering");
        assert!(clues.iter().any(|c| c.phrase == "cat"));
    }

    #[test]
    fn empty_when_target_word_unknown() {
        let g = Generator::from_json(TINY, GeneratorConfig::default()).unwrap();
        let clues = g.generate("orange");
        assert!(clues.is_empty());
    }
}
