//! # finvader
//!
//! Finance-aware sentiment analysis for Rust, built on top of the VADER
//! algorithm ([`vader-sentimental`](https://crates.io/crates/vader-sentimental)).
//!
//! Generic VADER was tuned for social media and misreads financial text in
//! two directions: it misses finance-specific sentiment ("beats
//! expectations", "cuts guidance" score near zero), and it misfires on
//! finance-neutral words ("gross margin", "cancer drug", "debt refinancing"
//! score negative). `finvader` fixes both:
//!
//! - a **financial lexicon** of single words and multi-word phrases with
//!   valences calibrated for market news,
//! - **neutral overrides** that mask finance-neutral words before the base
//!   VADER pass,
//! - **negation and booster handling** for financial terms ("failed to beat
//!   expectations" flips), and magnitude amplification ("beat by 23%"),
//! - **catalyst detection** for episodic-pivot events (FDA approvals,
//!   contract awards, index inclusions, bankruptcy filings).
//!
//! ```
//! use finvader::{FinVader, Signal};
//!
//! let fv = FinVader::new();
//! let s = fv.analyze("Acme beats Q3 expectations and raises full-year guidance");
//! assert!(s.compound > 0.5);
//! assert_eq!(s.signal, Signal::StronglyBullish);
//! ```
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod catalyst;
mod lexicon;
mod phrases;

use std::collections::HashMap;

use vader_sentimental::SentimentIntensityAnalyzer;

pub use catalyst::Catalyst;

/// VADER's normalization constant.
const ALPHA: f64 = 15.0;
/// Damping applied when a term's valence is flipped by negation (VADER's rule).
const NEGATION_SCALAR: f64 = 0.74;
/// Intensity added/removed by boosters and dampeners (VADER's rule).
const BOOST: f64 = 0.293;
/// Weight of the financial layer vs the base VADER score when financial
/// terms are present.
const FIN_WEIGHT: f64 = 0.65;
/// Compound bonus applied when a catalytic event is detected.
const CATALYST_BONUS: f64 = 0.25;

/// Discrete trading signal derived from the compound score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// compound >= 0.5
    StronglyBullish,
    /// compound >= 0.15
    Bullish,
    /// -0.15 < compound < 0.15
    Neutral,
    /// compound <= -0.15
    Bearish,
    /// compound <= -0.5
    StronglyBearish,
}

/// A lexicon term that contributed to the financial score.
#[derive(Debug, Clone)]
pub struct Trigger {
    /// The matched word or phrase.
    pub term: String,
    /// The valence applied, after negation/booster/magnitude adjustments.
    pub valence: f64,
}

/// Result of analyzing one piece of text.
#[derive(Debug, Clone)]
pub struct FinancialSentiment {
    /// Combined score in [-1.0, 1.0].
    pub compound: f64,
    /// Base VADER positive proportion.
    pub positive: f64,
    /// Base VADER negative proportion.
    pub negative: f64,
    /// Base VADER neutral proportion.
    pub neutral: f64,
    /// Discrete signal derived from `compound`.
    pub signal: Signal,
    /// Episodic-pivot event, if one was detected.
    pub catalyst: Option<Catalyst>,
    /// Financial lexicon terms that drove the score (empty if the score
    /// came purely from base VADER).
    pub triggers: Vec<Trigger>,
}

/// Finance-aware sentiment analyzer. Construction loads the lexicons once;
/// reuse one instance across calls (it is `Send + Sync`).
pub struct FinVader {
    base: SentimentIntensityAnalyzer<'static>,
    words: HashMap<&'static str, f64>,
}

impl Default for FinVader {
    fn default() -> Self {
        Self::new()
    }
}

impl FinVader {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self {
            base: SentimentIntensityAnalyzer::new(),
            words: lexicon::WORDS.iter().copied().collect(),
        }
    }

    /// Analyze a headline, sentence, or short document.
    pub fn analyze(&self, text: &str) -> FinancialSentiment {
        let masked = mask_for_base(text);
        let base = self.base.polarity_scores(&masked);

        let mut norm = normalize(text);
        let catalyst = catalyst::detect(&norm);

        let mut triggers = Vec::new();
        let mut fin_sum = phrase_pass(&mut norm, &mut triggers);
        let tokens: Vec<&str> = norm.split_whitespace().collect();
        let mut consumed = vec![false; tokens.len()];
        fin_sum += gap_pass(&tokens, &mut consumed, &mut triggers);
        fin_sum += self.word_pass(&tokens, &consumed, &mut triggers);

        let mut compound = if triggers.is_empty() {
            base.compound
        } else {
            let fin_compound = fin_sum / (fin_sum * fin_sum + ALPHA).sqrt();
            (1.0 - FIN_WEIGHT) * base.compound + FIN_WEIGHT * fin_compound
        };
        if let Some(c) = &catalyst {
            compound += if c.bullish { CATALYST_BONUS } else { -CATALYST_BONUS };
        }
        let compound = compound.clamp(-1.0, 1.0);

        FinancialSentiment {
            compound,
            positive: base.pos,
            negative: base.neg,
            neutral: base.neu,
            signal: signal_for(compound),
            catalyst,
            triggers,
        }
    }

    /// Single-word pass over the token stream (phrase and gap-phrase
    /// matches already consumed).
    fn word_pass(&self, tokens: &[&str], consumed: &[bool], triggers: &mut Vec<Trigger>) -> f64 {
        let mut sum = 0.0;

        let mut i = 0;
        while i < tokens.len() {
            if consumed[i] {
                i += 1;
                continue;
            }
            let tok = tokens[i];

            // "up 45%" / "down 30%" — magnitude-scaled directional move.
            if (tok == "up" || tok == "down")
                && let Some(pct) = tokens.get(i + 1).and_then(|t| parse_pct(t))
            {
                let mag = (pct / 25.0).min(3.0);
                let v = if tok == "up" { mag } else { -mag };
                sum += v;
                triggers.push(Trigger {
                    term: format!("{tok} {pct}%"),
                    valence: v,
                });
                i += 2;
            } else {
                if let Some(&valence) = self.words.get(tok) {
                    let v = adjust_for_context(valence, &tokens[..i]);
                    sum += v;
                    triggers.push(Trigger {
                        term: tok.to_string(),
                        valence: v,
                    });
                }
                i += 1;
            }
        }
        sum
    }
}

/// Map a compound score to a discrete signal.
fn signal_for(compound: f64) -> Signal {
    match compound {
        c if c >= 0.5 => Signal::StronglyBullish,
        c if c >= 0.15 => Signal::Bullish,
        c if c <= -0.5 => Signal::StronglyBearish,
        c if c <= -0.15 => Signal::Bearish,
        _ => Signal::Neutral,
    }
}

/// Lowercase and strip punctuation, keeping characters meaningful in
/// financial text (`- & % $ '`). Result is space-padded so phrase needles
/// can match on word boundaries.
fn normalize(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\u{2019}' => s.push('\''), // curly apostrophe
            c if c.is_alphanumeric() => {
                for lc in c.to_lowercase() {
                    s.push(lc);
                }
            }
            '-' | '&' | '%' | '$' | '\'' => s.push(ch),
            _ => s.push(' '),
        }
    }
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(" {collapsed} ")
}

/// Replace finance-neutral words with a neutral placeholder before the base
/// VADER pass, so generic valences for words like "gross" or "cancer" do not
/// pollute the score.
fn mask_for_base(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let clean: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .flat_map(|c| c.to_lowercase())
                .collect();
            if lexicon::NEUTRAL_OVERRIDES.contains(&clean.as_str()) {
                "item"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Match multi-word phrases, consuming matched text so the word pass cannot
/// double-count. Returns the summed valence.
fn phrase_pass(norm: &mut String, triggers: &mut Vec<Trigger>) -> f64 {
    let mut sum = 0.0;
    for &(phrase, valence) in phrases::PHRASES {
        let needle = format!(" {phrase} ");
        while let Some(pos) = norm.find(&needle) {
            let end = pos + needle.len();

            let before: Vec<&str> = norm[..pos].split_whitespace().collect();
            let mut v = adjust_for_context(valence, &before);

            // Magnitude amplifier: "<phrase> by 23%".
            let mut after = norm[end..].split_whitespace();
            if after.next() == Some("by")
                && let Some(pct) = after.next().and_then(parse_pct)
                && pct >= 10.0
            {
                v *= 1.25;
            }

            sum += v;
            triggers.push(Trigger {
                term: phrase.to_string(),
                valence: v,
            });
            // Keep a single space so token boundaries survive.
            norm.replace_range(pos..end, " ");
        }
    }
    sum
}

/// Match gap-tolerant verb+object pairs like "beats … expectations" with up
/// to two tokens between lead and tail. Matched tokens are marked consumed
/// so the word pass cannot double-count them.
fn gap_pass(tokens: &[&str], consumed: &mut [bool], triggers: &mut Vec<Trigger>) -> f64 {
    const MAX_GAP: usize = 2;
    let mut sum = 0.0;

    for i in 0..tokens.len() {
        if consumed[i] {
            continue;
        }
        for &(lead, tail, valence) in phrases::GAP_PHRASES {
            if tokens[i] != lead {
                continue;
            }
            let hi = (i + 2 + MAX_GAP).min(tokens.len());
            let Some(j) = (i + 1..hi).find(|&j| !consumed[j] && tokens[j] == tail) else {
                continue;
            };

            let mut v = adjust_for_context(valence, &tokens[..i]);
            // Magnitude amplifier: "<pair> by 23%".
            if tokens.get(j + 1) == Some(&"by")
                && let Some(pct) = tokens.get(j + 2).and_then(|t| parse_pct(t))
                && pct >= 10.0
            {
                v *= 1.25;
            }

            sum += v;
            triggers.push(Trigger {
                term: format!("{lead} … {tail}"),
                valence: v,
            });
            consumed[i] = true;
            consumed[j] = true;
            break;
        }
    }
    sum
}

/// Apply negation flip and booster/dampener rules based on the up-to-three
/// tokens preceding a matched term.
fn adjust_for_context(valence: f64, preceding: &[&str]) -> f64 {
    let window = &preceding[preceding.len().saturating_sub(3)..];
    if window.iter().any(|t| lexicon::NEGATIONS.contains(t)) {
        return -valence * NEGATION_SCALAR;
    }
    if let Some(last) = window.last() {
        if lexicon::BOOSTERS_UP.contains(last) {
            return valence + BOOST * valence.signum();
        }
        if lexicon::BOOSTERS_DOWN.contains(last) {
            return valence - BOOST * valence.signum();
        }
    }
    valence
}

/// Parse a percentage token like `"23%"` into `23.0`.
fn parse_pct(token: &str) -> Option<f64> {
    token.strip_suffix('%')?.parse().ok()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn strongly_bullish_on_beat_and_raise() {
        let fv = FinVader::new();
        let s = fv.analyze("Acme beats Q3 expectations and raises full-year guidance");
        assert!(s.compound > 0.5, "compound was {}", s.compound);
        assert_eq!(s.signal, Signal::StronglyBullish);
        assert!(!s.triggers.is_empty());
    }

    #[test]
    fn negation_flips_phrase() {
        let fv = FinVader::new();
        let s = fv.analyze("Acme failed to beat expectations this quarter");
        assert!(s.compound < 0.0, "compound was {}", s.compound);
    }

    #[test]
    fn gross_margin_is_not_negative() {
        // Generic VADER scores "gross" as disgust; finance says margin.
        let fv = FinVader::new();
        let s = fv.analyze("Gross margin expanded 300 basis points to 58%");
        assert!(s.compound > 0.2, "compound was {}", s.compound);
    }

    #[test]
    fn fda_approval_is_bullish_catalyst_despite_cancer() {
        let fv = FinVader::new();
        let s = fv.analyze("FDA approves Acme's new cancer drug");
        assert!(s.compound > 0.2, "compound was {}", s.compound);
        let cat = s.catalyst.expect("catalyst expected");
        assert!(cat.bullish);
    }

    #[test]
    fn bankruptcy_is_bearish_catalyst() {
        let fv = FinVader::new();
        let s = fv.analyze("Acme files for chapter 11 bankruptcy protection");
        assert!(s.compound < -0.5, "compound was {}", s.compound);
        let cat = s.catalyst.expect("catalyst expected");
        assert!(!cat.bullish);
    }

    #[test]
    fn plain_scheduling_news_is_neutral() {
        let fv = FinVader::new();
        let s = fv.analyze("Acme to report third quarter results on Thursday");
        assert_eq!(s.signal, Signal::Neutral, "compound was {}", s.compound);
    }

    #[test]
    fn magnitude_amplifier_applies() {
        let fv = FinVader::new();
        let small = fv.analyze("Acme beat estimates by 2%");
        let large = fv.analyze("Acme beat estimates by 40%");
        assert!(large.compound > small.compound);
    }

    #[test]
    fn magnitude_amplifier_applies_to_exact_phrases() {
        let fv = FinVader::new();
        let plain = fv.analyze("Q4 revenue fell short");
        let amplified = fv.analyze("Q4 revenue fell short by 25%");
        assert!(amplified.compound < plain.compound);
    }

    #[test]
    fn down_percent_is_bearish() {
        let fv = FinVader::new();
        let s = fv.analyze("Shares down 30% since January");
        assert!(s.compound < -0.15, "compound was {}", s.compound);
    }

    #[test]
    fn boosters_amplify_and_dampeners_soften() {
        let fv = FinVader::new();
        let boosted = fv.analyze("Acme sharply missed estimates");
        let plain = fv.analyze("Acme missed estimates");
        let dampened = fv.analyze("Acme slightly missed estimates");
        assert!(boosted.compound < plain.compound);
        assert!(dampened.compound > plain.compound);
    }

    #[test]
    fn curly_apostrophe_is_normalized() {
        let fv = FinVader::new();
        // U+2019 apostrophe must not split tokens or break matching.
        let s = fv.analyze("Acme\u{2019}s margin expansion continues");
        assert!(s.compound > 0.15, "compound was {}", s.compound);
    }

    #[test]
    fn gap_phrase_by_without_pct_does_not_amplify() {
        let fv = FinVader::new();
        // "by" at end of text → tokens.get(j+2) is None → no amplification
        let s = fv.analyze("Acme beats expectations by");
        let plain = fv.analyze("Acme beats expectations");
        // Without a parseable percentage, the magnitude amplifier should not fire.
        assert!(
            (s.compound - plain.compound).abs() < 0.05,
            "gap 'by' without pct changed compound too much: {} vs {}",
            s.compound,
            plain.compound
        );
    }

    #[test]
    fn default_matches_new() {
        let s = FinVader::default().analyze("Acme beats expectations");
        assert!(s.compound > 0.0);
    }
}
