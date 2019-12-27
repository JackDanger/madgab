require_relative 'search_result'
require_relative 'search_entry'
require 'fc'

module IPA
  class Search

    attr_reader :input,
                :timeout_seconds,
                :exclude,
                :frontier,
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
      prime = SearchEntry.new(nil, @target, nil, {}, nil)
      log "priming: #{prime}"
      ticked = 0
      frontier.push prime, prime.score

      @visited = 0
      @results = Set.new

      begin
        search_entry = frontier.pop
        @visited += 1

        seconds_since_start = (Time.now - @timeout_start).to_i
        if seconds_since_start > ticked
          puts("#{@visited}/#{@frontier.size} - #{results.size} results, current entry:\n\t#{search_entry}")
          ticked += 1
        end

        log ''
        log search_entry.to_s
        log "frontier size: #{@frontier.size}"

        # require 'pry'
        # binding.pry
        # require 'benchmark'
        # entries = IPA.transcriptions.flat_map do |word, data|
        #   next unless data['rarity'] && data['rarity'] < 30_000
        #   data['ipa'].map do |source, ipa|
        #     new_entry = SearchEntry.new(
        #       ipa,
        #       @target,
        #       word,
        #       data,
        #       search_entry
        #     )
        #   end
        # end.compact
        # Benchmark.measure { entries.take(10_000).map(&:score) }

        IPA.transcriptions.each do |word, data|
          next unless data['rarity'] && data['rarity'] < 30_000
          data['ipa'].each do |source, ipa|
            new_entry = SearchEntry.new(
              ipa,
              @target,
              word,
              data,
              search_entry
            )
            log "[terminal] enqueueing: #{new_entry}"
            frontier.push(new_entry, new_entry.score)
          end
        end
      end until frontier.empty? #|| (timeout? && collected?)

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
