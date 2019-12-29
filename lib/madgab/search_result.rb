module Madgab
  class SearchResult
    attr_reader :results, :time
    def initialize(results:, time:)
      @results = results.sort_by(&:score)
      @time = time
    end

    def to_s
      (["#{results.size} in #{time}s"] + results).join("\n")
    end

    alias inspect to_s
  end
end
