use crate::{FbasAnalyzer, FbasError, ResourceLimiter, SolveStatus};
use std::{path::PathBuf, process::Command};

fn assert_solver_limit_exceeded(res: Result<SolveStatus, FbasError>) -> bool {
    match res {
        Ok(_) => false,
        Err(e) => {
            if let FbasError::ResourcelimitExceeded(_) = e {
                true
            } else {
                false
            }
        }
    }
}

fn wrapped_solve(
    json_file: &PathBuf,
    time_limit_ms: u64,
    memory_limit_bytes: usize,
) -> Result<SolveStatus, FbasError> {
    let mut solver = FbasAnalyzer::from_json_path(
        json_file.as_os_str().to_str().unwrap(),
        ResourceLimiter::new(time_limit_ms, memory_limit_bytes),
    )?;
    solver.solve()
}

// This is a helper test that will be run in its own process. It is expected to
// fail with memory allocation error therefore we don't include it in the
// regular test suite.
#[ignore]
#[test]
fn test_memory_limit_ps() {
    let json_file = std::path::PathBuf::from(
        "./tests/test_data/random/almost_symmetric_network_16_orgs_delete_prob_factor_3.json",
    );
    let _ = wrapped_solve(&json_file, 1000, 100000);
}

#[test]
fn test_time_limit() -> Result<(), Box<dyn std::error::Error>> {
    let json_file = PathBuf::from(
        "./tests/test_data/random/almost_symmetric_network_16_orgs_delete_prob_factor_3.json",
    );
    // first solve it without interruption, it should return `UNSAT`
    assert_eq!(
        wrapped_solve(&json_file, 1000, 100_000_000)?,
        SolveStatus::UNSAT
    );

    // reaching time limit, it should fail gracefully with an FbasError
    assert_solver_limit_exceeded(wrapped_solve(&json_file, 1, 10000000));
    Ok(())
}

#[test]
fn test_memory_limit() -> Result<(), Box<dyn std::error::Error>> {
    let json_file = PathBuf::from(
        "./tests/test_data/random/almost_symmetric_network_16_orgs_delete_prob_factor_3.json",
    );

    // first solve it without interruption, it should return `UNSAT`
    assert_eq!(
        wrapped_solve(&json_file, 1000, 100_000_000)?,
        SolveStatus::UNSAT
    );

    // Test memory limit - should abort due to LimitedAllocator
    let output = Command::new("cargo")
        .args([
            "test",
            "--package",
            "stellar-quorum-analyzer",
            "--lib",
            "--",
            "test_memory_limit_ps",
            "--include-ignored",
            "--nocapture",
        ])
        .output()?;

    assert_eq!(output.status.code(), Some(101));
    #[cfg(unix)]
    {
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("SIGABRT"));
    }
    Ok(())
}
