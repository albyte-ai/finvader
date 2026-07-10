//! Catalytic ("episodic pivot") event detection.
//!
//! A catalyst is a single event that tends to permanently re-rate a stock:
//! an FDA approval, a major contract award, an acquisition offer, an index
//! inclusion, or on the downside a fraud charge or bankruptcy filing.
//! These deserve to be surfaced separately from the smooth sentiment score.

/// A detected catalytic event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalyst {
    /// The pattern that matched, e.g. `"fda approval"`.
    pub pattern: &'static str,
    /// Direction of the expected re-rating.
    pub bullish: bool,
}

/// Substring patterns matched against the normalized text.
const PATTERNS: &[(&str, bool)] = &[
    // regulatory / clinical
    ("fda approval", true),
    ("fda approves", true),
    ("fda approved", true),
    ("fda clearance", true),
    ("breakthrough therapy", true),
    ("met primary endpoint", true),
    ("missed primary endpoint", false),
    ("failed primary endpoint", false),
    ("complete response letter", false),
    ("fda rejects", false),
    ("fda rejection", false),
    // deals
    ("to be acquired", true),
    ("buyout offer", true),
    ("takeover bid", true),
    ("merger agreement", true),
    ("strategic partnership", true),
    // index events
    ("added to s&p", true),
    ("joins s&p", true),
    ("s&p 500 inclusion", true),
    ("index inclusion", true),
    ("removed from s&p", false),
    // contracts
    ("government contract", true),
    ("defense contract", true),
    ("contract award", true),
    ("awarded contract", true),
    // blowout results
    ("record quarter", true),
    ("record revenue", true),
    ("beat and raise", true),
    // distress
    ("chapter 11", false),
    ("chapter 7", false),
    ("going concern", false),
    ("sec charges", false),
    ("accounting fraud", false),
    ("auditor resigns", false),
];

/// Word pairs that signal a catalyst when both appear in the text.
const CO_OCCURRENCE: &[(&str, &str, bool)] = &[
    ("awarded", "contract", true),
    ("wins", "contract", true),
    ("won", "contract", true),
];

/// Detect the first catalytic event in space-padded normalized text.
pub(crate) fn detect(padded_norm: &str) -> Option<Catalyst> {
    for &(pattern, bullish) in PATTERNS {
        // Padded needle keeps matches on word boundaries.
        if padded_norm.contains(&format!(" {pattern} ")) {
            return Some(Catalyst { pattern, bullish });
        }
    }
    for &(a, b, bullish) in CO_OCCURRENCE {
        if padded_norm.contains(&format!(" {a} ")) && padded_norm.contains(&format!(" {b} ")) {
            return Some(Catalyst {
                pattern: a,
                bullish,
            });
        }
    }
    None
}
