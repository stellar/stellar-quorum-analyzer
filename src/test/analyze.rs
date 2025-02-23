use crate::{FbasAnalyzer, FbasError, ResourceLimiter, SolveStatus};
use std::collections::BTreeMap;
use std::{io::BufRead, str::FromStr};

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
    let expected_results: BTreeMap<&str, SolveStatus> = BTreeMap::from([
        ("missing_1", SolveStatus::UNSAT),
        ("circular_2", SolveStatus::UNSAT),
        ("validators_broken_1", SolveStatus::UNSAT),
        ("circular_1", SolveStatus::UNSAT),
        ("top_tier", SolveStatus::UNSAT),
        (
            "conflicted_2",
            SolveStatus::SAT((vec![2.into(), 3.into()], vec![0.into(), 1.into()])),
        ),
        ("homedomain_test_1", SolveStatus::UNSAT),
        (
            "conflicted_3",
            SolveStatus::SAT((vec![1.into()], vec![0.into()])),
        ),
        (
            "conflicted",
            SolveStatus::SAT((vec![1.into(), 2.into()], vec![3.into(), 5.into()])),
        ),
    ]);

    for entry in std::fs::read_dir("./tests/test_data/")? {
        let path = entry?.path();
        if let Some(extension) = path.extension() {
            if extension == "json" {
                let case_name = path.file_stem().unwrap().to_str().unwrap();
                let expected = expected_results.get(case_name).expect(&format!(
                    "No expected result found for test case: {}",
                    case_name
                ));
                let mut solver = FbasAnalyzer::from_json_path(
                    path.as_os_str().to_str().unwrap(),
                    ResourceLimiter::unlimited(),
                )
                .unwrap();
                let res = solver.solve()?;

                match (&res, expected) {
                    (SolveStatus::SAT((qa, qb)), SolveStatus::SAT((exp_qa, exp_qb))) => {
                        assert_eq!(qa, exp_qa);
                        assert_eq!(qb, exp_qb);
                    }
                    (SolveStatus::UNSAT, SolveStatus::UNSAT) => {}
                    _ => panic!(
                        "Case {} failed: expected {:?}, got {:?}",
                        case_name, expected, res
                    ),
                }
            }
        }
    }
    Ok(())
}

#[test]
fn test_random_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut test_cases = vec![];
    let dir_path = std::ffi::OsString::from_str("./tests/test_data/random/").unwrap();
    for entry in std::fs::read_dir("./tests/test_data/random/")? {
        let path = entry?.path();
        if let Some(extension) = path.extension() {
            if extension == "dimacs" {
                let case = path.file_stem().unwrap().to_os_string();
                test_cases.push(case);
            }
        }
    }

    for case in test_cases {
        let mut json_file = dir_path.clone();
        json_file.push(case.clone());
        json_file.push(".json");

        let mut dimacs_file = dir_path.clone();
        dimacs_file.push(case.clone());
        dimacs_file.push(".dimacs");

        let mut solver = FbasAnalyzer::from_json_path(
            json_file.as_os_str().to_str().unwrap(),
            ResourceLimiter::unlimited(),
        )
        .unwrap();
        let res = solver.solve()?;
        {
            // Open and read the file line by line
            let file = std::fs::File::open(dimacs_file).expect("Failed to open the DIMACS file");
            let reader = std::io::BufReader::new(file);

            // Look for the result comment line
            let mut expected = false;
            for line in reader.lines() {
                let line = line.expect("Failed to read line");
                if line.starts_with("c") {
                    if line.contains("UNSATISFIABLE") {
                        expected = false;
                        break;
                    } else if line.contains("SATISFIABLE") {
                        expected = true;
                        let (qa, qb) = solver.get_potential_split().unwrap();
                        println!("quorum a: {:?}, quorum b: {:?}", qa, qb);
                        break;
                    }
                }
            }
            let is_sat = matches!(res, SolveStatus::SAT(_));
            assert_eq!(is_sat, expected);
        }
    }
    Ok(())
}
