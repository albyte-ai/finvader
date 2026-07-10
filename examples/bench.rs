//! Throughput benchmark for finvader and the underlying generic VADER.
//!
//! Run: `cargo run --release --example bench`

use std::time::Instant;

use finvader::FinVader;
use vader_sentimental::SentimentIntensityAnalyzer;

const DATA: &str = include_str!("../data/headlines.tsv");
const ITERATIONS: usize = 20_000;

fn main() {
    let headlines: Vec<&str> = DATA
        .lines()
        .filter_map(|l| l.split_once('\t').map(|(_, t)| t))
        .collect();
    let n = headlines.len();

    let fv = FinVader::new();
    let base = SentimentIntensityAnalyzer::new();

    // Warm up.
    for text in &headlines {
        std::hint::black_box(fv.analyze(text));
        std::hint::black_box(base.polarity_scores(text));
    }

    let start = Instant::now();
    for i in 0..ITERATIONS {
        std::hint::black_box(base.polarity_scores(headlines[i % n]));
    }
    let base_elapsed = start.elapsed();

    let start = Instant::now();
    for i in 0..ITERATIONS {
        std::hint::black_box(fv.analyze(headlines[i % n]));
    }
    let fv_elapsed = start.elapsed();

    let base_us = base_elapsed.as_micros() as f64 / ITERATIONS as f64;
    let fv_us = fv_elapsed.as_micros() as f64 / ITERATIONS as f64;

    println!("iterations: {ITERATIONS} over {n} distinct headlines");
    println!(
        "generic VADER (rust): {base_us:.1} us/headline  ({:.0} headlines/sec)",
        1_000_000.0 / base_us
    );
    println!(
        "finvader:             {fv_us:.1} us/headline  ({:.0} headlines/sec)",
        1_000_000.0 / fv_us
    );
}
