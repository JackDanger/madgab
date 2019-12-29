require_relative 'search_result'
require_relative 'search_entry'
require 'fc'

module Madgab
  class Search

    attr_reader :input,
                :timeout_seconds,
                :exclude,
                :frontier,
                :visited,
                :min_results,
                :results

    def initialize(target, timeout_seconds: 5, exclude: [], min_results: 30)
      @target = target # the search term

      @timeout_seconds = timeout_seconds
      @exclude = exclude
      @min_results = min_results

      @frontier = FastContainers::PriorityQueue.new(:min)
    end

    def result
      SearchResult.new(results: results, time: Time.now - @timeout_start)
    end

    # Given a portion of the trie, the remainder of a charstring, and a count of
    # how many times we've either skipped over an input character or skipped over
    # a character in the trie,
    # Recursively find terminals that are rough approximations of the input
    # phrase, keeping the deviation count as part of the return value
    def perform
      @timeout_start = Time.now
      puts "Searching for #{@target}"
      prime = SearchEntry.new(nil, @target, Phonetics::Transcriptions.trie, nil)
      log "priming: #{prime}"
      ticked = 0
      frontier.push prime, prime.score

      @visited = Set.new
      @results = Set.new

      begin
        search_entry = frontier.pop
        @visited << search_entry

        seconds_since_start = (Time.now - @timeout_start).to_i
        if seconds_since_start > ticked
          puts("#{@visited.size}/#{@frontier.size} - #{results.size} results, current entry:\n\t#{search_entry}")
          ticked += 1
        end

        log ''
        log search_entry.to_s
        log "frontier size: #{@frontier.size}"

        search_entry.subtrie.each do |key, subtrie|
          next if key == :path
          next if key == :depth

          # We've reached the end of a word, continue with a pointer to the
          # top of the trie
          if key == :terminal
            new_entry = SearchEntry.new(
              search_entry.match,
              search_entry.target,
              Phonetics::Transcriptions.trie,
              search_entry,
            )
          else
            new_entry = SearchEntry.new(
              "#{search_entry.match}#{key}",
              @target,
              subtrie,
              search_entry.previous_entry,
            )
          end

          log "enqueueing: #{new_entry}"
          frontier.push(new_entry, new_entry.score)
        end
      end until frontier.empty? || timeout? #&& collected?)

      require 'pry'
      binding.pry
      nil
    end
    private

    def log(*args)
      puts(*args) if ENV['VERBOSE']
    end

    def collected?
      results.size >= min_results
    end

    def timeout?
      (Time.now - @timeout_start > timeout_seconds)
    end
  end
end
