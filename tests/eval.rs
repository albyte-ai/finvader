//! Accuracy benchmark: finvader vs generic VADER on labeled financial
//! headlines. Run with `cargo test --test eval -- --nocapture` to see the
//! full comparison table.

use finvader::FinVader;
use vader_sentimental::SentimentIntensityAnalyzer;

const DATA: &str = include_str!("../data/headlines.tsv");

/// Standard VADER classification convention: +-0.05 compound threshold.
fn classify(compound: f64) -> i8 {
    if compound >= 0.05 {
        1
    } else if compound <= -0.05 {
        -1
    } else {
        0
    }
}

fn labeled_headlines() -> Vec<(i8, &'static str)> {
    DATA.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let (label, text) = line.split_once('\t').expect("label<TAB>text");
            let want = match label {
                "B" => 1,
                "S" => -1,
                "N" => 0,
                other => panic!("unknown label {other:?}"),
            };
            (want, text)
        })
        .collect()
}

#[test]
fn beats_generic_vader_on_financial_headlines() {
    let fv = FinVader::new();
    let base = SentimentIntensityAnalyzer::new();

    let headlines = labeled_headlines();
    let total = headlines.len();
    let mut fv_correct = 0;
    let mut base_correct = 0;
    let mut fv_misses = Vec::new();

    for (want, text) in &headlines {
        let f = fv.analyze(text);
        let b = base.polarity_scores(text);
        if classify(f.compound) == *want {
            fv_correct += 1;
        } else {
            fv_misses.push((*text, f.compound, *want));
        }
        if classify(b.compound) == *want {
            base_correct += 1;
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let total_f = total as f64;
    let fv_acc = f64::from(fv_correct) / total_f;
    let base_acc = f64::from(base_correct) / total_f;

    println!("== labeled financial headlines ({total}) ==");
    println!(
        "finvader:      {fv_correct}/{total} = {:.1}%",
        fv_acc * 100.0
    );
    println!(
        "generic VADER: {base_correct}/{total} = {:.1}%",
        base_acc * 100.0
    );
    for (text, compound, want) in &fv_misses {
        println!("finvader MISS (want {want:+}, got {compound:+.3}): {text}");
    }

    assert!(
        fv_acc > base_acc,
        "finvader ({fv_acc:.3}) must beat generic VADER ({base_acc:.3})"
    );
    assert!(
        fv_acc >= 0.85,
        "finvader accuracy {fv_acc:.3} below the 0.85 bar"
    );
}
