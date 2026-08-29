# FBAS Quorum Intersection Analysis

In this document, we explain how to create a propositional formula, in CNF form, which is satisfiable if and only if there are two disjoint quorums in a give FBAS. This formula can then be passed to a SAT solver to determine whether it is satisfiable or not.

The method is inspired by the [python-fbas prototype](https://github.com/nano-o/python-fbas), which allows checking properties of FBA systems using automated constraint solvers.

## Derivation

The general idea is to create a formula that asserts that there are two non-empty, disjoint quorums $A$ and $B$.

Our input is a FBAS graph, which is a directed graph whose set of nodes consists of a set of validators $v_1,\dots,v_N$ and a (disjoint) set of other nodes (which we call quorum-set nodes) $q_{N+1},\dots,q_M$. We may refer to a node in the graph $v_i$ or $q_i$ simply as node $i$. A validator with a known quorum set has one edge to its quorum-set node. A validator whose quorum set is unknown has no outgoing edge and is modeled without a local constraint. Quorum-set nodes have an integer threshold; the implementation also handles zero thresholds and thresholds greater than their degree.

We say that a node $j$ is a successor of a node $i$ when there is an edge in the graph from $i$ to $j$. Given a node $i$, each set of $t_i$ successors of $i$ is called a slice of $i$. A quorum $Q$ is a set of nodes such that, for every node $i\in Q$, $i$ has a slice which is a subset of $Q$ (i.e. at least $t_i$ successors of $i$ are in $Q$).

**Objective**: Prove or disprove that every two quorums have at least one validator in common.

**Challenge**: The number of quorums may be exponential in the number of nodes in the graph, yet we must either a) find two quorums that have no validator in common or b) demonstrate that every two quorums have a validator in common.

### Constraints

We consider two unspecified quorums $A$ and $B$ and create $2$ propositional variables $A_i$ and $B_i$ for every node $i$ in the graph (both validator nodes and quorum-set nodes), with the intent that $A_i$ is true when node $i$ is in $A$ and $B_i$ is true when $i$ is in $B$.

We now create a propositional formula in CNF that is satisfiable if and only if there are two quorums $A$ and $B$ that have no validator in common. If the formula is satisfiable, a SAT solver will give use an assignment of truth values to the variables (called a model) that will explicitly describe two disjoint quorums (but there may be many others).

Remember that a CNF formula is a conjunction of disjunctions of literals (where a literal is a variable or the negation of a variable).

#### 1. Both A and B contain at least one validator with a known qset

$$
\left(\bigvee_{i=1}^N  A_i\right)\wedge\left(\bigvee_{i=1}^N  B_i\right)
$$

Here the disjunction ranges only over validators whose quorum sets are known.
Validators whose qsets are unknown may participate in either quorum, but a set
containing only such validators cannot form a quorum. This known-qset-anchor
rule matches `python-fbas.find_disjoint_quorums`. The formula is already in CNF
form.

#### 2.  A and B have no validator in common

$$
\bigwedge_{i=1}^N \left(\neg A_i \vee \neg B_i \right)
$$

This is also in CNF form.

#### 3.  A and B are quorums

For each quorum $Q\in\{A,B\}$, for each constrained node $i$ in the graph (a
known-qset validator or a quorum-set node), if $i$ is in $Q$ then $i$ has a
slice in $Q$ (i.e. $t_i$ successors of $i$ are in the quorum). Unknown-qset
validator nodes are intentionally omitted from this constraint, so their
membership variables are locally unconstrained. Formally:

$$
\bigwedge_{i=1}^M \left( A_i \implies \Phi^A_i  \right) \wedge \left( B_i \implies \Phi^B_i \right)
$$

where $\Phi^A_i$ asserts that $t_i$ successors of $i$ are in the quorum.

Using $a \implies b \equiv \neg a \vee b$, the formula expands to:

$$
\bigwedge_{i=1}^M \left( \neg A_i \vee \Phi^A_i \right) \wedge \left( \neg B_i \vee \Phi^B_i \right)
$$

Next we define $\Phi_i^A$ and $\Phi_i^B$, and then convert to CNF. We only explain $\neg A_i \vee \Phi_i^A$ in detail, as $\neg B_i \vee \Phi_i^B$ is similar.

##### Expansion of $\Phi_i^A$

Let $\Pi_i=\{s^1_i,s^2_i,\dots\}$ be the set of all combinations of $t_i$ successors of node $i$ (i.e. the set of all slices of node $i$). Thus $\Pi_i$ has $\binom{\text{degree}(i)}{t_i}$ elements (this is a binomial coefficient \"$n$ choose $k$\" where $n=\text{degree}(i)$ and $n=t_i$). Given $\Pi_i$, we can define $\Phi_i^A$ as:

$$
\Phi_i^A \equiv \bigvee_{j=1}^{|\Pi_i|} \bigwedge_{k\in s_i^j} A_k
$$

Here $j$ ranges over all the subsets in $\Pi_i$ and $k$ ranges over elements within each subset. This is saying that at least one slice must have all its members in the quorum.

However, this is in DNF form. To convert to CNF, we apply the Tseitin transformation.

##### Applying Tseitin Transformation

Introduce a new variable $\alpha^i_j$ for each inner AND:

$$
\alpha_i^j \leftrightarrow \bigwedge_{k\in s_i^j} A_k
$$

Now we can already write:

$$
\Phi_i^A = \bigvee_{j=1}^{|\Pi_i|} \alpha_i^j
$$

Finally, we must assert that:

$$
\alpha_i^j \leftrightarrow \bigwedge_{k\in s_i^j} A_k
$$

We apply the equivalence $a \leftrightarrow b \equiv \left(a \implies b\right) \bigwedge \left( b \implies a\right)$, further expand it by applying the distributive laws, and we get the following CNF formula:

$$
 \left(\bigwedge_{k\in s_i^j} \left( \neg \alpha_i^j \vee A_k \right) \right) \wedge \left( \alpha_i^j \vee \bigvee_{k\in s_i^j}\neg A_k \right)
$$

Overall, $\neg A_i \vee \Phi^A_i$ becomes the CNF:

$$
\left(\neg A_i \vee \bigvee_{j=1}^{|\Pi_i|} \alpha_i^j\right)\wedge \bigwedge_{j=1}^{|\Pi_i|}\left(\bigwedge_{k\in s_i^j} \left( \neg \alpha_i^j \vee A_k \right) \right) \wedge \left( \alpha_i^j \vee  \bigvee_{k\in s_i^j}\neg A_k \right)
$$

Similarly for $B$, we introduce variables $\beta_i^j$, and $\neg B_i \vee \Phi^B_i$ becomes the CNF:

$$
\left(\neg B_i \vee \bigvee_{j=1}^{|\Pi_i|} \beta_i^j\right)\wedge \bigwedge_{j=1}^{|\Pi_i|}\left(\bigwedge_{k\in s_i^j} \left( \neg \beta_i^j \vee B_k  \right) \right) \wedge \left( \beta_i^j \vee  \bigvee_{k\in s_i^j}\neg B_k \right)
$$

#####  Final formula encoding that A and B are quorums

To encode that $A$ is a quorum, we just conjoin the quorum conditions above for each $i$ from $1$ to $M$ and obtain:

$$
\bigwedge_{i=1}^M \left(\left(\neg A_i \vee \bigvee_{j=1}^{|\Pi_i|} \alpha_i^j\right)\wedge \bigwedge_{j=1}^{|\Pi_i|}\left(\bigwedge_{k\in s_i^j} \left( \neg \alpha_i^j \vee A_k \right) \right)  \wedge \left( \alpha_i^j \vee \bigvee_{k\in s_i^j}\neg A_k \right)\right)
$$

Similarly for $B$:

$$
\bigwedge_{i=1}^M \left(\left(\neg B_i \vee \bigvee_{j=1}^{|\Pi_i|} \beta_i^j\right)\wedge \bigwedge_{j=1}^{|\Pi_i|}\left(\bigwedge_{k\in s_i^j} \left( \neg \beta_i^j \vee B_k \right) \right)  \wedge \left( \beta_i^j \vee \bigvee_{k\in s_i^j}\neg B_k \right)\right)
$$

## Quorum existence and the `NoQuorum` result

The CNF above is satisfiable iff two non-empty, disjoint quorums exist. A plain
`UNSAT` therefore answers "there are no two disjoint quorums" — but that single
answer collapses two opposite network states:

1. **Intertwined (healthy):** quorums exist and every pair of quorums
   intersects.
2. **Degenerate:** no quorum containing a validator with a known quorum set
   exists at all (for example, every known validator's threshold exceeds the
   number of members in its qset). With no anchored quorum to find, the formula
   is *vacuously* unsatisfiable.

To avoid reporting a degenerate, possibly-halted network as "safe", the analyzer
runs a quorum-existence pre-pass before the SAT solve. It computes the **maximal
quorum** — the union of all quorums — by a greatest-fixpoint contraction: begin
with every validator, then repeatedly drop any validator whose quorum set is not
satisfied by the validators that remain, until the set is stable. Because
Validators with unknown qsets are always locally satisfied during contraction
and can satisfy known thresholds. Quorums are closed under union, so the
fixpoint contains every validator belonging to some modeled quorum. If the
fixpoint contains no known-qset validator, it is reported as empty to preserve
the SAT encoding's known-qset-anchor rule.

- Maximal quorum **empty** → `SolveStatus::NoQuorum` (degenerate / possible
  halt); the SAT solve is skipped.
- Maximal quorum **non-empty** → run the disjoint-quorum solve as above, giving
  `SAT` (a split) or `UNSAT` (intertwined).

This mirrors stellar-core's legacy C++ quorum-intersection checker, which
detects the same "no quorums found" condition via the same greatest-fixpoint
contraction (`QuorumIntersectionCheckerImpl::contractToMaximalQuorum`) and warns
about a possible network halt.

## Approximation and result interpretation

Missing quorum sets are treated as having no local requirement. Within the
known-qset-anchor policy described above, this relaxes local constraints and may
add a split that disappears once the missing qsets are known. The implementation
also skips an individual qset constraint when its DNF expansion would exceed one
million combinations. That wide-qset safeguard is another relaxation with the
same one-sided effect.

Both cases currently return ordinary `SolveStatus::SAT`; there is no separate
result variant recording approximation provenance. A `SAT` witness that uses
validators whose qsets are unknown, or depends on a relaxed wide qset, may
therefore be spurious. `UNSAT` remains evidence that no split exists even in the
permissive model. `NoQuorum` means the maximal permissive fixpoint contains no
validator with a known qset. `UNKNOWN` is reserved for an indeterminate solver
run and is not returned merely because input qsets are missing.
