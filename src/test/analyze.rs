use crate::{FbasAnalyzer, FbasError, ResourceLimiter, SolveStatus};
use std::collections::BTreeMap;

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

#[test]
fn test_resource_limit() -> Result<(), Box<dyn std::error::Error>> {
    let json_file = std::path::PathBuf::from(
        "./tests/test_data/random/almost_symmetric_network_16_orgs_delete_prob_factor_3.json",
    );

    let wrapped_solve =
        |time_limit_ms: u64, memory_limit_bytes: usize| -> Result<SolveStatus, FbasError> {
            let mut solver = FbasAnalyzer::from_json_path(
                json_file.as_os_str().to_str().unwrap(),
                ResourceLimiter::new(time_limit_ms, memory_limit_bytes),
            )?;
            solver.solve()
        };
    // first solve it without interruption, it should return `UNSAT`
    assert_eq!(wrapped_solve(1000, 100_000_000)?, SolveStatus::UNSAT);

    // reaching time limit
    assert_solver_limit_exceeded(wrapped_solve(1, 10000000));

    // reaching memory limit
    assert_solver_limit_exceeded(wrapped_solve(1000, 100000));

    Ok(())
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
    let expected_results: BTreeMap<&str, bool> = BTreeMap::from([
        ("missing_1", false),
        ("circular_2", false),
        ("validators_broken_1", false),
        ("circular_1", false),
        ("top_tier", false),
        ("conflicted_2", true),
        ("homedomain_test_1", false),
        ("conflicted_3", true),
        ("conflicted", true),
    ]);

    for entry in std::fs::read_dir("./tests/test_data/")? {
        let path = entry?.path();
        if let Some(extension) = path.extension() {
            if extension == "json" {
                let case_name = path.file_stem().unwrap().to_str().unwrap();
                let expected_sat = expected_results.get(case_name).expect(&format!(
                    "No expected result found for test case: {}",
                    case_name
                ));

                let mut solver = FbasAnalyzer::from_json_path(
                    path.as_os_str().to_str().unwrap(),
                    ResourceLimiter::unlimited(),
                )?;

                let res = solver.solve()?;
                let actual_sat = matches!(res, SolveStatus::SAT(_));

                assert_eq!(
                    actual_sat, *expected_sat,
                    "Case {} failed: expected {}, got {}",
                    case_name, expected_sat, actual_sat
                );

                // Print the split if one was found
                if actual_sat {
                    let (qa, qb) = solver.get_potential_split()?;
                    println!("\nFound quorum split for {}:", case_name);
                    println!("Quorum A:");
                    for validator in &qa {
                        println!("  - {}", validator);
                    }
                    println!("\nQuorum B:");
                    for validator in &qb {
                        println!("  - {}", validator);
                    }
                    println!();
                }
            }
        }
    }
    Ok(())
}

#[test]
fn test_random_data() -> Result<(), Box<dyn std::error::Error>> {
    let expected_results: BTreeMap<&str, bool> = BTreeMap::from([
        ("almost_symmetric_network_10_orgs_", false),
        ("almost_symmetric_network_12_orgs_", false),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_1",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_10",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_11",
            true,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_2",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_3",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_4",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_5",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_6",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_7",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_8",
            false,
        ),
        (
            "almost_symmetric_network_12_orgs_delete_prob_factor_9",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_1",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_10",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_11",
            true,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_12",
            true,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_2",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_3",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_4",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_5",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_6",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_7",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_8",
            false,
        ),
        (
            "almost_symmetric_network_13_orgs_delete_prob_factor_9",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_1",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_10",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_11",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_12",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_13",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_2",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_3",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_4",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_5",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_6",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_7",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_8",
            false,
        ),
        (
            "almost_symmetric_network_14_orgs_delete_prob_factor_9",
            false,
        ),
        ("almost_symmetric_network_16_orgs_", false),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_1",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_10",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_11",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_12",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_13",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_14",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_15",
            true,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_2",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_3",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_4",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_5",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_6",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_7",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_8",
            false,
        ),
        (
            "almost_symmetric_network_16_orgs_delete_prob_factor_9",
            false,
        ),
        ("almost_symmetric_network_2_orgs_", false),
        (
            "almost_symmetric_network_5_orgs_delete_prob_factor_1",
            false,
        ),
        (
            "almost_symmetric_network_5_orgs_delete_prob_factor_2",
            false,
        ),
        (
            "almost_symmetric_network_5_orgs_delete_prob_factor_3",
            false,
        ),
        ("almost_symmetric_network_5_orgs_delete_prob_factor_4", true),
        (
            "almost_symmetric_network_6_orgs_delete_prob_factor_1",
            false,
        ),
        (
            "almost_symmetric_network_6_orgs_delete_prob_factor_2",
            false,
        ),
        (
            "almost_symmetric_network_6_orgs_delete_prob_factor_3",
            false,
        ),
        (
            "almost_symmetric_network_6_orgs_delete_prob_factor_4",
            false,
        ),
        (
            "almost_symmetric_network_6_orgs_delete_prob_factor_5",
            false,
        ),
        ("almost_symmetric_network_8_orgs_", false),
        (
            "almost_symmetric_network_8_orgs_delete_prob_factor_1",
            false,
        ),
        (
            "almost_symmetric_network_8_orgs_delete_prob_factor_2",
            false,
        ),
    ]);

    for entry in std::fs::read_dir("./tests/test_data/random/")? {
        let path = entry?.path();
        if let Some(extension) = path.extension() {
            if extension == "json" {
                let case_name = path.file_stem().unwrap().to_str().unwrap();
                let expected_sat = expected_results.get(case_name).expect(&format!(
                    "No expected result found for test case: {}",
                    case_name
                ));

                let mut solver = FbasAnalyzer::from_json_path(
                    path.as_os_str().to_str().unwrap(),
                    ResourceLimiter::unlimited(),
                )?;

                let res = solver.solve()?;
                let actual_sat = matches!(res, SolveStatus::SAT(_));

                assert_eq!(
                    actual_sat, *expected_sat,
                    "Case {} failed: expected {}, got {}",
                    case_name, expected_sat, actual_sat
                );

                // Print the split if one was found
                if actual_sat {
                    let (qa, qb) = solver.get_potential_split()?;
                    println!("\nFound quorum split for {}:", case_name);
                    println!("Quorum A:");
                    for validator in &qa {
                        println!("  - {}", validator);
                    }
                    println!("\nQuorum B:");
                    for validator in &qb {
                        println!("  - {}", validator);
                    }
                    println!();
                }
            }
        }
    }
    Ok(())
}
