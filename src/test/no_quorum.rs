use crate::fbas::Fbas;
use crate::{FbasAnalyzer, ResourceLimiter, SolveStatus};
use std::collections::BTreeSet;

const DATA_DIR: &str = "./tests/test_data/no_quorum/";

#[test]
fn no_quorum_status_formats_and_differs_from_unsat() {
    let status = SolveStatus::NoQuorum;
    assert_eq!(format!("{status:?}"), "NoQuorum");
    assert_ne!(status, SolveStatus::UNSAT);
}

fn maximal_quorum_strings(path: &str) -> BTreeSet<String> {
    let rl = ResourceLimiter::unlimited();
    let fbas = Fbas::from_json_path(path, &rl).expect("load fbas");
    fbas.maximal_quorum(&rl)
        .expect("maximal quorum")
        .iter()
        .map(|ni| fbas.try_get_validator_string(ni).expect("validator string"))
        .collect()
}

#[test]
fn maximal_quorum_intertwined_is_all_validators() {
    let mq = maximal_quorum_strings(&format!("{DATA_DIR}intertwined.json"));
    assert_eq!(
        mq,
        BTreeSet::from(["A".to_string(), "B".to_string(), "C".to_string()])
    );
}

#[test]
fn maximal_quorum_empty_when_threshold_exceeds_degree() {
    let rl = ResourceLimiter::unlimited();
    let fbas = Fbas::from_json_path(
        &format!("{DATA_DIR}no_quorum_threshold_exceeds_degree.json"),
        &rl,
    )
    .expect("load fbas");
    assert!(fbas.maximal_quorum(&rl).expect("maximal quorum").is_empty());
}

#[test]
fn maximal_quorum_feasible_with_unknown_qset_is_all_validators() {
    let mq = maximal_quorum_strings(&format!("{DATA_DIR}feasible_split_with_unknown.json"));
    assert_eq!(
        mq,
        BTreeSet::from([
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
            "GHOST".to_string(),
        ])
    );
}

fn solve_status(path: &str) -> SolveStatus {
    let mut analyzer =
        FbasAnalyzer::from_json_path(path, ResourceLimiter::unlimited()).expect("load analyzer");
    analyzer.solve().expect("solve")
}

#[test]
fn solve_reports_no_quorum_when_threshold_exceeds_degree() {
    let status = solve_status(&format!(
        "{DATA_DIR}no_quorum_threshold_exceeds_degree.json"
    ));
    assert!(
        matches!(status, SolveStatus::NoQuorum),
        "expected NoQuorum, got {status}"
    );
}

#[test]
fn no_quorum_analyzer_skips_sat_formula_construction() {
    let analyzer = FbasAnalyzer::from_json_path(
        &format!("{DATA_DIR}no_quorum_threshold_exceeds_degree.json"),
        ResourceLimiter::unlimited(),
    )
    .expect("load analyzer");

    assert_eq!(analyzer.sat_formula_size_for_test(), (0, 0, 0));
}

#[test]
fn solve_reports_split_when_feasible() {
    let status = solve_status(&format!("{DATA_DIR}feasible_split_with_unknown.json"));
    assert!(
        matches!(status, SolveStatus::SAT(_)),
        "expected SAT (split), got {status}"
    );
}

#[test]
fn solve_reports_unsat_when_intertwined() {
    let status = solve_status(&format!("{DATA_DIR}intertwined.json"));
    assert!(
        matches!(status, SolveStatus::UNSAT),
        "expected UNSAT (intertwined), got {status}"
    );
}
