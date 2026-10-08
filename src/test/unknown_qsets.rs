use crate::fbas::{Fbas, InternalScpQuorumSet, QuorumSetMap};
use crate::xdr::{Limits, NodeId, PublicKey, ScpQuorumSet, Uint256, VecM, WriteXdr};
use crate::{FbasAnalyzer, FbasError, ResourceLimiter, SolveStatus};
use std::collections::BTreeSet;
use std::rc::Rc;

const DATA_DIR: &str = "./tests/test_data/missing_qsets/";

fn known_qset(threshold: u32, validators: &[&str]) -> Option<Rc<InternalScpQuorumSet>> {
    Some(Rc::new(InternalScpQuorumSet {
        threshold,
        validators: validators.iter().map(|v| (*v).to_string()).collect(),
        inner_sets: vec![],
    }))
}

fn qset_map(entries: Vec<(&str, Option<Rc<InternalScpQuorumSet>>)>) -> QuorumSetMap {
    entries
        .into_iter()
        .map(|(validator, qset)| (validator.to_string(), qset))
        .collect()
}

fn analyzer_from_map(qsm: QuorumSetMap) -> FbasAnalyzer {
    let limiter = ResourceLimiter::unlimited();
    let fbas = Fbas::from_quorum_set_map(qsm, &limiter).expect("build FBAS");
    FbasAnalyzer::from_fbas(fbas, limiter).expect("build analyzer")
}

fn validator_strings(fbas: &Fbas) -> BTreeSet<String> {
    fbas.validators
        .iter()
        .map(|v| fbas.try_get_validator_string(v).expect("validator string"))
        .collect()
}

fn split_sets(analyzer: &FbasAnalyzer) -> Vec<BTreeSet<String>> {
    let (quorum_a, quorum_b) = analyzer.get_potential_split().expect("split witness");
    vec![
        quorum_a.into_iter().collect(),
        quorum_b.into_iter().collect(),
    ]
}

#[test]
fn validator_with_unknown_qset_enables_split_and_appears_in_witness() {
    let mut analyzer = analyzer_from_map(qset_map(vec![
        ("A", known_qset(2, &["A", "X"])),
        ("B", known_qset(1, &["B"])),
    ]));

    let status = analyzer.solve().unwrap();
    let (quorum_a, quorum_b) = match status {
        SolveStatus::SAT(split) => split,
        other => panic!("expected SAT, got {other}"),
    };
    let validators_with_unknown_qsets = [
        analyzer
            .validator_strings_with_unknown_qsets(&quorum_a)
            .unwrap(),
        analyzer
            .validator_strings_with_unknown_qsets(&quorum_b)
            .unwrap(),
    ];
    assert!(validators_with_unknown_qsets.contains(&vec!["X".to_string()]));

    let split = split_sets(&analyzer);
    assert!(split.contains(&BTreeSet::from(["A".to_string(), "X".to_string()])));
    assert!(split.contains(&BTreeSet::from(["B".to_string()])));
    assert!(split[0].is_disjoint(&split[1]));
}

#[test]
fn validator_with_unknown_qset_is_unconstrained_and_satisfies_known_threshold() {
    let limiter = ResourceLimiter::unlimited();
    let fbas = Fbas::from_quorum_set_map(qset_map(vec![("A", known_qset(1, &["X"]))]), &limiter)
        .expect("build FBAS");

    let maximal = fbas.maximal_quorum(&limiter).expect("maximal quorum");
    let maximal_strings = maximal
        .iter()
        .map(|v| fbas.try_get_validator_string(v).unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        maximal_strings,
        BTreeSet::from(["A".to_string(), "X".to_string()])
    );

    let x = fbas
        .validators
        .iter()
        .find(|v| fbas.try_get_validator_string(v).unwrap() == "X")
        .copied()
        .unwrap();
    assert!(!fbas.validator_has_qset(x));
    assert_eq!(fbas.graph.neighbors(x).count(), 0);
}

#[test]
fn sets_containing_only_validators_with_unknown_qsets_cannot_anchor_quorums() {
    let mut no_known_qsets = analyzer_from_map(qset_map(vec![("X", None), ("Y", None)]));
    assert_eq!(no_known_qsets.solve().unwrap(), SolveStatus::NoQuorum);

    // Both SAT quorums would have to contain A because it is the only validator
    // with a known qset. Disjointness therefore makes the formula UNSAT; X
    // cannot anchor the second quorum.
    let mut one_known =
        analyzer_from_map(qset_map(vec![("A", known_qset(1, &["A"])), ("X", None)]));
    assert_eq!(one_known.solve().unwrap(), SolveStatus::UNSAT);
}

#[test]
fn repeated_references_to_validator_with_unknown_qset_share_one_vertex() {
    let limiter = ResourceLimiter::unlimited();
    let fbas = Fbas::from_quorum_set_map(
        qset_map(vec![
            ("A", known_qset(1, &["X", "X"])),
            ("B", known_qset(1, &["X"])),
        ]),
        &limiter,
    )
    .expect("build FBAS");

    assert_eq!(validator_strings(&fbas).len(), 3);
    assert_eq!(
        fbas.validators
            .iter()
            .filter(|v| fbas.try_get_validator_string(v).unwrap() == "X")
            .count(),
        1
    );

    let analyzer = FbasAnalyzer::from_fbas(fbas, limiter).expect("build analyzer");
    // Two SAT membership variables per graph node. The membership-map length
    // demonstrates that the validator with an unknown qset participates in
    // normal encoding.
    assert_eq!(analyzer.sat_formula_size_for_test().2, 4);
}

#[test]
fn top_level_known_qset_takes_precedence_over_reference_order() {
    let limiter = ResourceLimiter::unlimited();
    let fbas = Fbas::from_quorum_set_map(
        qset_map(vec![
            ("A", known_qset(1, &["X"])),
            ("X", known_qset(1, &["X"])),
        ]),
        &limiter,
    )
    .expect("build FBAS");

    assert_eq!(fbas.validators_with_qsets().count(), 2);
    assert!(fbas
        .validators
        .iter()
        .all(|validator| fbas.validator_has_qset(*validator)));
}

#[test]
fn resource_accounting_includes_referenced_validator_with_unknown_qset() {
    // One known validator and its lookup entry fit in this deliberately tiny
    // budget. Adding the referenced validator with an unknown qset and its
    // lookup entry does not, so construction must fail when X is preserved.
    let limiter = ResourceLimiter::new(u64::MAX, 200);
    let result = Fbas::from_quorum_set_map(qset_map(vec![("A", known_qset(1, &["X"]))]), &limiter);
    assert!(matches!(result, Err(FbasError::ResourcelimitExceeded(_))));
}

#[test]
fn impossible_known_qset_remains_no_quorum() {
    let mut analyzer = analyzer_from_map(qset_map(vec![("A", known_qset(2, &["A"]))]));
    assert_eq!(analyzer.solve().unwrap(), SolveStatus::NoQuorum);
}

#[test]
fn fixpoint_with_only_validators_with_unknown_qsets_is_no_quorum() {
    let mut analyzer = analyzer_from_map(qset_map(vec![("A", known_qset(2, &["A"])), ("X", None)]));
    assert_eq!(analyzer.solve().unwrap(), SolveStatus::NoQuorum);
    assert_eq!(analyzer.sat_formula_size_for_test(), (0, 0, 0));
}

fn node(id: u8) -> NodeId {
    NodeId(PublicKey::PublicKeyTypeEd25519(Uint256([id; 32])))
}

fn qset(threshold: u32, validators: Vec<NodeId>) -> ScpQuorumSet {
    ScpQuorumSet {
        threshold,
        validators: VecM::try_from(validators).unwrap(),
        inner_sets: VecM::default(),
    }
}

fn xdr<T: WriteXdr>(value: &T) -> Vec<u8> {
    value.to_xdr(Limits::none()).unwrap()
}

#[test]
fn empty_xdr_qset_preserves_validator_with_unknown_qset() {
    let limiter = ResourceLimiter::unlimited();
    let fbas = Fbas::from_quorum_set_map_buf(
        vec![xdr(&node(1))].into_iter(),
        vec![Vec::new()].into_iter(),
        &limiter,
    )
    .expect("build FBAS");

    assert_eq!(fbas.validators.len(), 1);
    assert_eq!(fbas.validators_with_qsets().count(), 0);
    assert_eq!(fbas.graph.neighbors(fbas.validators[0]).count(), 0);
}

#[test]
fn referenced_xdr_validator_with_unknown_qset_is_preserved_in_split() {
    let a = node(1);
    let b = node(2);
    let x = node(3);
    let nodes = vec![xdr(&a), xdr(&b)];
    let qsets = vec![
        xdr(&qset(2, vec![a.clone(), x.clone()])),
        xdr(&qset(1, vec![b.clone()])),
    ];
    let mut analyzer = FbasAnalyzer::from_quorum_set_map_buf(
        nodes.into_iter(),
        qsets.into_iter(),
        ResourceLimiter::unlimited(),
    )
    .expect("build analyzer");

    assert!(matches!(analyzer.solve().unwrap(), SolveStatus::SAT(_)));
    let split = split_sets(&analyzer);
    let x_string = stellar_strkey::ed25519::PublicKey([3; 32]).to_string();
    assert!(split.iter().any(|quorum| quorum.contains(&x_string)));
}

#[test]
fn python_missing_qset_fixtures_have_matching_split_results() {
    let expected = [
        ("python_missing_1.json", false),
        ("python_missing_2.json", true),
        ("python_missing_3.json", true),
    ];

    for (fixture, expected_split) in expected {
        let mut analyzer = FbasAnalyzer::from_json_path(
            &format!("{DATA_DIR}{fixture}"),
            ResourceLimiter::unlimited(),
        )
        .expect("load fixture");
        let status = analyzer.solve().expect("solve fixture");
        assert_eq!(
            matches!(status, SolveStatus::SAT(_)),
            expected_split,
            "unexpected split result for {fixture}: {status}"
        );

        if expected_split {
            let witness = split_sets(&analyzer);
            let witness = witness.into_iter().flatten().collect::<BTreeSet<_>>();
            let expected_validators_with_unknown_qsets = if fixture == "python_missing_2.json" {
                ["PK3", "PK4"].as_slice()
            } else {
                ["PKX", "PKY"].as_slice()
            };
            assert!(
                expected_validators_with_unknown_qsets
                    .iter()
                    .all(|validator| witness.contains(*validator)),
                "missing validator with an unknown quorum set in witness for {fixture}: {witness:?}"
            );
        }
    }
}
