require 'phonetics'
require_relative 'madgab/search'

module Madgab
  def self.search(input, options = {})
    transcribed = Phonetics.transcription_for(input)
    search = Madgab::Search.new(transcribed, **options)
    search.perform
    p search.result
  end
end
