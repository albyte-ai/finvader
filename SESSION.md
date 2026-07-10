# finvader — session handoff

Context for another Claude Code session picking this up on any machine. Private repo.

## The bigger goal

Owner (albyte-ai / Albin) wants a **news-driven momentum-stock alert system**. He
works an MNC day job and can't watch tickers all day. He wants: scrape financial
news (Robinhood-linked sources — Benzinga, TipRanks, Nasdaq, Motley Fool, newsBTC,
24/7 Wall St, Simply Wall St), score sentiment, and **push a notification** the
moment strong-bullish news + a catalyst hits, so he never misses a mover. Weekly
scan for slower setups too. Wants a free / self-built alternative to Unusual Whales
(too costly). Rust chosen for throughput + parallel multi-source fetches.

**finvader is the first building block** — the sentiment/catalyst scoring core.

## What finvader is

`~/Projects/finvader` — Rust crate (edition 2024). Finance-aware sentiment: a VADER
port (on top of `vader-sentimental`) extended with a financial lexicon, phrase +
gap-phrase rules, neutral-override masking, negation/booster handling, and catalyst
detection (FDA, M&A, index inclusion, contracts, bankruptcy, fraud).

### Files
- `src/lib.rs` — core `FinVader::analyze`, blend, signals, all passes + unit tests
- `src/lexicon.rs` — 181 word valences, NEUTRAL_OVERRIDES, NEGATIONS, BOOSTERS
- `src/phrases.rs` — PHRASES + GAP_PHRASES
- `src/catalyst.rs` — episodic-pivot event detection
- `tests/eval.rs` — 60-headline accuracy benchmark (integration)
- `examples/{demo,bench}.rs` — side-by-side demo + throughput bench
- `data/headlines.tsv` — 60 labeled headlines
- `docs/*.mmd` + `docs/*.svg` — mermaid sources + pre-rendered SVG (see gotcha)
- `README.md`, `LICENSE` (MIT)

## Current state (as of 2026-07)

- **Accuracy: 60/60 (100%)** vs generic VADER 31/60 (51.7%) on the labeled set.
- **Bench:** finvader 22.3 µs/headline (~45k/sec) vs generic VADER 1.8 µs (~570k/sec),
  Apple Silicon, `cargo run --release --example bench`. Plenty fast for the use case.
- **Coverage:** was 98.9% line / 100% func. The only "misses" were closing braces
  after `continue`/nested-`if` — llvm-cov region artifacts, not untested paths.
  Refactored `word_pass` (continue → if/else) and the phrase/gap "by X%" amplifiers
  (nested if → edition-2024 let-chains) to push toward literal 100%. **NOTE: these
  refactors were committed but NOT re-verified** (owner said skip the test run before
  commit). First thing next session: `cargo test` + `cargo llvm-cov --summary-only`
  to confirm still green + coverage.
- **Not published to crates.io.** Repo is private; flip public at publish time.

## Pending / next steps

1. **Verify the uncommitted-intent refactors** — run `cargo test` and coverage.
2. Optional: true Python `vaderSentiment` head-to-head (bench currently compares
   finvader vs the Rust VADER crate only).
3. crates.io publish (needs public repo + `cargo publish` dry-run).
4. Move up the stack: the scraper + alert pipeline (see `docs/system.svg`).

## Gotchas / decisions

- **crates.io does NOT render mermaid** (open feature request, not shipped 2026).
  So diagrams are **pre-rendered to SVG** (`docs/*.svg`, via `mmdr` =
  `cargo install mermaid-rs-renderer`) and referenced by **relative path** in the
  README — crates.io rewrites relative image paths to the repo, so they render on
  BOTH crates.io and GitHub. Keep `.mmd` sources in sync when editing diagrams.
- **Owner preference: narrate before acting.** State what you're about to do before
  running tools (esp. builds/commits/side-effects), then act. He interrupts silent
  execution.
- Local Claude memory files exist at
  `~/.claude/projects/-Users-albyte/memory/` (finvader-project, narrate-before-acting)
  — those DON'T travel across machines; this file does.
