use crate::{FbasAnalyzer, FbasError, ResourceLimiter, SolveStatus};
use std::{path::PathBuf, process::Command, u64};

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

fn solve_unlimited(json_file: &PathBuf) -> Result<SolveStatus, FbasError> {
    let mut solver = FbasAnalyzer::from_json_path(
        json_file.as_os_str().to_str().unwrap(),
        ResourceLimiter::unlimited(),
    )?;
    solver.solve()
}

fn solve_with_time_limit(
    json_file: &PathBuf,
    time_limit_ms: u64,
) -> Result<SolveStatus, FbasError> {
    let mut solver = FbasAnalyzer::from_json_path(
        json_file.as_os_str().to_str().unwrap(),
        ResourceLimiter::new(time_limit_ms, usize::MAX),
    )?;
    solver.solve()
}

// This is a test helper that overrides the global allocator's memory limit, and
// therefore must be run in an isolated process. It is ignored when running the
// regular test setup.
#[ignore]
#[test]
fn test_memory_limit_ps() {
    let args: Vec<String> = std::env::args().collect();
    // args[]: program name, test name, "--include-ignored", "--no-capture", memory_limit_bytes
    let memory_limit_bytes = if args.len() > 4 {
        args[4].parse::<usize>().unwrap()
    } else {
        usize::MAX
    };

    let json_file = std::path::PathBuf::from(
        "./tests/test_data/random/almost_symmetric_network_16_orgs_delete_prob_factor_3.json",
    );
    let mut solver = FbasAnalyzer::from_json_path(
        json_file.as_os_str().to_str().unwrap(),
        ResourceLimiter::new(u64::MAX, memory_limit_bytes),
    )
    .unwrap();
    solver.solve().unwrap();
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
    // first solve it without interruption, it should return normally
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
    assert_eq!(output.status.code(), Some(0));

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
            "1000", // memory_limit_bytes
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
