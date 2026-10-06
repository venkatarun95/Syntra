# CBR-delay CCA development notes

This document is for agents changing the CCA experiment.  The user-facing
policy/design surface is [`src/cbrdelay_cca_config.rs`](src/cbrdelay_cca_config.rs).
Keep it small and directly editable: changes to CCA policy should normally be
made there, rather than folded into the generic minimax implementation.

## What this binary is

`cbrdelay-cca` is a CCA-only minimax experiment for the existing CBR-delay
network model.  It is deliberately separate from the video/ABR code.

* Binary wrapper: `src/cbrdelay_cca.rs`
* CCA runner and policy/action implementation: `src/cong_ctrl.rs`
* Editable experiment configuration: `src/cbrdelay_cca_config.rs`
* Generic minimax search: `src/lib.rs`
* CBR-delay network model: `../network_model_nc/`
* Generated QE predicate consumed by that model:
  `../network_model_nc_qe_output/src/lib.rs`

The wrapper includes `cong_ctrl.rs`, so it is also compiled as the legacy `cc`
binary.  Do not put CBR-specific policy choices in the generic minimax crate
root unless they truly apply to every model.

## Intended editable controls

The configuration file owns these choices:

| Control | Meaning |
| --- | --- |
| `APP_IS_BACKLOGGED` | `true` means the source always supplies whatever the CCA requests. |
| `APP_SEND_CHOICES` | When not backlogged, the adversary chooses Full, Half, or Zero application supply each control interval. |
| `SEARCH_DEPTH` | Even minimax ply count: a CCA action followed by an adversary action is one round.  Keep the default modest. |
| `CandidateAction` / `CANDIDATE_ACTIONS` | The finite, editable CCA action space. |
| `OBJECTIVE_ORDER` | Lexicographic metric order, from most to least important. |
| `state_policy` | Per-committed-state hook for QE-history length, tolerances, and objective order. |

The non-backlogged option is an adversarial finite approximation; it does *not*
require a different QE model.  For each feasible network response the
adversary can also select the configured application-supply fraction.  The
fixed simulation path still uses a full (backlogged) source, so do not claim it
simulates an adversarial application trace.

## Long QE histories and per-state policy

Generated QE is only practical for six transitions (seven observations).  The
CCA therefore never sends a longer formula to QE.  Set
`StatePolicy::belief_history_observations` to any value at least
`QE_WINDOW_OBSERVATIONS`; the engine evaluates every overlapping QE-sized
window ending in that history and intersects each returned belief bound.  For a
history ending at `t`, this is `[t-6,t]`, `[t-7,t-1]`, and so on.

Edit `state_policy(&StateQuantities)` in `cbrdelay_cca_config.rs` to select the
history length, `loss_tolerance_abs`, `delay_tolerance_frac`, and
`objective_order`.  Its input exposes the full oldest-to-newest observation
history, timestep, and preceding belief.  The resulting policy is frozen for
the entire minimax tree and is selected again only after the next real network
observation is committed.

## Objectives and new metrics

`OBJECTIVE_ORDER` names existing fields calculated in `CongCtrlState::compute_metrics`.
The comments next to `OBJECTIVE_ORDER` enumerate the currently supported keys
and their direction of preference.  The important distinction is:

* Reordering or selecting existing keys is a policy change and belongs in
  `cbrdelay_cca_config.rs`.
* Inventing a new notion of quality requires a new field in
  `cc_common::Metrics`, computing it in `cong_ctrl.rs`, and adding its
  lexicographic comparison in `CCValue`.

Several `tt*` metrics use the end of the speculative horizon as their target.
They therefore measure progress within the current look-ahead, not an
independent convergence guarantee.

`ld` minimizes only loss exceeding `loss_tolerance_abs`; use the new `l`
objective when every observed loss should be minimized, including loss within
that tolerance.

`--obj-perm` is useful for temporary experiments: it prepends CLI-supplied
metric names to the configured order.  Validate any new name against the
comparison code before relying on it.

## Running and debugging

From the `synthesizer` directory:

```bash
cargo run -p minimax --bin cbrdelay-cca -- \
  --sim --sim-c 20 --sim-b 10 --steps 4 --csv \
  --out outputs/cbrdelay-cca
```

This runs actual minimax against a concrete CBR-delay simulation and writes a
CSV below the selected output directory.  `--sim-max-rate` bypasses minimax and
uses a simple baseline strategy; it is useful as a smoke test but not for
evaluating a candidate CCA.

Progress logging is intentionally controlled by `SYNTRA_LOG`, rather than an
ambient `RUST_LOG`, so a shell-level `RUST_LOG=warn` cannot make the program
appear stuck.  Use `SYNTRA_LOG=debug` or `SYNTRA_LOG=trace` for more detail.

The search grows quickly with both action count and depth.  Start around four
control intervals and an even depth of four.  If the program fails before
printing a step result, reproduce with `--steps 1` before making search-wide
changes.

## QE/transpilation boundary

The runner evaluates the generated Rust QE implementation in
`network_model_nc_qe_output`; it does not transpile QE at CCA runtime.  When
regenerating a CBR-delay QE result, use the QE driver explicitly, validate that
the resulting dispatch tables are complete, then deliberately decide whether
to replace the active generated evaluator.  Do not silently copy a partially
generated `qe/outputs/.../lib.rs` into the active crate.

The repository may contain large, user-generated QE `.smt2`, `.z3`, output,
and `target/` artifacts.  They are not source changes and should not be staged
or deleted as part of CCA work unless the user explicitly requests it.

## Known correctness guardrails

* `SEARCH_DEPTH` must be even.
* The generic minimax root must be inserted regardless of the selected search
  depth.  The old `SPECULATION_SIZE` special case was removed because it made
  shallow searches unwrap an empty root result.
* Time-to metrics operate at the initial history pointer too; use a signed
  time representation or `saturating_sub`, never `usize - 1`.
* Avoid stateful exact-rational updates in long simulation runs unless their
  denominator growth is bounded.  A previous loss-fraction update overflowed
  after many steps.
