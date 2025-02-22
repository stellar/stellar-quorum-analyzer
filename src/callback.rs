use crate::allocator::get_memory_usage;
use batsat::{
    callbacks::{Callbacks, ProgressStatus},
    lbool,
};
use log::trace;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
    usize,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Resource {
    pub time: Duration,
    pub mem_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct ResourceUsage(Rc<RefCell<Resource>>);

impl ResourceUsage {
    pub fn new() -> Self {
        ResourceUsage(Rc::new(RefCell::new(Resource {
            time: Duration::ZERO,
            mem_bytes: 0,
        })))
    }

    pub fn get_time(&self) -> Duration {
        self.0.borrow().time
    }

    pub fn get_mem_bytes(&self) -> usize {
        self.0.borrow().mem_bytes
    }
}

/// An implementation of the `Callbacks` trait that tracks and limits the memory usage and processing time of the solver.
pub struct ResourceLimitingCB {
    start_time: Instant,
    start_memory: usize,
    time_limit: Duration,
    memory_limit: usize,
    current_usage: ResourceUsage,
}

impl ResourceLimitingCB {
    pub fn new(time_limit_ms: u64, memory_limit_bytes: usize, usage: ResourceUsage) -> Self {
        Self {
            start_time: Instant::now(),
            start_memory: get_memory_usage(),
            time_limit: Duration::from_millis(time_limit_ms),
            memory_limit: memory_limit_bytes,
            current_usage: usage,
        }
    }

    fn measure_resources(&self) {
        *self.current_usage.0.borrow_mut() = Resource {
            time: self.start_time.elapsed(),
            mem_bytes: get_memory_usage()
                .checked_sub(self.start_memory)
                .unwrap_or(usize::MAX),
        };
    }
}

impl Callbacks for ResourceLimitingCB {
    fn on_start(&mut self) {
        trace!( target: "SCP",
            "c ============================[ Search Statistics ]=============================="
        );
        trace!( target: "SCP",
            "c | Conflicts |          ORIGINAL         |          LEARNT          | Progress |"
        );
        trace!( target: "SCP",
            "c |           |    Vars  Clauses Literals |    Limit  Clauses Lit/Cl |          |"
        );
        trace!( target: "SCP",
            "c ==============================================================================="
        );
    }

    fn on_result(&mut self, _: lbool) {
        self.measure_resources();

        trace!( target: "SCP",
            "c ==============================================================================="
        );
        trace!( target: "SCP",
            "Solver completed -- Total time: {} ms, memory usage: {} bytes,",
            self.current_usage.get_time().as_millis(), self.current_usage.get_mem_bytes()
        );
    }

    fn on_progress<F>(&mut self, p: F)
    where
        F: FnOnce() -> ProgressStatus,
    {
        let p = p();
        trace!( target: "SCP",
            "c | {:9} | {:7} {:8} {:8} | {:8} {:8} {:6.0} | {:6.3} % |",
            p.conflicts,
            p.dec_vars,
            p.n_clauses,
            p.n_clause_lits,
            p.max_learnt,
            p.n_learnt,
            p.n_learnt_lits,
            p.progress_estimate
        );
    }

    fn on_gc(&mut self, old: usize, new: usize) {
        trace!( target: "SCP",
            "|  Garbage collection:   {:12} bytes => {:12} bytes             |",
            old, new
        );
    }

    fn stop(&self) -> bool {
        self.measure_resources();
        let (time_elapsed, bytes_used) = (
            self.current_usage.get_time(),
            self.current_usage.get_mem_bytes(),
        );
        let stop = time_elapsed > self.time_limit || bytes_used > self.memory_limit;
        if stop {
            trace!( target: "SCP",
                "Stopped due to going over the resource limits -- Time elapsed: {} ms, Time limit: {} ms; Memory usage: {} bytes, Memory limit: {} bytes",
                time_elapsed.as_millis(), self.time_limit.as_millis(), bytes_used, self.memory_limit
            );
        }
        stop
    }
}
