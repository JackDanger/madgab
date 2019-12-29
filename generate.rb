require_relative 'lib/madgab'

input = ARGV.first
if input.to_s.empty?
  warn %Q|USAGE: #{$0} "let's play a game"|
else
  results = Madgab.search(
    input || "let's play Mad Gab for a while",
    timeout_seconds: 15,
    max_rarity: 10_000,
  )
  p results
end
