//! Measurement-only evaluation of the memory estimator.
//!
//! Compiled only under the `mem_eval` feature. It installs a process-wide
//! tracking allocator that *measures* (never limits) real heap allocations, then
//! compares the actual peak heap usage of an analysis against the estimate
//! produced by `ResourceLimiter`. This is purely a development/validation tool
//! and should never be a part of a normal library build in order to avoid
//! overriding the global allocator in a library context.
//!
//! Run with:
//!   cargo test --release --features mem_eval -- evaluate_memory_estimate --nocapture

use crate::test::generators::{build_analyzer, gen_network_with_threshold, gen_symmetric_network};
use crate::{fbas::QuorumSetMap, FbasAnalyzer, ResourceLimiter};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct TrackingAllocator;

#[derive(Clone, Copy)]
struct Counters {
    current: usize,
    peak: usize,
}
thread_local! {
    /// Per-thread live and peak memory bytes. Thread-local (not global atomics)
    /// so concurrent tests on other threads can't inflate this thread's peak.
    /// The analyzer solves synchronously on the test thread, so all of its
    /// allocations and their frees on `drop` land on the same thread's counters.
    static COUNTERS: Cell<Counters> = const { Cell::new(Counters { current: 0, peak: 0 }) };
}

/// Tracking allocator that counts bytes allocated and freed and the peak memory.
/// `realloc` is intentionally left to the trait default, which routes through
/// `alloc`/`dealloc` above and is therefore counted.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        COUNTERS.with(|c| {
            let mut v = c.get();
            v.current += layout.size();
            if v.current > v.peak {
                v.peak = v.current;
            }
            c.set(v);
        });
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        COUNTERS.with(|c| {
            let mut v = c.get();
            v.current = v.current.saturating_sub(layout.size());
            c.set(v);
        });
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator;

fn reset_peak_to_current() -> usize {
    COUNTERS.with(|c| {
        let mut v = c.get();
        v.peak = v.current;
        c.set(v);
        v.current
    })
}

fn peak() -> usize {
    COUNTERS.with(|c| c.get().peak)
}

struct Row {
    name: String,
    actual: usize,
    estimate: usize,
}

fn eval_dir(dir: &str, rows: &mut Vec<Row>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let path_str = path.to_str().unwrap().to_string();

        // Clone the limiter so we can read its estimate after it has been moved
        // into the analyzer (it is an Rc<RefCell<..>>, so the clone shares
        // state).
        let limiter = ResourceLimiter::new(u64::MAX, usize::MAX);
        let probe = limiter.clone();

        // Everything allocated up to here is part of the baseline.
        let baseline = reset_peak_to_current();
        let mut analyzer = FbasAnalyzer::from_json_path(&path_str, limiter).unwrap();
        analyzer.solve().unwrap();
        let actual = peak().saturating_sub(baseline);
        let estimate = probe.get_peak_mem_bytes();
        drop(analyzer);

        rows.push(Row {
            name,
            actual,
            estimate,
        });
    }
}

fn eval_case(name: &str, qsm: QuorumSetMap, rows: &mut Vec<Row>) {
    let limiter = ResourceLimiter::new(u64::MAX, usize::MAX);
    let probe = limiter.clone();
    let baseline = reset_peak_to_current();
    let mut analyzer = build_analyzer(qsm, limiter).unwrap();
    analyzer.solve().unwrap();
    let actual = peak().saturating_sub(baseline);
    let estimate = probe.get_peak_mem_bytes();
    drop(analyzer);
    rows.push(Row {
        name: name.to_string(),
        actual,
        estimate,
    });
}

/// Large generated cases that exercise construction-heavy memory
/// (wide thresholds) and real solver search (symmetric networks).
fn eval_generated(rows: &mut Vec<Row>) {
    eval_case(
        "gen:wide_threshold(16,8)",
        gen_network_with_threshold(16, 8),
        rows,
    );
    eval_case(
        "gen:wide_threshold(20,10)",
        gen_network_with_threshold(20, 10),
        rows,
    );
    eval_case(
        "gen:symmetric(14,3,10,2)",
        gen_symmetric_network(14, 3, 10, 2),
        rows,
    );
    eval_case(
        "gen:symmetric(16,4,11,3)",
        gen_symmetric_network(16, 4, 11, 3),
        rows,
    );
}

/// An over-wide quorum set must be relaxed to an empty qset (not aborted, not
/// errored), so the analysis completes while using only a tiny amount of memory.
#[test]
fn test_wide_qset_does_not_blow_up_actual_memory() {
    let limiter = ResourceLimiter::new(u64::MAX, usize::MAX);
    let baseline = reset_peak_to_current();
    let mut analyzer = build_analyzer(gen_network_with_threshold(40, 20), limiter)
        .expect("wide qset should be relaxed");
    let ok = analyzer.solve().is_ok();
    let actual = peak().saturating_sub(baseline);
    println!("relaxation: wide_threshold(40,20) solved={ok} actual_peak={actual}");
    assert!(ok, "relaxed wide qset should solve");
    assert!(
        actual < 10 * 1024 * 1024,
        "relaxed wide qset used {actual} bytes - blow-up not prevented?"
    );
}

#[test]
fn evaluate_memory_estimate() {
    const MAX_OVERESTIMATE_FACTOR: f64 = 3.0;

    let mut rows = Vec::new();
    eval_dir("./tests/test_data/", &mut rows);
    eval_dir("./tests/test_data/random/", &mut rows);
    eval_generated(&mut rows);
    rows.sort_by(|a, b| b.actual.cmp(&a.actual));

    println!(
        "\n{:<58} {:>12} {:>12} {:>10}",
        "case", "actual(B)", "estimate(B)", "est/act"
    );
    println!("{}", "-".repeat(108));

    for r in &rows {
        let ratio = r.estimate as f64 / r.actual as f64;

        assert!(
            ratio >= 1.0,
            "estimator under-estimated real memory for case {}; estimate={}; actual={}",
            r.name,
            r.estimate,
            r.actual
        );
        assert!(
            ratio <= MAX_OVERESTIMATE_FACTOR,
            "estimator over-estimated real memory by more than {MAX_OVERESTIMATE_FACTOR}x for case {}; estimate={}; actual={}",
            r.name,
            r.estimate,
            r.actual
        );

        println!(
            "{:<58} {:>12} {:>12} {:>10.2}",
            r.name, r.actual, r.estimate, ratio,
        );
    }

    println!("{}", "-".repeat(108));
}
