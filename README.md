# FBAS Quorum Intersection Analyzer

A library for analyzing quorum intersection properties in Federated Byzantine Agreement Systems (FBAS), specifically designed for the Stellar Consensus Protocol (SCP).

## Overview

This analyzer uses a SAT solver to verify the quorum intersection property of FBAS networks. In SCP, the quorum intersection property (any two quorums must share at least one node) is crucial for network safety. The library can detect potential network splits by finding disjoint quorums if they exist.

## Use Cases

- **Primary**: Integrated with stellar-core for runtime quorum analysis during network operations (validator joins, departures, or configuration changes)
- **Secondary**: Experimental analysis of network configurations via JSON input

## Features

- SAT solver-based analysis of quorum intersection properties
- Support for XDR-serialized quorum set maps via buffer interface
- JSON-based quorum set map input (optional, requires `json` feature)

## Building and Testing

- `cargo build --release`

- `cargo build --features json`

- `cargo test`

Test cases can be found in the `tests/test_data` directory. The
`tests/test_data/random` directory contains randomly generated configurations
up to 16 organizations.

## Performance

Performance benchmarks comparing different SAT solvers are available in the `benches` directory. After evaluating multiple pure-Rust SAT solvers against the test cases, Batsat was chosen as the primary solver for its performance and features (e.g. async interrupt).

To run benchmarks:

- `cargo bench`


## Documentation

- `docs/method.md`: Contains detailed derivation of the methodology and SAT formulas used in the analyzer
- API documentation: `cargo doc --open`

## Usage

### As a Library

```rust
use stellar_quorum_analyzer::{FbasAnalyzer, ResourceLimiter};
// From XDR-serialized buffer
let limiter = ResourceLimiter::new(u64::MAX, usize::MAX);
let mut analyzer = FbasAnalyzer::from_quorum_set_map_buf(nodes, quorum_sets, limiter)?;
let result = analyzer.solve()?;
// From JSON (requires 'json' feature)
let limiter = ResourceLimiter::new(u64::MAX, usize::MAX);
let mut analyzer = FbasAnalyzer::from_json_path("quorum_map.json", limiter)?;
let result = analyzer.solve()?;
// Get potential split information
if let Ok((quorum_a, quorum_b)) = analyzer.get_potential_split() {
    // Process split information
}
```

## Input Formats

- **Buffer Interface**: Primary method for stellar-core integration, accepts XDR-serialized quorum maps
- **JSON**: Alternative input method for configuration testing (requires `json` feature)

### Missing quorum sets and over-approximation

The analyzer accepts incomplete quorum-set maps. A validator whose quorum set
is absent is retained in the FBAS and modeled as having no local quorum
requirement. It can participate in a candidate quorum and count toward a known
validator's threshold. Each candidate quorum must nevertheless contain at least
one validator whose quorum set is known. 

For the purpose of checking quorum intersection, this is a safe
overapproximation: an `UNSAT` result implies that quorum intersection
holds regardless of what the unknown quorum sets turn out to be.

An unknown quorum set can be expressed by:

- referencing a validator that has no top-level map entry;
- supplying an empty qset buffer through the XDR API; or
- omitting the qset field, or setting it to `null`, in either JSON format.

The analyzer logs a warning for each validator modeled this way. Before
returning a split, it emits another warning naming any validators with unknown
quorum sets in each candidate quorum. A `SAT` result still uses the existing
result variant and may therefore describe a potential, rather than confirmed,
split if its witness contains a validator whose quorum set is unknown. Learning
the missing qset could invalidate that witness. Likewise, the existing
wide-qset safeguard relaxes constraints whose exact encoding would require more
than one million combinations, which can also make `SAT` spurious. This release
does not distinguish exact and over-approximate `SAT` outcomes in the result
type.

`UNSAT` means no split exists even under this permissive model, subject to the
known-qset anchor rule. `NoQuorum` means no quorum containing a known-qset
validator exists even after unknown qsets are treated permissively. Missing
qsets alone do not cause `SolveStatus::UNKNOWN`; that status remains reserved
for an indeterminate solver run, normally due to resource limits.

## Future Work

- Command-line interface for JSON-based quorum analysis
