//! Side-by-side comparison of generic VADER and finvader on headlines
//! where the finance context changes the meaning.
//!
//! Run: `cargo run --example demo`

use finvader::FinVader;
use vader_sentimental::SentimentIntensityAnalyzer;

fn main() {
    let fv = FinVader::new();
    let base = SentimentIntensityAnalyzer::new();

    let headlines = [
        "Acme beats Q3 expectations and raises full-year guidance",
        "Gross margin expanded 300 basis points to 58%",
        "FDA approves Acme's new cancer drug",
        "Acme completes previously announced debt refinancing",
        "Acme misses estimates and cuts guidance",
        "Auditor raises going concern doubt",
        "Acme failed to beat expectations this quarter",
        "Acme to report third quarter results on Thursday",
    ];

    println!("{:>8}  {:>8}  {:<16}  headline", "vader", "finvader", "signal");
    println!("{}", "-".repeat(100));
    for text in headlines {
        let b = base.polarity_scores(text);
        let f = fv.analyze(text);
        let catalyst = f
            .catalyst
            .as_ref()
            .map(|c| format!("  [catalyst: {} {}]", c.pattern, if c.bullish { "↑" } else { "↓" }))
            .unwrap_or_default();
        println!(
            "{:>+8.3}  {:>+8.3}  {:<16}  {}{}",
            b.compound,
            f.compound,
            format!("{:?}", f.signal),
            text,
            catalyst
        );
    }
}
