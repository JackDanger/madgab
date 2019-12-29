require 'phonetics/levenshtein'
module Madgab
  class SearchEntry
    attr_accessor :match,
                  :target,
                  :subtrie,
                  :previous_entry

    def initialize(match, target, subtrie, previous_entry = nil)
      @match = match
      @target = target
      @subtrie = subtrie
      @previous_entry = previous_entry
    end

    # Rank search entries in the fibonacci heap of the priority queue by
    # Levenstein distance
    def <=>(other)
      score <=> other.score
    end

    def to_s
      "<SearchEntry:#{object_id} #{full_ipa_phrase(' ').inspect} " \
        " (#{full_english_phrase.inspect}) score: #{score} " \
        " lev: #{phonetic_levenshtein_distance}, popularity: #{popularity_boost}, chain size penalty: #{entry_chain_size_penalty}" \
        "#{subtrie && ", path: #{subtrie[:path]}"}>"
    end
    alias inspect to_s

    # lower score is better
    def score
      @score ||= phonetic_levenshtein_distance + penalties
    end

    def hash
      [
        full_ipa_phrase(' '),
        full_english_phrase(' '),
        match,
      ].join(',').hash
    end

    # The concatenation of all search entries thus far
    def full_ipa_phrase(delimiter = '')
      @full_ipa_phrase ||= {}
      @full_ipa_phrase[delimiter] ||= entry_chain.map(&:match).compact.join(delimiter)
    end

    def full_english_phrase(delimiter = ' ')
      @full_english_phrase ||= {}
      @full_english_phrase[delimiter] ||= words.join(delimiter)
    end

    def penalties
      entry_chain.map(&:penalty).reduce(&:+)
    end

    def penalty
      entry_chain_size_penalty + stutter_penalty
    end

    def entry_chain_size_penalty
      entry_chain.size * 0.5
    end

    def stutter_penalty
      return 0.0 unless words.size > 1

      if words.first == words[1]
        2.0
      end

      0.0
    end

    def phonetic_levenshtein_distance
      Phonetics::Levenshtein.distance(full_ipa_phrase, target)
    end

    def levenshtein_distance
      DamerauLevenshtein.distance(full_ipa_phrase, target)
    end

    def hamming_distance_of_shared_substring
      s1 = target
      s2 = full_ipa_phrase
      s1, s2 = s2, s1 if s1.size < s2.size
      s1.bytes[0..(s2.bytes.size - 1)].zip(s2.bytes).reject { |a, b| a == b }.size
    end

    # Give a boost for using more popular words
    # Assumes the max rarity is 60_000
    def popularity_boost
      return 0 if word_datas.empty?

      rare_words = word_datas.select { |data| data[:rarity] }
      return 0 if rare_words.empty?

      # 0-1 score for popularity
      # Then divide it by 0-1 for word length
      boosts = rare_words.map do |word|
        1 - Math.log(word[:rarity] + 1, 60_000)
      end
      boosts.reduce(0, &:+)
    end

    def short_word_penalty
      words.map do |word|
        if word.size < 2
          0.3
        elsif word.size < 3
          0.15
        else
          0
        end
      end.reduce(0, &:+)
    end

    def word
      words.first
    end

    def words
      word_datas.map {|word_data| word_data[:word] }
    end

    def word_datas
      entry_chain.flat_map(&:word_data).compact
    end

    def word_data
      subtrie[:terminal]
    end

    def entry_chain
      if previous_entry
        previous_entry.entry_chain + [self]
      else
        [self]
      end
    end
  end
end
