require 'phonetics'
require_relative 'madgab/search'

module Madgab
  def self.search_phonetic_permutations(input, options = {})
    transcribed = Phonetics.transcription_for(input)
    search = Madgab::SearchPhoneticPermutations.new(transcribed, **options)
    p search.perform
  end

  def self.search_a_star(input, options = {})
    transcribed = Phonetics.transcription_for(input)
    search = Madgab::SearchAStar.new(transcribed, **options)
    search.perform
    p search.result
  end
end
