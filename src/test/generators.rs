//! Programmatic FBAS input generators for stress / adversarial testing.
//!
//! These build a `QuorumSetMap` directly and run it through the real pipeline
//! (`Fbas::from_quorum_set_map` -> `FbasAnalyzer::from_fbas`), so tests can
//! exercise large / pathological networks without committing huge JSON fixtures.

use crate::{
    fbas::{Fbas, InternalScpQuorumSet, QuorumSetMap},
    FbasAnalyzer, FbasError, ResourceLimiter,
};
use std::rc::Rc;

/// A single quorum set shared by `nodes` validators, listing all `nodes`
/// of them with specified `threshold`.
/// The shared qset deduplicates to one graph vertex with `validators`
/// successors and `threshold` threshold, so `construct_formula` enumerates
/// `C(validators, threshold)` combinations - the combinatorial "bomb".
/// Pick `threshold ≈ validators/2` for the worst case.
pub(crate) fn gen_network_with_threshold(nodes: usize, threshold: u32) -> QuorumSetMap {
    let names: Vec<String> = (0..nodes).map(|i| format!("v{i}")).collect();
    let qset = Rc::new(InternalScpQuorumSet {
        threshold,
        validators: names.clone(),
        inner_sets: vec![],
    });
    let mut map = QuorumSetMap::new();
    for name in names {
        map.insert(name, Some(qset.clone()));
    }
    map
}

/// A symmetric network of `orgs` organizations, each with `nodes_per_org`
/// validators. Every node shares one top quorum set whose threshold is
/// `org_threshold` over one inner set per org; each inner set has threshold
/// `node_threshold` over that org's validators. Mirrors the shape of the
/// `almost_symmetric_network_*` fixtures and forces real solver search.
pub(crate) fn gen_symmetric_network(
    orgs: usize,
    nodes_per_org: usize,
    org_threshold: u32,
    node_threshold: u32,
) -> QuorumSetMap {
    let node_name = |o: usize, n: usize| format!("org{o}v{n}");
    let inner_sets: Vec<InternalScpQuorumSet> = (0..orgs)
        .map(|o| InternalScpQuorumSet {
            threshold: node_threshold,
            validators: (0..nodes_per_org).map(|n| node_name(o, n)).collect(),
            inner_sets: vec![],
        })
        .collect();
    let top = Rc::new(InternalScpQuorumSet {
        threshold: org_threshold,
        validators: vec![],
        inner_sets,
    });
    let mut map = QuorumSetMap::new();
    for o in 0..orgs {
        for n in 0..nodes_per_org {
            map.insert(node_name(o, n), Some(top.clone()));
        }
    }
    map
}

/// Build an analyzer from a generated map through the real pipeline.
pub(crate) fn build_analyzer(
    qsm: QuorumSetMap,
    limiter: ResourceLimiter,
) -> Result<FbasAnalyzer, FbasError> {
    let fbas = Fbas::from_quorum_set_map(qsm, &limiter)?;
    FbasAnalyzer::from_fbas(fbas, limiter)
}
