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
        " (#{full_english_phrase.inspect}) score: #{'%0.3f' % score} " \
        " lev: #{'%0.3f' % phonetic_levenshtein_distance}, popularity: #{'%0.3f' % popularity_boost}, chain size penalty: #{entry_chain_size_penalty}" \
        "#{subtrie && ", path: #{subtrie[:path]}"}>"
    end
    alias inspect to_s

    # lower score is better
    def score
      @score ||= phonetic_levenshtein_distance + penalties
    end

    def hash
      @hash ||= [
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

    def padded_full_ipa_phrase
      full_ipa_phrase + ('ə' * [0, target.size - full_ipa_phrase.size].max)
    end

    def full_english_phrase(delimiter = ' ')
      @full_english_phrase ||= {}
      @full_english_phrase[delimiter] ||= words.join(delimiter)
    end

    def phonetic_levenshtein_distance
      @phonetic_levenshtein_distance ||=  Phonetics::Levenshtein.distance(padded_full_ipa_phrase, target)
    end

    def penalties
      entry_chain.map(&:penalty).reduce(&:+)
    end

    def penalty
      entry_chain_size_penalty + stutter_penalty
    end

    def entry_chain_size_penalty
      entry_chain.size * 0.01
    end

    def stutter_penalty
      return 0.0 unless words.size > 1

      if words.first == words[1]
        2.0
      end

      0.0
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
      return @word if defined?(@word)
      @word ||= word_data && word_data.min_by { |w| w[:rarity] }[:word]
    end

    def words
      entry_chain.map(&:word)
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
