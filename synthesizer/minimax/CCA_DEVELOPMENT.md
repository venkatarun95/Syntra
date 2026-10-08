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
| `CandidateAction` | The finite, editable CCA action space; the planner picks the subset minimax may use each RTT (`StatePolicy::candidate_actions`). |
| `OBJECTIVE_ORDER` | Lexicographic metric order, from most to least important. |
| `state_policy` | Per-committed-state planner: mode, candidate actions, QE-history length, tolerances, and objective order. |
| `K_POLICY`, `K_CONFIRM_RTTS` | Bursty model: fixed K, or K hypotheses tracked by `update_k`. |
| Planner constants | `C_TIGHT_RATIO`, `SMALL_BUFFER_BDP`, `PROBE_INTERVAL_RTTS`, `DRAIN_INTERVAL_RTTS`, loss budgets; see "Planner". |

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

Committed windows' bounds are cached (per K set), so a long history costs
one QE evaluation per new window rather than per window per tree node.

Edit `state_policy(&StateQuantities, &mut PlannerState)` in `cbrdelay_cca_config.rs` to select the
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

## QE models: plain and bursty

`QE_MODEL` in `cbrdelay_cca_config.rs` (or `--qe-model plain|bursty`)
selects which generated QE evaluator the unchanged `NetworkModelNC` uses:

* `plain`: the original CBR-delay link, `network_model_nc_qe_output`.
* `bursty`: CBR-delay behind a sender whose arrivals may be perturbed by up to
  `K` bytes, `network_model_bursty_qe_output`.  QE leaves `K` and `pert` free
  (they must satisfy `K < pert * B`); `pert` is `BURSTY_PERT` and K comes
  from `K_POLICY` (see "Burstiness hypotheses").  Bursty output files get a
  `_bursty_k…_pert…_simk…` suffix.

`network_model_nc/src/qe_tables.rs` hides the differences behind `QeTables`.
Its lookups return `None` for an unsolved (T, losses) key instead of
panicking, which is what makes partial QE results usable.

### Unfinished QE queries for C

The bursty C query did not finish at T=6, and at T=5 only one loss count
finished.  When the C lookup for a window is missing, `NetworkModelNC::compute_c`
intersects every constraint on C that *was* solved:

1. C from shorter sub-windows, largest T first (T=5, then T=4, ...), over every
   sub-window whose loss count has a result.
2. `c_from_b`: the clauses of this window's (solved) B query that constrain C
   without mentioning B.  The transpiler emits these as `COMPUTE_C_FROM_B`.
3. Numerical elimination of B: C is feasible for the window iff its B interval
   is non-empty, so `tighten_c_with_b` bisects for that range on a 1/16 grid.
   Like `my_min`/`my_max`, it assumes the feasible C set is one interval.

Step 3 is what the unfinished QE query would have done symbolically; for
concrete observations it is a 1-D search.  The test
`test_c_tightened_with_b_matches_exact` hides the plain model's T=6 C and
checks that this recovers the exact T=6 C to within the grid, even with no
sub-window results at all.  Steps 2-3 also keep every C passed to the T=6
B/Q/S/L functions feasible; those functions `assert!` that C is feasible.

Regenerate the bursty crate from all bursty results with
`sh qe/scripts/transpile_bursty.sh` (see the `qe` README).

## Planner (long-horizon modes)

Minimax only looks `SEARCH_DEPTH / 2` RTTs ahead, so decisions on longer
time scales are made by a planner in `cbrdelay_cca_config.rs`.  The engine
owns its memory (`PlannerState`) and calls, once per committed RTT:

1. `update_k` (bursty model): which burstiness hypotheses are still alive,
   and the K set the belief uses.
2. `state_policy`: picks a `Mode` and, from it, the candidate actions,
   loss budget and objectives for this RTT's search.  With one candidate
   the search is skipped.
3. `record_action`, after the move is chosen (probe bookkeeping).

Modes: `Explore` (C loose; minimax picks among `MaxAllowedRate`,
`DrainQueue`, `ProbeAboveLimit`, or only `DrainQueue` once the queue may
exceed `EXPLORE_MAX_QUEUE_BDP`), `Cruise` (C tight; `HoldQueue`, i.e.
`min_c` less any queue above `DRAIN_MAX_Q + K`, the smallest queue bursts
let the belief prove), `Probe` (C tight and a probe is due) and `Drain`
(every `DRAIN_INTERVAL_RTTS`, held until the belief proves
`max_q <= DRAIN_MAX_Q + K` or for at most `MAX_DRAIN_RTTS`, so the min-RTT
estimate is refreshed).  C counts as tight once
`max_c <= C_TIGHT_RATIO * min_c + C_TIGHT_PER_K * K`: bursts blur C, so a
fixed ratio is unreachable for large K.  All constants are experiment
starting points; `--probe-interval`, `--drain-interval` and
`--belief-history` override three of them.

`--belief-history N` intersects only the newest N observations' windows.  In
60-RTT sweeps the whole history (the default) was clearly better than N=10:
a short history forgets B and C, re-probes and loses 4-7x more packets.

### Burstiness hypotheses

With `KPolicy::Hypotheses`, each K is tested on every new QE window with
`NetworkModelNC::window_feasible` (a K is ruled out when no C on the search
grid leaves B non-empty).  Feasibility is not monotone in K: bursts rule out
small K, and a small buffer rules out large K (`B > K / pert`).  So the
belief uses `UnionBurstyQe`, the union of the per-K QE bounds over the
assumed set: every alive K, until the smallest alive K has survived
`K_CONFIRM_RTTS` without any K being ruled out, then that K alone.  A
generated function asserts that its (C, B) inputs are feasible for its K;
the union skips a K whose assertion fails (see `quietly_catch`).

### Simulation policies

The simulated link has its own K (`--sim-k`, via `with_sim_qe`), so the
CCA's hypotheses do not change it.  K only bounds what the link may do;
`--sim-policy` decides what it does: `max`/`min` service (never/always
withhold bursts), `random` (seeded by `--seed`), `adversary` (replay the
reply minimax expected, which is feasible for the CCA's belief rather than
the true C/B) or `mixed` (switch among those every `--mixed-period` RTTs).

### Application-limited senders

`--app` simulates an application the CCA does not control: `backlogged`,
`rate:X`, `onoff:ON:OFF` or `random`.  The CCA sends `min(its rate,
buffered app data)`, and that *sent* rate is what is committed as the CCA's
move, so the QE history always holds the real arrivals and beliefs stay
sound.  The planner (`record_action`) sees both the intended and the sent
rate:

* An RTT with `sent < intended` is app-limited.  A probe that was not sent
  does not reset the probe timer, so the CCA probes once data returns.
* An RTT that delivered less than `INFORMATIVE_LOAD * min_c` does not count
  towards confirming a small K (`KTracker::stable_rtts`): an idle link
  cannot rule out a burstiness, so silence is not evidence for small K.
  Ruling K out still uses every RTT (that is always sound).

* The committed belief never gets broader while the evidence for it
  stands (`retained_prior` in `cong_ctrl.rs`): each RTT's C/B bounds are
  intersected with the previous committed ones, provided the K set did not
  grow (a K that was ruled out may have produced the older bounds).  With
  the default whole-history belief this applies every RTT; with
  `--belief-history N` while any of the last N RTTs was app-limited, so
  idle RTTs cannot push out what busy ones proved: the window forgets only
  once N busy RTTs have refilled it.  `state_policy`'s belief query, speculative
  tree states (intersected with the committed root) and window conflicts
  (which keep the older accumulated bound per dimension, instead of the
  newest window alone) follow the same rule.

The minimax tree still assumes a backlogged source (`APP_IS_BACKLOGGED`);
set it to `false` to let the adversary also choose the app supply.  The CSV
has `sent` and `app_limited`; `summarize_cca.py` reports utilization over
RTTs that were not app-limited.

### Belief soundness (bursty)

Forward-simulated traces (`BurstyLinkSim`) are feasible for the true
(C, B, K) by construction, so the tests in `network_model_nc` check that
beliefs never exclude the truth (`test_forward_sim_true_k_*`).  They found:

* The bursty crate's only sub-window C result (T=5, 1 loss) came from a
  query without the burst constraints (its input in `qe/bursty_cbrdelay_T6/`
  uses `A` rather than `P` and has no K), i.e. a plain bound.  `BurstyQe::c`
  ignores it (`BURSTY_C_RESOLVED`) until it is re-solved.
* The feasible C set of a window can be several islands, some a single point
  (a backlogged link serves exactly C).  `tighten_c_with_b` therefore takes the
  hull of the feasible `c_candidates` (grid plus observed and K-shifted rates)
  and `compute_belief_bounds` samples B at those rates too.  This is still a
  numerical search, not an exact projection.
* The QE-window simulator (`get_network_move_sim_policy`) chooses each service
  from the newest window alone, whose initial queue is free, so its traces can
  be feasible per window yet impossible overall (it can starve the link
  forever).  Use `BurstyLinkSim` for anything but `max`.

### Experiment scripts

```bash
sh scripts/cca_sweep.sh OUT 60 "20,10,1,max 20,9,4,min 50,200,1,random,3"
python3 scripts/summarize_cca.py OUT
```

Each scenario is `C,B,SIM_K,POLICY[,SEED]`.  The summary prints one compact
block per run (convergence, utilization, loss gaps, queue, drains, mode and
K timelines), which is small enough to hand to a cheaper model.

```bash
python3 scripts/train_cca_tree.py OUT [--searched-only] [--label action|mode|rate]
```

fits a scikit-learn decision tree from the belief before each RTT (row t-1,
mostly as multiples of `min_c`) to that RTT's choice (row t), and reports
leave-one-run-out accuracy.  The CSV's `candidates` column is the number of
root moves; `--searched-only` keeps the RTTs where minimax, not the
planner, made the choice.

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
