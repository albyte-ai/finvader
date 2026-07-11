<div align="center">

# 📈 finvader

### Finance-aware sentiment analysis for Rust

**VADER, re-tuned for the market** — financial lexicon, phrase rules, and
catalyst detection for news headlines and market text instead of tweets.

[![crates.io](https://img.shields.io/crates/v/finvader.svg?style=for-the-badge&color=fc8d62&logo=rust)](https://crates.io/crates/finvader)
[![docs.rs](https://img.shields.io/docsrs/finvader?style=for-the-badge&color=66c2a5&logo=docsdotrs)](https://docs.rs/finvader)
[![CI](https://img.shields.io/github/actions/workflow/status/albyte-ai/finvader/ci.yml?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/albyte-ai/finvader/actions)
[![license](https://img.shields.io/crates/l/finvader.svg?style=for-the-badge&color=8da0cb)](https://github.com/albyte-ai/finvader/blob/main/LICENSE)

[![accuracy](https://img.shields.io/badge/accuracy-100%25_(60%2F60)-brightgreen?style=flat-square)](#-accuracy)
[![generic VADER](https://img.shields.io/badge/generic_VADER-51.7%25_(31%2F60)-red?style=flat-square)](#-accuracy)
[![throughput](https://img.shields.io/badge/throughput-~45k_headlines%2Fsec-blue?style=flat-square)](#-performance)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange?style=flat-square)](https://github.com/albyte-ai/finvader/blob/main/Cargo.toml)
[![deps](https://img.shields.io/badge/dependencies-1-blueviolet?style=flat-square)](https://github.com/albyte-ai/finvader/blob/main/Cargo.toml)

</div>

---

## 🎯 Why finvader?

Generic [VADER](https://github.com/cjhutto/vaderSentiment) was calibrated for
social media. On financial text it misfires in **two directions**:

| ❌ Generic VADER problem | 💥 Example | finvader fix |
| --- | --- | --- |
| **Misses finance sentiment** | *"beats expectations"*, *"cuts guidance"*, *"going concern"* → score ≈ 0 | 📖 Financial lexicon + phrase rules |
| **Misfires on neutral finance words** | *"**gross** margin"*, *"**cancer** drug"*, *"**debt** refinancing"* → scored negative | 🎭 Neutral-override masking |

And it adds something VADER never had: **🚨 catalyst detection** — single
events (FDA approval, buyout offer, index inclusion, bankruptcy) that
permanently re-rate a stock and deserve to be surfaced on their own.

## 🏆 Accuracy

On a hand-labeled set of 60 financial headlines:

| Analyzer | Correct signal | Accuracy | |
| --- | --- | --- | --- |
| generic VADER | 31 / 60 | 51.7% | 🟥🟥🟥🟥🟥⬜⬜⬜⬜⬜ |
| **finvader** | **60 / 60** | **100%** | 🟩🟩🟩🟩🟩🟩🟩🟩🟩🟩 |

## 🚀 Quick start

```toml
[dependencies]
finvader = "0.1"
```

```rust
use finvader::{FinVader, Signal};

let fv = FinVader::new();
let s = fv.analyze("Acme beats Q3 expectations and raises full-year guidance");

assert!(s.compound > 0.5);
assert_eq!(s.signal, Signal::StronglyBullish);

// Which terms drove the score:
for t in &s.triggers {
    println!("{:<20} {:+.3}", t.term, t.valence);
}
```

> 💡 Construct one `FinVader` and reuse it across calls — it loads the
> lexicons once and is `Send + Sync`.

## ⚙️ How it works

Each input runs two parallel passes — a masked base-VADER pass and a
financial layer (phrases, gap-phrases, single words) — then the two are
blended, nudged by any catalyst, and clamped:

<p align="center">
  <img src="https://raw.githubusercontent.com/albyte-ai/finvader/main/docs/pipeline.svg" alt="finvader analysis pipeline" width="620">
</p>

| Stage | What it does |
| --- | --- |
| 🔡 **normalize** | Lowercase, strip punctuation, keep market-text characters (`-`, `%`, `$`, `'`) |
| 🎭 **mask_for_base** | Replace finance-neutral words (`gross`, `cancer`, `debt`, `crude`, `vice`, `share`…) with a placeholder *before* base VADER, so everyday valences never pollute the score |
| 🧩 **phrase / gap / word passes** | Match multi-word phrases (`beats expectations`), gap-tolerant pairs (`beats … expectations`, up to 2 tokens apart), and single words — consumed so nothing double-counts. `up 45%` / `down 30%` become magnitude-scaled moves |
| 🔁 **negation & boosters** | `failed to beat expectations` **flips** · `sharply missed` **amplifies** · `slightly missed` **softens** · `beat by 40%` **scales on magnitude** |
| ⚖️ **blend** | With financial terms present: `0.35 × base + 0.65 × financial`; otherwise base VADER passes through untouched |
| 🚨 **catalyst bonus** | A detected event shifts the compound by ±0.25 |

### 📊 Signals

`compound` maps to a discrete `Signal`:

| compound | Signal | |
| --- | --- | --- |
| `>= 0.5` | `StronglyBullish` | 🟢🟢 |
| `>= 0.15` | `Bullish` | 🟢 |
| `-0.15 .. 0.15` | `Neutral` | ⚪ |
| `<= -0.15` | `Bearish` | 🔴 |
| `<= -0.5` | `StronglyBearish` | 🔴🔴 |

### 🚨 Catalyst detection

Beyond the smooth sentiment score, finvader flags **episodic-pivot events**
and returns them as `Option<Catalyst>`:

| 📈 Bullish catalysts | 📉 Bearish catalysts |
| --- | --- |
| FDA approval / clearance | FDA rejection |
| Breakthrough therapy | Missed / failed endpoint |
| Met primary endpoint | Chapter 11 / Chapter 7 |
| Buyout / takeover / merger | Going concern |
| S&P 500 inclusion | SEC charges |
| Contract awards | Accounting fraud |
| Record quarter, beat-and-raise | Auditor resignation |

## 🔌 Where it fits

finvader is the scoring core of a news-driven momentum alert pipeline:

<p align="center">
  <img src="https://raw.githubusercontent.com/albyte-ai/finvader/main/docs/system.svg" alt="finvader in an alert pipeline" width="820">
</p>

## ⚡ Performance

Single-threaded, release build, `cargo run --release --example bench`
(Apple Silicon):

| Analyzer | Per headline | Throughput |
| --- | --- | --- |
| generic VADER | 1.8 µs | ~570,000 / sec |
| **finvader** | 22.3 µs | **~45,000 / sec** |

finvader does more work per call (masking + three match passes + catalyst
detection) and still clears tens of thousands of headlines per second per
core.

## 📖 Lexicon

Single-word and phrase valences are calibrated for market news, informed by
the [Loughran-McDonald](https://sraf.nd.edu/loughranmcdonald-master-dictionary/)
financial sentiment research. Valences are on VADER's `-4.0 ..= 4.0` scale.

## 🧪 Examples

```sh
cargo run --example demo              # side-by-side finvader vs generic VADER
cargo run --release --example bench   # throughput benchmark
```

## 📄 License

MIT © [albyte-ai](https://github.com/albyte-ai)
