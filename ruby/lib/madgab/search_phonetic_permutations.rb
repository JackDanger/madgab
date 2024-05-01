module Madgab
  class SearchPhoneticPermutations

    attr_reader :target,
                :timeout_seconds,
                :results,
                :max_rarity

    def initialize(target, timeout_seconds: 5, max_rarity: 10_000)
      @target = target # the search term

      @max_rarity = max_rarity
      @timeout_seconds = timeout_seconds
    end

    def whole_trie
      @whole_trie ||= Phonetics::Transcriptions.trie(max_rarity)
    end

    def debug_log
      @debug_log ||= File.open('./madgab.log', 'a')
    end

    def debug(str)
      debug_log.write "#{str}\n"
      debug_log.flush
    end

    # Transcribe the input to IPA
    # using the set of IPA fragments that map to words, perform LTR word-finding algo to generate list of fragments
    #   Each fragment should be a reference to the global set of words and their IPA variations
    # Iterate over each word list
    #   then over each word list comprising the variation
    #   then over each variation's 
    def perform
      @timeout_start = Time.now
      puts "Searching for #{@target}"

      ss = phonetic_words_in(target)
      require 'pry'
      binding.pry
      nil
    end

    private

    # Given a string of IPA symbols
    # Iterate through each symbol, checking if the subset from index 0 to the
    #   current index exists as an IPA transcription of something.
    # If found, recursively find the rest of the words
    # Continue to find other, longer words beginning at index 0
    # Compact at end to not return paths that yielded no coherent sets of words
    def phonetic_words_in(phrase)
      sentences = find_rest(phrase)
      require 'pry'
      binding.pry
      sentences
    end

    def find_rest(phrase, depth='')
      chars = phrase.chars
      possibilities = []
      chars.size.times do |n|
        candidate = phrase[0...(n + 1)]
        puts "#{depth}#{n}/#{chars.size} – #{candidate}/#{phrase[(n+1)..]}"
        if is_word?(candidate)
          puts "#{depth}is word: #{candidate} => #{is_word?(candidate)}"

          if chars.size - 1 == n
            possibilities << [candidate]
          else

            rest = find_rest(phrase[(n+1)..], depth + '  ')

            rest.each do |path|
              possibilities << [candidate] + path
            end unless rest == false
          end
          puts "#{depth} #{possibilities.inspect}"
        end
      end

      # base cases are either:
      #   The phrase ended in at least one set of valid words
      return possibilities if possibilities.any?
      #   Or not
      return false
    end

    def is_word?(chars)
      reverse_transcriptions[chars]
    end

    # A map of all words to a list of their possible transcriptions
    def transcriptions
      @transcriptions ||= Phonetics::Transcriptions.transcriptions.inject({}) do |acc, (word, data)|
        acc.update word => data['ipa'].values
      end
    end

    # A map of all transcriptions to a list of their possible words
    def reverse_transcriptions
      @reverse_transcriptions ||= Phonetics::Transcriptions.transcriptions.inject({}) do |acc, (word, data)|
        data['ipa'].values.each do |ipa|
          acc[ipa] ||= []
          acc[ipa] << word
        end
        acc
      end
    end

    def timeout?
      (Time.now - @timeout_start > timeout_seconds)
    end
  end
end
