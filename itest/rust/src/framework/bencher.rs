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
//
// Each sample repeats the operation until the timed window reaches MIN_SAMPLE_TIME, so the two Instant::now() calls per sample (~20-30ns
// each) cost ~1% even for single-nanosecond operations. The window is kept small on purpose: it is far below the scheduler quantum,
// so most samples run without interruption and `min` remains a meaningful best case rather than an average over noise.
//
// We currently avoid mean or max, as we're not that interested in outliers (e.g. CPU spike).
// This may of course obscure bad performance in only small number of cases, but that's something we take into account.
// Instead, we focus on min (fastest run) and median -- even median may vary quite a bit between runs; but it gives an idea of the distribution.
// See also https://easyperf.net/blog/2019/12/30/Comparing-performance-measurements#average-median-minimum.
// We also measure noise as IQR/mean and flag benchmarks that exceed a threshold; see below.
//
// Comparing two commits is coarser than the sample statistics suggest. Repeated runs of one binary differ by a few percent, two builds by
// more: `itest` instantiates godot-core's generics, so a changed function set moves inlining and code placement for untouched code.
// `check.sh bench` limits this, but differences of a few percent stay unattributable; count instructions instead (`objdump`, `iai-callgrind`).
//
// Above ~10ns per operation, that spread stems from clock drift, code/data layout and allocator state, so it does not shrink with more
// samples. Benchmarks are therefore measured in PASSES interleaved passes. Spreading a transient over all passes dampens it a little,
// but the point is detection: passes that disagree get flagged, which a single long run cannot do. See anchor_bench.rs.

use std::any::TypeId;
use std::time::{Duration, Instant};

/// Lower bound for the timed window of one sample; faster operations are repeated until reached.
const MIN_SAMPLE_TIME: Duration = Duration::from_micros(5);

/// Time spent warming up caches and estimating the cost of one operation.
const WARMUP_TIME: Duration = Duration::from_millis(5);

/// Time budget for one benchmark across all passes, including warmup. No hard cap: the sample count is derived from it up front, and
/// `MIN_SAMPLES` are always run.
const TOTAL_BUDGET: Duration = Duration::from_millis(50);

/// Lower bound on the sample count per pass, even if that exceeds the budget. Uneven, so the median needs no interpolation.
/// Only reached above ~0.4ms per operation; no benchmark is currently that slow.
const MIN_SAMPLES: usize = 31;

/// Ceiling, to bound the suite runtime. More samples give the `min` metric more chances to catch an uninterrupted run, but cost time
/// (e.g. raising 501 -> 2001 reduced the run-to-run spread by ~2.5x, for +0.7s runtime over the whole suite.)
const MAX_SAMPLES: usize = 2001;

/// Number of interleaved passes over the whole suite, spreading a benchmark's samples over the entire run.
/// Costs one warmup per pass; the sample count is divided, so the timed work stays the same.
pub const PASSES: usize = 3;

/// Upper bound for repetitions per sample. Even a 0.05ns operation needs far fewer. Also limits the inputs held by bench_measure_batched().
const MAX_REPETITIONS: usize = 100_000;

/// Interquartile range divided by median, above which a measurement is considered noisy.
///
/// Calibrated so that a quiet machine flags almost nothing (2 of 69 benches, against 6 while the machine was busy compiling).
///
/// Our bencher reports only the flag, not the ratio itself. It sometimes measures the machine rather than the code, so a column of fluctuating
/// percentages would invite comparing those values over time, which may not contain useful information. The flag means "look further into this".
const NOISY_IQR_RATIO: f64 = 0.05;

/// How far the `min` of the individual passes may disagree before the benchmark is considered noisy.
///
/// Higher than [`NOISY_IQR_RATIO`]: passes span the whole run and thus also pick up drift, which is a few percent even when nothing
/// is wrong.
const NOISY_PASS_SPREAD: f64 = 0.10;

/// Upper bound on the samples of one pass, so that all passes together stay at [`MAX_SAMPLES`]. Uneven, like [`MIN_SAMPLES`].
const MAX_SAMPLES_PER_PASS: usize = (MAX_SAMPLES / PASSES) | 1;

const METRIC_COUNT: usize = 2;

pub type BenchResult = Result<BenchMeasurement, String>;

pub struct BenchMeasurement {
    /// Per-operation timings in nanoseconds. Release-mode operations are often sub-nanosecond, which `Duration` cannot represent.
    pub stats: [f64; METRIC_COUNT],

    /// Whether the samples were too spread out to be trusted.
    pub noisy: bool,

    /// How far the `min` of the individual passes moved, relative to the smallest one. Zero for a single pass.
    ///
    /// Covers only what varies within a run; code layout is fixed per binary, so this says nothing about how far two builds may differ.
    pub pass_spread: f64,
}

pub fn metrics() -> [&'static str; METRIC_COUNT] {
    ["min", "median"]
}

/// Measures the timing of the passed closure.
///
/// Used by both `#[bench]` automatic mode (generated by macro) and `#[bench(manual)]` (called explicitly).
/// Repetition count and sample count are tuned automatically; see module docs.
///
/// Returns `Err(String)` if there is something wrong with the benchmark setup.
pub fn bench_measure<R: 'static>(mut work: impl FnMut() -> R) -> BenchResult {
    check_non_unit::<R>()?;

    let (op_cost, start) = warmup(&mut work);
    let inner = repetitions_for(op_cost);

    sample_within_budget(start, op_cost * inner as f64, inner, |times| {
        let begin = Instant::now();
        for _ in 0..inner {
            bench_used(work());
        }
        times.push(begin.elapsed());
    })
}

/// Like [`bench_measure`], but for operations consuming their input, such as `Drop`.
///
/// `make_input` is called once per operation, outside the timed window, with a running index.
pub fn bench_measure_batched<I, R: 'static>(
    mut make_input: impl FnMut(usize) -> I,
    mut work: impl FnMut(I) -> R,
) -> BenchResult {
    check_non_unit::<R>()?;

    // Inputs cannot be reused, so each round builds its own batch. Rounds grow like in warmup(), and the cheapest one is the estimate.
    let start = Instant::now();
    let (mut op_cost, mut setup_cost) = (f64::MAX, f64::MAX);
    let mut chunk = 1;

    // Runs past WARMUP_TIME while an estimate is still missing; chunks keep growing until their window exceeds clock granularity.
    while start.elapsed() < WARMUP_TIME || op_cost == f64::MAX || setup_cost == f64::MAX {
        let setup_begin = Instant::now();
        let inputs: Vec<I> = (0..chunk).map(&mut make_input).collect();

        let work_begin = Instant::now();
        for input in inputs {
            bench_used(work(input));
        }

        record_min(&mut setup_cost, work_begin - setup_begin, chunk);
        record_min(&mut op_cost, work_begin.elapsed(), chunk);
        chunk = (chunk * 2).min(MAX_REPETITIONS);
    }

    let inner = repetitions_for(op_cost);

    // Building the inputs is untimed, but still costs wall time, so the budget must include it.
    let sample_cost = (op_cost + setup_cost) * inner as f64;

    let take_sample = |times: &mut Vec<Duration>| {
        // The loop consumes the `Vec` and thus deallocates its buffer inside the timed window; amortized over `inner` this is negligible.
        let inputs: Vec<I> = (0..inner).map(&mut make_input).collect();
        let begin = Instant::now();
        for input in inputs {
            bench_used(work(input));
        }
        times.push(begin.elapsed());
    };

    sample_within_budget(start, sample_cost, inner, take_sample)
}

/// Keeps the cheapest per-op cost seen so far.
///
/// A window of zero means it stayed below clock granularity (~100ns for QPC on Windows) and is ignored: rounds are combined with min(),
/// so recording it would pin the estimate to zero for the entire benchmark.
fn record_min(cost: &mut f64, window: Duration, count: usize) {
    if !window.is_zero() {
        *cost = cost.min(ns_per_op(window, count));
    }
}

fn check_non_unit<R: 'static>() -> Result<(), String> {
    if TypeId::of::<R>() == TypeId::of::<()>() {
        return Err("benchmark closure must return non-unit type to prevent the computation from being optimized away".to_string());
    }
    Ok(())
}

/// Runs `work` for [`WARMUP_TIME`], returning the estimated cost of one operation in nanoseconds, and the benchmark start time.
fn warmup<R>(mut work: impl FnMut() -> R) -> (f64, Instant) {
    let start = Instant::now();
    let mut iterations = 0u64;
    let mut chunk = 1;
    let mut elapsed = Duration::ZERO;

    // Doubling keeps clock checks rare for cheap operations, while an expensive one overshoots WARMUP_TIME by at most 2x.
    while elapsed < WARMUP_TIME {
        for _ in 0..chunk {
            bench_used(work());
        }

        iterations += chunk;
        chunk *= 2;
        elapsed = start.elapsed();
    }

    let estimate_ns = ns_per_op(elapsed, iterations as usize);
    (estimate_ns, start)
}

/// Number of repetitions needed to fill one sample window.
fn repetitions_for(op_cost: f64) -> usize {
    let inner = (MIN_SAMPLE_TIME.as_nanos() as f64 / op_cost).ceil();

    (inner as usize).clamp(1, MAX_REPETITIONS)
}

/// Collects as many samples as the remaining budget affords, but always at least [`MIN_SAMPLES`].
fn sample_within_budget(
    start: Instant,
    sample_cost: f64,
    inner: usize,
    mut take_sample: impl FnMut(&mut Vec<Duration>),
) -> BenchResult {
    // Each pass may spend its share of the budget; without the division, a benchmark slow enough for the budget to bind would take
    // PASSES times as long.
    let budget = TOTAL_BUDGET / PASSES as u32;
    let remaining = budget.saturating_sub(start.elapsed()).as_nanos() as f64;
    let affordable = (remaining / sample_cost) as usize;
    let samples = affordable.clamp(MIN_SAMPLES, MAX_SAMPLES_PER_PASS) | 1;

    let mut times = Vec::with_capacity(samples);
    for _ in 0..samples {
        take_sample(&mut times);
    }
    times.sort();

    Ok(calculate_stats(times, inner))
}

/// Calculate per-operation stats (nanoseconds) from multiple sample windows.
fn calculate_stats(times: Vec<Duration>, inner: usize) -> BenchMeasurement {
    // See top of file for rationale.
    let min = ns_per_op(times[0], inner);
    let median = ns_per_op(times[times.len() / 2], inner);

    // Interpolating percentiles is not that important.
    let iqr =
        ns_per_op(times[times.len() * 3 / 4], inner) - ns_per_op(times[times.len() / 4], inner);
    let noisy = iqr > median * NOISY_IQR_RATIO;

    BenchMeasurement {
        stats: [min, median],
        noisy,
        pass_spread: 0.0,
    }
}

/// Combines the passes of one benchmark into a single measurement.
///
/// `min` is the smallest sample of all passes, `median` the median of the per-pass medians (close enough at this resolution). Noisy if the
/// majority of passes were -- so a single transient doesn't flag it -- or if the passes' minima differ by more than [`NOISY_PASS_SPREAD`].
pub fn merge_passes(passes: Vec<BenchMeasurement>) -> BenchMeasurement {
    let noisy_count = passes.iter().filter(|pass| pass.noisy).count();

    let mut medians: Vec<f64> = passes.iter().map(|pass| pass.stats[1]).collect();
    medians.sort_by(f64::total_cmp);
    let median = medians[medians.len() / 2];

    let mins = passes.iter().map(|pass| pass.stats[0]);
    let min = mins.clone().fold(f64::INFINITY, f64::min);
    let max = mins.fold(0.0, f64::max);

    // A zero minimum means the sample window stayed below clock granularity, so the passes cannot be said to agree on anything.
    let pass_spread = if min > 0.0 {
        (max - min) / min
    } else {
        f64::INFINITY
    };

    BenchMeasurement {
        stats: [min, median],
        noisy: noisy_count * 2 > passes.len() || pass_spread > NOISY_PASS_SPREAD,
        pass_spread,
    }
}

/// Nanoseconds spent per operation, given a window covering `count` of them.
fn ns_per_op(window: Duration, count: usize) -> f64 {
    window.as_nanos() as f64 / count as f64
}

/// Signal to the compiler that a value is used (to avoid optimization).
fn bench_used<T: Sized>(value: T) {
    // The following check would be used to prevent `()` arguments, ensuring that a value from the bench is actually going into the blackbox.
    // However, we run into this issue, despite no array being used: https://github.com/rust-lang/rust/issues/43408.
    //   error[E0401]: can't use generic parameters from outer function
    // sys::static_assert!(std::mem::size_of::<T>() != 0, "returned unit value in benchmark; make sure to use a real value");

    std::hint::black_box(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measurement(min: f64, median: f64, noisy: bool) -> BenchMeasurement {
        BenchMeasurement {
            stats: [min, median],
            noisy,
            pass_spread: 0.0,
        }
    }

    #[test]
    fn merged_passes_combine_stats() {
        let merged = merge_passes(vec![
            measurement(12.0, 22.0, false),
            measurement(10.0, 20.0, false),
            measurement(11.0, 21.0, false),
        ]);

        assert_eq!(merged.stats[0], 10.0); // Smallest sample of all passes, which is also the smallest overall.
        assert_eq!(merged.stats[1], 21.0); // Median of the per-pass medians.
        assert_eq!(merged.pass_spread, 0.2);
        assert!(merged.noisy); // 20% apart, well above NOISY_PASS_SPREAD.
    }

    #[test]
    fn merged_passes_need_a_majority_to_flag_noise() {
        let agreeing = [10.0, 10.2, 10.4]; // Below NOISY_PASS_SPREAD, so only the per-pass flags decide.

        let single = merge_passes(
            agreeing
                .iter()
                .enumerate()
                .map(|(i, min)| measurement(*min, 20.0, i == 0))
                .collect(),
        );
        assert!(!single.noisy);

        let majority = merge_passes(
            agreeing
                .iter()
                .enumerate()
                .map(|(i, min)| measurement(*min, 20.0, i < 2))
                .collect(),
        );
        assert!(majority.noisy);
    }

    #[test]
    fn merged_passes_below_clock_granularity_are_noisy() {
        let merged = merge_passes(vec![
            measurement(0.0, 0.0, false),
            measurement(0.0, 1.0, false),
        ]);

        assert_eq!(merged.pass_spread, f64::INFINITY);
        assert!(merged.noisy);
    }

    #[test]
    fn repetitions_fill_sample_window() {
        assert_eq!(repetitions_for(1_000_000.0), 1); // 1ms operation: a single repetition already exceeds the window.
        assert_eq!(repetitions_for(5.0), 1000);
        assert_eq!(repetitions_for(0.5), 10_000); // Sub-nanosecond operations must not collapse to a single repetition.
        assert_eq!(repetitions_for(0.0), MAX_REPETITIONS); // Degenerate estimate must stay survivable, rather than saturating.
    }

    #[test]
    fn zero_windows_do_not_pin_the_estimate() {
        let mut cost = f64::MAX;

        record_min(&mut cost, Duration::ZERO, 1);
        assert_eq!(cost, f64::MAX); // Below clock granularity: no information, so no estimate.

        record_min(&mut cost, Duration::from_nanos(500), 100);
        record_min(&mut cost, Duration::from_nanos(900), 100);
        assert_eq!(cost, 5.0); // Cheapest round wins.
    }

    #[test]
    fn stats_are_per_operation() {
        let times = vec![
            Duration::from_nanos(300),
            Duration::from_nanos(400),
            Duration::from_nanos(500),
        ];

        assert_eq!(calculate_stats(times, 100).stats, [3.0, 4.0]);
    }

    #[test]
    fn noisy_flag_reacts_to_spread() {
        // Sorted, as calculate_stats() expects.
        let tight: Vec<_> = (0..101)
            .map(|i| Duration::from_nanos(1000 + i / 50))
            .collect();
        let spread: Vec<_> = (0..101)
            .map(|i| Duration::from_nanos(1000 + i * 10))
            .collect();

        assert!(!calculate_stats(tight, 1).noisy);
        assert!(calculate_stats(spread, 1).noisy);
    }
}
