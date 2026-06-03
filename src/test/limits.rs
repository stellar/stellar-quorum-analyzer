use crate::{FbasAnalyzer, FbasError, ResourceLimiter, SolveStatus};
use std::path::{Path, PathBuf};

fn assert_solver_limit_exceeded(res: Result<SolveStatus, FbasError>) -> bool {
    match res {
        Ok(_) => false,
        Err(e) => matches!(e, FbasError::ResourcelimitExceeded(_)),
    }
}

fn solve_unlimited(json_file: &Path) -> Result<SolveStatus, FbasError> {
    let mut solver = FbasAnalyzer::from_json_path(
        json_file.as_os_str().to_str().unwrap(),
        ResourceLimiter::unlimited(),
    )?;
    solver.solve()
}

fn solve_with_time_limit(json_file: &Path, time_limit_ms: u64) -> Result<SolveStatus, FbasError> {
    let mut solver = FbasAnalyzer::from_json_path(
        json_file.as_os_str().to_str().unwrap(),
        ResourceLimiter::new(time_limit_ms, usize::MAX),
    )?;
    solver.solve()
}

fn solve_with_mem_limit(
    json_file: &Path,
    mem_limit_bytes: usize,
) -> Result<SolveStatus, FbasError> {
    let mut solver = FbasAnalyzer::from_json_path(
        json_file.as_os_str().to_str().unwrap(),
        ResourceLimiter::new(u64::MAX, mem_limit_bytes),
    )?;
    solver.solve()
}

#[test]
fn test_time_limit() -> Result<(), Box<dyn std::error::Error>> {
    let json_file = PathBuf::from(
        "./tests/test_data/random/almost_symmetric_network_16_orgs_delete_prob_factor_3.json",
    );
    // first solve it without interruption, it should return `UNSAT`
    assert_eq!(solve_unlimited(&json_file)?, SolveStatus::UNSAT);

    // reaching time limit, it should fail gracefully with an FbasError
    assert_solver_limit_exceeded(solve_with_time_limit(&json_file, 1));
    Ok(())
}

#[test]
fn test_memory_limit() -> Result<(), Box<dyn std::error::Error>> {
    let json_file = PathBuf::from(
        "./tests/test_data/random/almost_symmetric_network_16_orgs_delete_prob_factor_3.json",
    );
    // With a generous memory limit it should solve normally to UNSAT.
    assert_eq!(
        solve_with_mem_limit(&json_file, usize::MAX)?,
        SolveStatus::UNSAT
    );

    // With a tight memory limit the estimated usage is exceeded during
    // construction/solving, and it should fail gracefully with an FbasError
    // rather than aborting the process.
    assert!(assert_solver_limit_exceeded(solve_with_mem_limit(
        &json_file, 1000
    )));
    Ok(())
}

#[test]
fn test_wide_qset_relaxed() {
    use super::generators::{build_analyzer, gen_network_with_threshold};
    // A single quorum set of 40 validators with threshold 20 implies
    // C(40, 20) ~= 1.4e11 Tseitin combinations -- far past MAX_QSET_COMBINATIONS.
    // Rather than aborting (multi-terabyte `Vec::with_capacity`) or erroring, the
    // analyzer relaxes the over-wide qset to an empty quorum set and still
    // produces a sound result. The fact that this returns is the assertion that
    // no abort happened.
    let mut analyzer = build_analyzer(
        gen_network_with_threshold(40, 20),
        ResourceLimiter::new(u64::MAX, usize::MAX),
    )
    .expect("wide qset should be relaxed, not error");
    // Relaxing frees the qset, so a (sound, possibly-spurious) split exists.
    assert!(matches!(analyzer.solve().unwrap(), SolveStatus::SAT(_)));

    // Even under a tight memory limit the relaxed formula is tiny, so an even
    // wider qset still completes gracefully instead of aborting or erroring.
    let mut analyzer = build_analyzer(
        gen_network_with_threshold(64, 32),
        ResourceLimiter::new(u64::MAX, 50 * 1024 * 1024),
    )
    .expect("relaxed wide qset should fit a tight limit");
    assert!(analyzer.solve().is_ok());
}

#[test]
fn test_large_network_completes() {
    use super::generators::{build_analyzer, gen_symmetric_network};
    // A symmetric 2/3-threshold network has quorum intersection, so it solves to
    // UNSAT. Confirms the generator produces valid, solvable input.
    let qsm = gen_symmetric_network(8, 3, 6, 2);
    let mut analyzer =
        build_analyzer(qsm, ResourceLimiter::new(u64::MAX, usize::MAX)).expect("should build");
    assert_eq!(analyzer.solve().unwrap(), SolveStatus::UNSAT);
}
