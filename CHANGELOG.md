# Changelog

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-10

### Added

- Core `FinVader` analyzer: finance-aware VADER with blended compound scoring
- Financial lexicon: 181 word valences calibrated for market news
- Multi-word phrase matching with magnitude amplification ("beat by 23%")
- Gap-phrase matching for verb+object pairs ("beats ... expectations")
- Neutral-override masking (gross, cancer, debt, crude, vice, share, etc.)
- Negation handling for financial terms ("failed to beat expectations")
- Booster/dampener support (significantly, sharply, slightly, modestly, etc.)
- Catalyst detection: FDA, M&A, index inclusion, contracts, distress events
- Co-occurrence catalyst patterns (wins + contract, awarded + contract)
- Discrete `Signal` enum: StronglyBullish, Bullish, Neutral, Bearish, StronglyBearish
- 60-headline evaluation benchmark (100% accuracy vs generic VADER's 51.7%)
- `examples/demo.rs` — side-by-side comparison with generic VADER
- `examples/bench.rs` — throughput benchmark (~45k headlines/sec)
- Pipeline and system architecture diagrams (SVG in docs/)
