/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This is a very minimalistic measurement tool for micro-benchmarks. Its goal is to provide coarse overview of performance improvements
// or regressions, NOT a statistically rigorous analysis. We simply measure wall time (not CPU time) and don't consider specifics of
// the hardware or architecture. There are more sophisticated benchmarking tools, but at the moment there is no need for them:
// - https://github.com/bheisler/criterion.rs
// - https://github.com/Canop/glassbench
// - https://github.com/sharkdp/hyperfine

// We currently avoid mean or max, as we're not that interested in outliers (e.g. CPU spike).
// This may of course obscure bad performance in only small number of cases, but that's something we take into account.
// Instead, we focus on min (fastest run) and median -- even median may vary quite a bit between runs; but it gives an idea of the distribution.
// See also https://easyperf.net/blog/2019/12/30/Comparing-performance-measurements#average-median-minimum.

use std::time::{Duration, Instant};

const WARMUP_RUNS: usize = 200;
const TEST_RUNS: usize = 501; // uneven, so median need not be interpolated.
const METRIC_COUNT: usize = 2;

pub struct BenchResult {
    pub stats: [Duration; METRIC_COUNT],
}

pub fn metrics() -> [&'static str; METRIC_COUNT] {
    ["min", "median"]
}

pub fn run_benchmark(code: fn(), inner_repetitions: usize) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        code();
    }

    let mut times = Vec::with_capacity(TEST_RUNS);
    for _ in 0..TEST_RUNS {
        let start = Instant::now();
        code();
        let duration = start.elapsed();

        times.push(duration / inner_repetitions as u32);
    }
    times.sort();

    calculate_stats(times)
}

fn calculate_stats(times: Vec<Duration>) -> BenchResult {
    // See top of file for rationale.

    /*let mean = {
        let total = times.iter().sum::<Duration>();
        total / TEST_RUNS as u32
    };
    let std_dev = {
        let mut variance = 0;
        for time in times.iter() {
            let diff = time.as_nanos() as i128 - mean.as_nanos() as i128;
            variance += (diff * diff) as u128;
        }
        Duration::from_nanos((variance as f64 / TEST_RUNS as f64).sqrt() as u64)
    };
    let max = times[TEST_RUNS - 1];
    let percentile05 = times[(TEST_RUNS as f64 * 0.05) as usize];
    */

    // Interpolating percentiles is not that important.
    let min = times[0];
    let median = times[TEST_RUNS / 2];

    BenchResult {
        stats: [min, median],
    }
}
