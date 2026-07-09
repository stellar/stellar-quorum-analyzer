use crate::FbasError;
use batsat::{
    callbacks::{Callbacks, ProgressStatus},
    lbool,
};
use log::{error, trace};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

// Estimation constants used to approximate memory consumption without a global
// allocator. They are deliberate *over*-estimates of the real layout so that the
// soft cap trips before actual memory would be exhausted. They do not need to be
// exact -- they only need to grow monotonically with the work performed and stay
// on the conservative (high) side.

// Fixed allocation batsat makes on `Solver::new`: its `ClauseAllocator` reserves
// a clause region of `1024 * 1024` `ClauseData` (`u32`) words = 4 MiB up front,
// regardless of problem size. We add headroom for the other fixed-size solver
// state. This dominates small instances, where the count-based terms below are
// negligible. (Measured against real allocations via the `mem_eval` feature.)
const SOLVER_BASE_BYTES: usize = 5 * 1024 * 1024;
// Per clause literal: each literal is a `u32` word in the clause region, and the
// backing `Vec` keeps up to ~2x slack as it doubles, so we budget 8 bytes.
const LIT_BYTES: usize = 8;
// Per clause: header words plus the two watch-list entries (cref + blocker) that
// reference it, each in `Vec`s that carry growth slack.
const CLAUSE_OVERHEAD_BYTES: usize = 48;
// Per variable: assignment, reason, activity, polarity, order-heap slot, `seen`
// flag and the two per-literal watch vectors, all `IntMap`-backed with slack.
const PER_VAR_BYTES: usize = 128;
// The clause region is a `Vec` that doubles its capacity, so the bytes actually
// reserved can be up to ~2x the used size that `on_gc` reports. Applied to the
// `on_gc` ground-truth measurement.
const REGION_CAPACITY_SLACK: usize = 2;
// Multiplier on the *current* learnt-clause count when estimating solver memory
// during search, to cover growth between batsat's sparse progress callbacks.
const LEARNT_GROWTH_MARGIN: usize = 2;

/// Conservative estimate of the bytes held by the SAT solver, derived purely
/// from variable / clause / literal counts (which batsat exposes) plus batsat's
/// fixed up-front allocation, without observing the allocator. Tuned to
/// *over*-estimate real usage -- see the `mem_eval` feature for the evaluation
/// harness that validates this.
pub(crate) fn estimate_solver_bytes(
    num_vars: usize,
    total_clauses: usize,
    total_lits: usize,
) -> usize {
    SOLVER_BASE_BYTES
        .saturating_add(total_lits.saturating_mul(LIT_BYTES))
        .saturating_add(total_clauses.saturating_mul(CLAUSE_OVERHEAD_BYTES))
        .saturating_add(num_vars.saturating_mul(PER_VAR_BYTES))
}

/// Saturating binomial coefficient `C(n, k)`, capped at `usize::MAX`.
///
/// The Tseitin encoding in `construct_formula` enumerates `C(K, threshold)`
/// combinations of a vertex's successors, allocating one auxiliary variable and
/// several clauses per combination. For a wide quorum set this count is
/// astronomically large, so we must be able to project it *before* allocating
/// anything (a single `Vec::with_capacity(C(K,T))` would otherwise abort the
/// process, bypassing the soft cap entirely). Intermediate products use `u128`
/// and the whole result saturates rather than overflowing.
pub(crate) fn estimate_combinations(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    // C(n, k) == C(n, n - k); pick the smaller factor count.
    let k = k.min(n - k);
    let mut result: u128 = 1;
    for i in 0..k as u128 {
        result = result.saturating_mul(n as u128 - i) / (i + 1);
        if result >= usize::MAX as u128 {
            return usize::MAX;
        }
    }
    result as usize
}

#[derive(Clone, Debug, Copy)]
pub struct ResourceQuantity {
    pub time: Duration,
    pub mem_bytes: usize,
}

impl ResourceQuantity {
    pub fn zero() -> Self {
        ResourceQuantity {
            time: Duration::ZERO,
            mem_bytes: 0,
        }
    }

    pub fn new(time_ms: u64, mem_bytes: usize) -> Self {
        ResourceQuantity {
            time: Duration::from_millis(time_ms),
            mem_bytes,
        }
    }

    pub fn exceeds(&self, limit: &Self) -> bool {
        self.time > limit.time || self.mem_bytes > limit.mem_bytes
    }
}

/// An implementation of the `Callbacks` trait that tracks and limits the memory
/// usage and processing time of the solver.
///
/// Memory is *estimated* rather than measured: the analyzer charges the cost of
/// the data structures it builds via [`ResourceLimiter::account_structural`],
/// and the solver's footprint is estimated from the clause/literal counts batsat
/// reports through [`Callbacks::on_progress`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResourceLimiterImpl {
    start_time: Instant,
    limits: ResourceQuantity,
    current_usage: ResourceQuantity,
    // Bytes held by our own intermediate structures (parsed quorum set map, the
    // petgraph graph, variable map). Set as an absolute value by the analyzer.
    structural_bytes: usize,
    // Estimated bytes held by the SAT solver (original + learnt clauses).
    solver_bytes: usize,

    // Highest `current_usage.mem_bytes` observed so far. Tracked for diagnostics
    // and for evaluating the estimator against real allocations; not used for
    // enforcement (the limit is checked against the current usage).
    #[cfg(feature = "mem_eval")]
    peak_mem_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceLimiter(Rc<RefCell<ResourceLimiterImpl>>);

impl ResourceLimiterImpl {
    pub fn new(time_limit_ms: u64, mem_limit_bytes: usize) -> Self {
        Self {
            start_time: Instant::now(),
            limits: ResourceQuantity::new(time_limit_ms, mem_limit_bytes),
            current_usage: ResourceQuantity::zero(),
            structural_bytes: 0,
            solver_bytes: 0,
            #[cfg(feature = "mem_eval")]
            peak_mem_bytes: 0,
        }
    }

    fn measure(&mut self, verbose: bool) {
        let time = self.start_time.elapsed();
        let mem_bytes = self.structural_bytes.saturating_add(self.solver_bytes);
        #[cfg(feature = "mem_eval")]
        {
            self.peak_mem_bytes = self.peak_mem_bytes.max(mem_bytes);
        }
        self.current_usage = ResourceQuantity { time, mem_bytes };
        if verbose {
            trace!( target: "SCP",
                "Time elapsed: {} ms, Time limit: {} ms; Memory usage: {} bytes, Memory limit: {} bytes",
                self.current_usage.time.as_millis(), self.limits.time.as_millis(), self.current_usage.mem_bytes, self.limits.mem_bytes
            );
        }
    }

    fn measure_and_enforce_limits(&mut self) -> Result<(), FbasError> {
        self.measure(false);
        if self.current_usage.exceeds(&self.limits) {
            error!( target: "SCP",
                "Resource limits exceeded -- Time elapsed: {} ms, Time limit: {} ms; Memory usage: {} bytes, Memory limit: {} bytes",
                self.current_usage.time.as_millis(), self.limits.time.as_millis(), self.current_usage.mem_bytes, self.limits.mem_bytes
            );
            return Err(FbasError::ResourcelimitExceeded(self.current_usage));
        }
        Ok(())
    }
}

impl ResourceLimiter {
    pub fn new(time_limit_ms: u64, mem_limit_bytes: usize) -> Self {
        Self(Rc::new(RefCell::new(ResourceLimiterImpl::new(
            time_limit_ms,
            mem_limit_bytes,
        ))))
    }

    #[cfg(test)]
    pub(crate) fn unlimited() -> Self {
        Self(Rc::new(RefCell::new(ResourceLimiterImpl::new(
            u64::MAX,
            usize::MAX,
        ))))
    }

    pub(crate) fn measure(&self, verbose: bool) {
        self.0.borrow_mut().measure(verbose);
    }

    pub fn measure_and_enforce_limits(&self) -> Result<(), FbasError> {
        self.0.borrow_mut().measure_and_enforce_limits()
    }

    /// Record the (absolute) byte cost of our own intermediate data structures
    /// and enforce the limit.
    pub(crate) fn account_structural(&self, bytes: usize) -> Result<(), FbasError> {
        let mut inner = self.0.borrow_mut();
        inner.structural_bytes = bytes;
        inner.measure_and_enforce_limits()
    }

    /// Record the (absolute) estimated byte cost of the SAT solver and enforce
    /// the limit.
    pub(crate) fn account_solver(&self, bytes: usize) -> Result<(), FbasError> {
        let mut inner = self.0.borrow_mut();
        inner.solver_bytes = bytes;
        inner.measure_and_enforce_limits()
    }

    pub fn get_time_ms(&self) -> u64 {
        self.0.borrow().current_usage.time.as_millis() as u64
    }

    pub fn get_mem_bytes(&self) -> usize {
        self.0.borrow().current_usage.mem_bytes
    }

    /// Highest estimated memory usage observed over this limiter's lifetime.
    #[cfg(all(test, feature = "mem_eval"))]
    pub(crate) fn get_peak_mem_bytes(&self) -> usize {
        self.0.borrow().peak_mem_bytes
    }
}

impl Callbacks for ResourceLimiter {
    fn on_start(&mut self) {
        self.measure(true);
        trace!( target: "SCP",
            "c ====================================[ Search Statistics ]===================================="
        );
        trace!( target: "SCP",
            "c | Conflicts |          ORIGINAL         |          LEARNT          |        Resource        |"
        );
        trace!( target: "SCP",
            "c |           |    Vars  Clauses Literals |    Limit  Clauses Lit/Cl | Time(ms) Memory(bytes) |"
        );
        trace!( target: "SCP",
            "c ============================================================================================="
        );
    }

    fn on_result(&mut self, _: lbool) {
        trace!( target: "SCP",
            "c ============================================================================================="
        );
        self.measure(true);
    }

    fn on_progress<F>(&mut self, p: F)
    where
        F: FnOnce() -> ProgressStatus,
    {
        let p = p();
        // Refresh the solver memory estimate from the counts batsat reports.
        //
        // Note `n_learnt_lits` is the *average* literals per learnt clause (not a
        // total), so multiply it by the learnt-clause count. We budget a little
        // above the current learnt count (`LEARNT_GROWTH_MARGIN`) to cover growth
        // between batsat's exponentially-spaced progress callbacks, but not all
        // the way to the `max_learnt` ceiling - solves rarely fill it, and doing
        // so over-estimates by 10-50x. H3 (`on_gc`) backstops the rest with the
        // real region size.
        let num_vars = p.dec_vars.max(0) as usize;
        let orig_clauses = p.n_clauses as usize;
        let orig_lits = p.n_clause_lits.max(0) as usize;
        let avg_learnt_lits = if p.n_learnt_lits.is_finite() && p.n_learnt_lits > 0.0 {
            p.n_learnt_lits
        } else {
            0.0
        };
        let learnt_clauses = (p.n_learnt as usize).saturating_mul(LEARNT_GROWTH_MARGIN);
        let learnt_lits = (learnt_clauses as f64 * avg_learnt_lits) as usize;
        let total_clauses = orig_clauses.saturating_add(learnt_clauses);
        let total_lits = orig_lits.saturating_add(learnt_lits);
        let solver_bytes = estimate_solver_bytes(num_vars, total_clauses, total_lits);
        {
            let mut inner = self.0.borrow_mut();
            inner.solver_bytes = solver_bytes;
            inner.measure(false);
        }
        let current = self.0.borrow().current_usage;
        trace!( target: "SCP",
            "c | {:9} | {:7} {:8} {:8} | {:8} {:8} {:6.0} | {:8} {:13} |",
            p.conflicts,
            p.dec_vars,
            p.n_clauses,
            p.n_clause_lits,
            p.max_learnt,
            p.n_learnt,
            p.n_learnt_lits,
            current.time.as_millis(),
            current.mem_bytes
        );
    }

    fn on_gc(&mut self, old: usize, new: usize) {
        trace!( target: "SCP",
            "|  Garbage collection:   {:12} bytes => {:12} bytes             |",
            old, new
        );
        // `old` is the real clause-region size (live + wasted words) at the
        // moment GC triggers - ground truth that the count-based estimate
        // misses, because `reduce_db` leaves freed clauses as "wasted" space the
        // `RegionAllocator` never shrinks. Fold it in (with slack for the backing
        // `Vec`'s doubling capacity) so `peak_mem_bytes` reflects reality. This
        // only ever raises the estimate; enforcement still uses current usage.
        let mut inner = self.0.borrow_mut();
        inner.solver_bytes = inner
            .solver_bytes
            .max(old.saturating_mul(REGION_CAPACITY_SLACK));
        inner.measure(false);
    }

    fn stop(&self) -> bool {
        self.measure_and_enforce_limits().is_err()
    }
}
