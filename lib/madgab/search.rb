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
                :max_rarity,
                :results

    def initialize(target, timeout_seconds: 5, exclude: [], min_results: 30, max_rarity: 10_000)
      @target = target # the search term

      @max_rarity = max_rarity
      @timeout_seconds = timeout_seconds
      @exclude = exclude
      @min_results = min_results

      @frontier = FastContainers::PriorityQueue.new(:min)
      @visited = FastContainers::PriorityQueue.new(:min)
    end

    def result
      SearchResult.new(results: results, time: Time.now - @timeout_start)
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

    # Given a portion of the trie, the remainder of a charstring, and a count of
    # how many times we've either skipped over an input character or skipped over
    # a character in the trie,
    # Recursively find terminals that are rough approximations of the input
    # phrase, keeping the deviation count as part of the return value
    def perform
      @timeout_start = Time.now
      puts "Searching for #{@target}"
      prime = SearchEntry.new(nil, @target, whole_trie, nil)
      debug "priming: #{prime}"
      debug ''
      ticked = 0
      frontier.push prime, prime.score

      @results = Set.new

      begin
        search_entry = frontier.pop
        @visited.push search_entry, search_entry.score

        status = "#{@visited.size}/#{@frontier.size} - #{search_entry}"
        debug status

        seconds_since_start = (Time.now - @timeout_start).to_i
        if seconds_since_start > ticked
          puts status
          ticked += 1
        end


        # If we've reached the end of a word, continue with a pointer to the
        # top of the whole trie and enqueue this in the frontier
        if search_entry.subtrie[:terminal]
          new_entry = SearchEntry.new(
            '',
            search_entry.target,
            whole_trie,
            search_entry,
          )
          debug "+ found terminal entry: #{new_entry}"
          frontier.push(new_entry, new_entry.score)
        end

        search_entry.subtrie.each do |key, subtrie|
          next if key == :path
          next if key == :depth
          next if key == :terminal

          new_entry = SearchEntry.new(
            "#{search_entry.match}#{key}",
            @target,
            subtrie,
            search_entry.previous_entry,
          )
          debug "- iterating: #{search_entry.match.inspect}+#{key.inspect} #{new_entry}"
          frontier.push(new_entry, new_entry.score)
        end
      end until frontier.empty? || timeout? #&& collected?)

      require 'pry'
      binding.pry
      nil
    end
    private

    def collected?
      results.size >= min_results
    end

    def timeout?
      (Time.now - @timeout_start > timeout_seconds)
    end
  end
end
