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

### Exact beliefs: B with C symbolic

Each window's belief is the *exact* projection of its feasible (C, B) set,
not a grid search.  The generated B functions are linear in C (and B) for
concrete observations, so `scripts/gen_b_lin.py` copies them into
`network_model_nc/src/generated/b_lin_*.rs` with C of type `ds::lin::Lin`
(`k + c*C`, `i128` rationals).  A comparison of two `Lin` values is answered
on the current *cell* of C (an open interval or a point); if its sign
changes inside the cell, the root is recorded and the cell is split
(`network_model_nc::exact::pieces`).  When no split remains, every cell's
result is exact: a list of pieces, each a cell of C with B's interval list
linear in C.  From the pieces:

* the feasible C set (`exact::c_set`) and B range (`exact::b_range`),
* `min_cb`, the smallest C + B of one feasible link (a loss pins C + B, so
  this beats `min_c + min_b`),
* the joint (C, B) set of several windows (`exact::intersect`): one link must
  explain every window.  `fold_belief_windows` intersects all windows of the
  history this way (`NetworkModel::joint_c_b_bounds`), on top of the
  per-window boxes; committed prefixes are cached, so this costs nothing
  measurable.  It does *not* enforce continuity of the hidden queue between
  overlapping windows; only a longer QE would.

The B formulas' clauses that do not mention B (on C or on the observations
alone) become "no B here" (`None`), not panics.  With the plain T=6 C query
hidden, the projection reproduces the QE's C exactly on every simulated
window (`test_c_tightened_with_b_matches_exact`); under the true K, per-window
and joint beliefs contain the true C, B and C + B with zero tolerance
(`test_forward_sim_*`), so `K_WINDOW_DISAGREEMENT_TOL` is 0.

Bounds are rounded *outward* to dyadic rationals (denominator a power of two
up to 1024): `RealNumRep` is `Ratio<i32>`, and mixing denominators (e.g. 3
and 1024) in cumulative arrivals overflowed.  Release builds now have
`overflow-checks = true`, so an overflow stops the run instead of silently
corrupting the history.  Points at which the concrete QE functions are
evaluated (`exact::representatives`) may be exact non-dyadic values with
modest denominators.

`MaxAllowedRate` from sampled (C, B) paths can miss the worst case (it once
returned `LARGEST_BW`), so the engine clamps it to the belief's sound bound
`max(min_c, min_cb - max_q)`.

When a window's C query is missing (the bursty T=6 C query did not finish),
`compute_c` intersects C from shorter sub-windows (where solved) with the
exact projection.

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

*Superseded:* the modes below are the pre-search design; the current
planner (Probe, Cruise, Drain, Recover, each with its own objective, and a
search every RTT) is described in "Search CCA" below.

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
  (a backlogged link serves exactly C).  The grid search that used to find it
  could miss islands and rounded inward, so it is replaced by the exact
  projection above.
* Under a union of K, a (C, B) can fit no K: Q, S and L are then the empty
  set (`infeasible_is_empty`, `UnionBurstyQe::union`), that adversary path is
  dropped, and a network node with no feasible move is scored as a leaf.
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

First results (32 runs × 60 RTTs: C 10-100, B 0.25-8 BDP, sim K 1-16,
max/min/random/mixed links, backlogged/on-off/rate/random apps):

* Minimax's own choices are nearly constant.  Of 491 searched (Explore)
  RTTs it chose `ProbeAboveLimit` in 479, `DrainQueue` in 8 and
  `MaxAllowedRate` in 4; no tree beats the majority class
  (leave-one-run-out 0.976).  The only split worth having is
  `max_b <= 0.75 min_c` -> `DrainQueue`.  With the current objective
  order (`lt` then `schadd`), the depth-4 search in Explore is therefore
  close to the fixed rule "probe unless the buffer is proven small".
* Over every RTT, a depth-4 tree reaches 0.94 leave-one-run-out (baseline
  0.37), but it relearns the planner: `max_q > 1.0 min_c` ->
  `DrainQueue` (`EXPLORE_MAX_QUEUE_BDP`), `max_c / min_c` around 1.4 for
  C tightness, and `HoldQueue` persisting in Cruise.  App-limited RTTs
  with tight C show up as repeated `ProbeAboveLimit`, because an unsent
  probe does not reset the probe timer.

### What the search decides, exactly

`--decisions FILE` writes one JSON line per searched RTT: the root's belief,
candidates, metrics and K set, the chosen rate, and every root move's value
as the search compared it (`Search::root_values`; a pruned child's value is
the bound the root saw).

The search tries the root moves last to first (`best_move_guess` pops) and
keeps the first unless a later one is strictly better, so a tie goes to the
last distinct candidate rate: `ProbeAboveLimit` when the planner offers it,
otherwise `DrainQueue`.  Note that `MaxAllowedRate` is *not* the choice when
probing is rationed; the planner offering {MaxAllowedRate, DrainQueue} means
DrainQueue.  Equal rates are deduplicated, so label the choice by its rate.

Over 30 varied 60-RTT runs (675 searched RTTs) every root move tied exactly
while `max_c` was still `LARGEST_BW` (the delay box lets the worst-case link
reveal nothing new in two RTTs, whatever the CCA sends).  With `max_c`
bounded the tie-break was still the choice in 214 of 216 RTTs; the other two
were `DrainQueue`, which won on window-local objectives (`scb`, the width of
C) that depend on the worst-case replies through QE, once with a single K
assumed.  A decision tree on the root's belief, candidates and metrics does
not predict those two (leave-one-run-out 0.944, below the 0.991 majority).

`explore_rule` returns the tie-break and whether it is known to be the
search's choice (`max_c` unbounded).  `--explore-rule` always uses it (no
search: 99.7% agreement above); `--explore-rule-exact` searches where it is
not known; `--check-rule` searches every RTT and marks the CSV column `rule`
`ok` or `MISMATCH` (`-known` where the rule claimed to know).

### External links

`--external` runs the same engine against a link outside the process: each
RTT it prints `rate R` and reads `SENT S APP_LIMITED [ROUND:L ...]` (round 0
is the first RTT, -1 the empty initial history).  Fed by a script replaying
`BurstyLinkSim`'s `max` policy it reproduces `--sim` exactly.  cc_sim's
`MinimaxQe` CCA (`src/minimax_qe.rs` in github.com/venkatarun95/cc_sim)
drives it from packets; its header explains the mapping (service counts only
packets sent before the round, and how unresolved losses are reported).
`--record FILE` saves the input lines; `examples/window_check.rs` in
`network_model_nc` replays them and tests every window under each K.

The model has no slack where a packet link deviates from it by a packet:
an RTT that overflows serves exactly C (`det_loss` with `I = I[t-1] + C`),
so a loss pins `B + C` exactly, and K only perturbs service when there is no
overflow.  Two consequences for packet traffic:

* The unit must be at least one packet (`K_WINDOW_DISAGREEMENT_TOL` and the
  other slacks are one unit).  With half-packet units, losses on a static
  cc_sim link ruled out every K within 16 RTTs.
* Service must not be counted past the start of the next RTT's arrivals at
  the link, i.e. at most up to one transmission time before the RTT ends:
  one extra packet of service before an overflow shifts `B + C` by one
  between losses, and the window becomes infeasible for every K.

### Planner changes after the exact beliefs (Nov 2026)

A 17-scenario sweep (C 10-100, B 5-400, K 1-16, max/min/random/mixed links,
on-off and rate-limited apps) found the planner, not minimax, at fault:

* *Deadlock*: on a K=4 min-service link the CCA sent 0 for 46 RTTs.  `max_q`
  includes what no rate can drain (bursts held back, unreported losses,
  delay-box lag), and `DrainQueue` waited for it forever.  Now
  `PlannerState::queue_floor` is the smallest `max_q` over the last
  `QUEUE_FLOOR_RTTS` RTTs that sent at least `QUIET_MARGIN` below `min_c`
  (any real queue drains in those), queue targets are at least the floor,
  and Drain/Explore use `HoldQueue` at the target rather than idling.
* *Idle exploration*: every probe was followed by 1-3 RTTs at rate 0.  Now
  every Explore rate is capped (`StatePolicy::queue_cap`) so the worst-case
  queue stays within `EXPLORE_MAX_QUEUE_BDP` (2: one BDP of delay box plus
  one of growth) plus `max(floor, K)`, and above the cap the CCA holds the
  queue there.  The link stays backlogged, which is what bounds C from above.
* *Compounding probes*: three aggressive probes in a row each spent a full
  loss budget before the first one's losses were reported.  Explore probes
  are now `EXPLORE_PROBE_GAP_RTTS` = 2 apart.

Before/after (60 RTTs, second-half utilization, packets lost): K=4 min link
0.00/0.73, 4/5; C=30 random 0.68/0.87; C=10 B=80 converges with 3 instead of
16 idle RTTs; C=75 mixed crashed at RTT 25 and now reaches 0.91 with 6 lost;
max links stay at 1.00.  The C=50 B=25 K=16 run is outside the model
(K < pert * B requires B > 32), and its belief excludes the truth: the model
cannot detect such a link.  The K confirmation (`K_CONFIRM_RTTS`) is an
assumption, not a deduction: on the mixed link it once confirmed K=1 (true
K=4) and briefly believed C = 6.3 (true 75).

### Ties, objectives and the exploration trade-off (Nov 2026)

`--exact-root` searches every root move with a full window, so the
`--decisions` values are exact (pruned children otherwise report bounds),
and `--depth` overrides `SEARCH_DEPTH`.  The transposition table is now
fresh for every decision: it stores alpha-beta bounds as if exact, and
values depend on the RTT's policy.

* At depth 4, 164 of 168 searched decisions (16 links) were exact ties on
  every objective; the other 4 preferred the higher rate (on `c`).  Two
  RTTs is shorter than both ways a probe pays off or costs: the link can
  hide C behind up to one RTT of service lag, and a probe's losses are
  reported about two RTTs later.  So `sent` (data sent over the horizon,
  which the adversary does not choose) is now the last objective; it agrees
  with every non-tie and leaves 6 genuine ties (moves whose follow-ups send
  the same total).  A distilled rule is correct if it picks any optimum.
* At depth 6 (about 20x slower) the search sees the trade-off: probing at
  2x `min_c` pins C exactly in the worst case but loses about one BDP, just
  over the budget, so with `lt` first it declines.  A lexicographic order can
  only pick an extreme of that trade-off; the loss budget is the real knob.
* The trade-off (16 links, 60 RTTs; `--explore-budget` in BDPs, queue cap
  2 BDP):

  | budget | C within 25% (median RTT) | never | util | util RTT 0-20 | loss (sum) |
  |---|---|---|---|---|---|
  | 0.25 | 20 | 4 | 0.68 | 0.51 | 55 |
  | 0.5 | 20 | 1 | 0.77 | 0.59 | 254 |
  | 1 (default) | 15 | 0 | 0.86 | 0.74 | 489 |
  | 2 | 11 | 0 | 0.90 | 0.84 | 896 |

  The queue cap (`--explore-queue` 1.5-4) barely matters (loss 469-550).
  This cannot be avoided: a link at `C = min_c` is indistinguishable from a
  faster one until the CCA sends more than `min_c` can serve, and on the
  slow link with a small buffer that excess is lost.
* An avoidable limit: one QE window (6 transitions, up to one RTT of lag)
  proves at best `C <= served / (T - 1)`, i.e. within 25%, and the joint
  fold does not carry the hidden queue between windows, so beliefs such as
  [10, 12.5] stop there however long the link stays backlogged.

### Chained C bounds (Nov 2026)

One QE window proves C only to within about 1/(T-1) of what it served
(e.g. `[10, 12.5]` for C = 10): the link may serve up to one RTT behind its
ideal curve, and every window's initial queue is free, so intersecting
windows (even jointly in (C, B)) never gets better.  `network_model_nc/src/chain.rs`
applies the model's equations to a whole run of consecutive explained
windows instead.  Unrolling `I[t] = min(I[t-1] + C, P[t] - L[t])` from `t0`
to `t1`, with `I[t-1] <= S[t] <= I[t]` and `A - K <= P <= A`:

* `C >= (S[t1] - S[t0+1]) / (t1 - t0)`;
* if `S[t1+1] < A[t1] - K - L[t1]`, then
  `C <= max((S[t1+1] - S[t0]) / (t1-t0), max_{t0<tau<t1} (S[t1+1] - A[tau] + K + L[tau]) / (t1-tau))`.

No backlog assumption is needed (a gap in the backlog makes a `tau` term
large).  L enters only through upper bounds (L never decreases, so any
later reported loss bounds it), which also copes with speculative histories
whose loss reports skip times.  K is the largest K hypothesis still alive.
`fold_belief_windows` chains each run of consecutive explained windows (a
dropped window may be a link change) and passes the result to
`joint_c_b_bounds`, which also tightens B through the pieces.

`test_chained_c_bounds_sound` checks the bounds contain the true C, with no
tolerance, on forward-simulated histories (C in {10, 20, 37}, five (K, B),
four link patterns) and their suffixes.  The whole histories pin C to about
0.5% (e.g. `[9.96, 10.006]`).

On the 16-link sweep it fixed the case it was meant for (C=10 B=80:
`[10, 12.5]` to `[10, 10.19]`) and changed no decision but one mixed link
(second-half util 0.73 to 0.78).  What remains is not an inference gap: in
Cruise the CCA sends exactly `min_c`, the queue empties, and a faster link
gives identical observations (C=100 B=400 stays `[100, 114]`, C=15 B=60
`[15, 16]`).  Only sending above `min_c`, i.e. the exploration trade-off,
can narrow it, and there C = `min_c` already.

### Depth 6, and C tightness without a K allowance (Nov 2026)

`C_TIGHT_PER_K` was 1/2: with K = 64 still alive, any belief narrower than
32 packets/RTT counted as tight, so a depth-6 run on C=20 B=10 K=4 (`min`)
left Explore at C in [14.68, 20] and cruised below 14.68 (0.55 util).  The
chained bounds amortize bursts (K/(t1-tau)), so the allowance is now 0.  At
depth 4 this changed nothing; at depth 6 that link went to 0.70 and C=20
K=4 random to 0.90.

16 links, 60 RTTs (15 counted):

| search | util | util 0-20 | loss total | loss max | never within 25% | median step |
| --- | --- | --- | --- | --- | --- | --- |
| depth 4 | 0.86 | 0.74 | 490 | 150 | 0 | 3.9 s |
| depth 6 | 0.86 | 0.72 | 232 | 56 | 1 | 22 s (max 373 s) |

Depth 6 sees both sides of a probe, so it declines 2x probes whose loss it
can now see: half the loss at the same utilization (not reachable by
lowering the depth-4 budget: 0.5 BDP gave 0.77 util for 254 lost), at the
cost of slower C convergence on small buffers (C=40 B=20 ends at
[37.9, 55.6]).

## Changing links and convergence

`--sim-c-schedule T:C,...` changes the simulated link rate from RTT T on;
the CSV has `true_c`, `belief_start` and `restarts`, and
`scripts/convergence_cca.py RUN_DIR...` reports, per run and per segment
between changes, the RTTs until the belief holds the true C tightly,
utilization against the true C, and loss.

A QE window that straddles a change has no static explanation at all.
`NetworkModel::try_compute_belief_bounds` returns `None` for it: the lenient
fold skips it, the strict fold reports it as an `infeasible` conflict, and
while the newest window is infeasible `state_policy` offers only
`DrainQueue` (there are no feasible network moves to search over).  Before
this, any rate change crashed the binary a few RTTs later.

Belief strategies (`BeliefStrategy`, CLI switches; defaults are unchanged):

* `--change-restart`: a confirmed K that stops fitting is either a burstier
  link or a changed one.  K escalates as before, and a shadow belief under
  the confirmed K is built from windows that start after the conflict.  If
  `SHADOW_WINDOWS` of them fit, the link changed: the belief restarts there
  (`belief_start`), K stays confirmed, and the committed C/B bounds reset.
  If one does not fit, the shadow is dropped.  If no K fits at all, the
  restart is immediate.  Restarting on any disagreement (the first version)
  is not usable: in the cc_sim port bursts triggered hundreds of restarts.
* `--c-upper-rtts N`: only the newest N RTTs bound `max_c` for the planner.
  Meant for rises in C that probes at the old `max_c` cannot reveal; it
  never triggered in these runs (the link stayed busy), so it is unproven.
* `--k-confirm N`, `--k-optimistic`: confirm the smallest K sooner, or
  assume it from the start.  Both are unsafe: on a K=4 random link the K=1
  belief settled on C in [12.4, 12.5] with true C=50 and never re-explored
  (0.19 utilization over 40 RTTs); with N=5, C=50 became tight at RTT 38
  instead of 16 and C=100 lost 22%.  The 20-RTT confirmation is a safety
  margin, not the convergence bottleneck.
* `--explore-free` (an objective change): do not force `DrainQueue` once the
  queue may exceed one BDP; minimax chooses with `qdel` second in the order.
  It then picks `ProbeAboveLimit` with the cautious 1-packet budget every
  RTT: a standing queue instead of probe/drain pulses.  C=20 B=20: 3.3% loss
  instead of 10% and 0.95 utilization instead of 0.91, but C tight at RTT 24
  instead of 10; C=50 B=100 (bursty) was far slower.  Not adopted.

Results (60 RTTs, sim K=1, `max` link unless noted; `summary.txt` of
`cc_sim/experiments/minimax/minimax_runs` has every run):

| scenario | strategy | after the change |
| --- | --- | --- |
| C 20 -> 50 at RTT 25 | none | `max_c` stuck at the old C, 0.40 utilization |
| | `--belief-history 20` | 0.84, 131 packets lost |
| | `--change-restart` | detected the next RTT (no K fits), 0.83, 68 lost |
| C 50 -> 20 at RTT 25 | none | C never tight again, 137 lost |
| | `--change-restart` | shadow confirmed after 8 RTTs, C tight 9 RTTs after the change, 120 lost |
| C 20 -> 40 -> 20 | none | 0.50 after the rise |
| | `--change-restart` | 3 restarts; 0.62 after the rise, C tight 4 RTTs after the fall |
| C=50 B=100 K=4 random, static | `--change-restart` | identical to the default: no false restarts |

On a `random` link (which may under-serve) a rise in C cannot be told from
a link withholding service, and no strategy detected it.

### Change detection by the longest consistent prefix (Nov 2026)

Change restarts are on by default (`--no-change-restart` turns them off).
After the K update each RTT, `CongCtrlState::locate_change` checks the
belief's history (strict fold: every window, the joint (C, B) bound and the
chained C bounds) and, if no link explains it:

1. If K values that no window has ruled out, other than the assumed ones,
   explain the whole history, the link is burstier than assumed (K
   confirmation is an assumption): the assumed K are ruled out and
   confirmation starts over.  No restart.  Without this, a wrongly
   confirmed K=1 on a K=4 random link restarted the belief 4 times.
2. Otherwise the link changed.  If `history[start..e]` is consistent and
   `history[start..e+1]` is not, the change came no later than observation
   `e`, so the belief restarts at `e`; repeated while the rest is
   inconsistent.  Consistency of prefixes is monotone, so `e` is found by
   binary search.  The longest consistent *suffix* (tried first) is unsound:
   after C 20 -> 50 it kept pre-change windows because one fictitious link
   (C = 40) explained both sides, and the belief settled on C in [40, 40].

Only T = 6 QE tables exist, so for the 6 RTTs after a restart there is no
window.  `fold_history` then uses `NetworkModel::fresh_belief_bounds` (the
chained C bounds, B unknown, queue at most what is in flight) and the planner
is in `Mode::Recover`, sending `MatchDelivery` (the delivery per RTT since
the restart): holding the queue on an in-flight bound, which includes up to
an RTT of service lag, idled the link at 1-5 packets/RTT.  Recover is a
heuristic; the beliefs stay sound.

Simulated changes (`--sim-c-schedule`, and new `--sim-b-schedule`; CSV
`true_b`), 60 RTTs, K=1 `max` unless noted, after the change:

| change | off: util / loss | on: util / loss | detected | C within 25% after |
| --- | --- | --- | --- | --- |
| C 20 -> 50 at 25 | 0.41 / 0 | 0.80 / 64 | RTT 29 (probe) | 17 RTTs |
| C 50 -> 20 at 25 | 1.00 / 130, C never right | 1.00 / 70 | RTT 27 | 13 RTTs |
| C 20 -> 40 -> 20 | 0.51, then 1.00 | 0.60, then 1.00 / 108 | RTT 29, 42 | -, 12 RTTs |
| B 40 -> 10 at 25 | B never right | 1.00 / 30 | RTT 26 | - |
| B 10 -> 60 at 25 | not detected | not detected | - | - |
| C 30 -> 60 at 30, K=4 random | 0.44 | 0.43 | RTT 52 | - |

No belief excluded the true (C, B) after a restart, and the 16 static links
had no restart.  Rises are only seen when the CCA sends above the old C,
hence the rise checks below.

### Min-RTT samples (Nov 2026)

The model's time step is the min RTT; a wrapper that measures it needs RTTs
whose queue is provably near empty.  `PlannerState::min_rtt_sample` is set
when the belief proves `max_q <= DRAIN_MAX_Q + K` (not the queue floor).
A drain is due once no RTT has been one for `DRAIN_INTERVAL_RTTS` (40), so
on a link with an unprovable floor the CCA still drains explicitly; a drain
that ends at the floor also resets the timer.  The CSV has a
`min_rtt_sample` column, and `--external --report-queue` prints
`rate R maxq Q min_rtt_sample 0|1` (cc_sim's `MinimaxQe` parses only
`rate R`, so this is opt-in).  On the 16-link sweep the longest gap between
samples was 47 RTTs (40 plus a 3-RTT drain); throughput and loss were
unchanged.

### Rise checks (Nov 2026)

A rise in C is invisible at rates up to `max_c`, and a rise in B at rates
the old buffer absorbed, so neither ever contradicts the belief on its own.
`Mode::RiseCheck` sends `AboveMaxC` (`max_c * (1 + RISE_STEP)`, at least one
packet more) for `ceil(r / (r - max_c)) + 1` RTTs (6 with `RISE_STEP` = 1/4).
If C rose to at least `r`, the chained lower bound passes `max_c`; with a
small buffer, if B rose, the absence of loss is unexplainable.  Either way
`locate_change` restarts the belief; otherwise the planner drains what the
check queued.

The cost is inherent to the delay box, not to the step: the link may serve
up to an RTT late, so proving `C > max_c` from rate `r` takes about
`r / (r - max_c)` RTTs, which queue about `(r - max_c) * r / (r - max_c) = r`
packets if C did not rise.  So every check costs about one BDP of queue, or
loss when the buffer cannot hold it.  A larger step only shortens the check.

Schedule (user-approved trade-off):

* Lossless checks (not cautious, and the worst-case queue after another
  RTT, at `C = min_c`, fits in `min_b`) start `RISE_INTERVAL_RTTS` (10)
  after a restart or after C becomes tight, and back off 10, 20, 40
  (`RISE_MAX_INTERVAL_RTTS`).  A lossless check ends early once the next
  RTT might overflow `min_b`.
* Lossy checks (cautious: small buffer or unconfirmed K) wait 40 RTTs, and
  end at the first reported loss, which says the old buffer still
  overflows.  This is the only B-rise check: with a large buffer a B rise
  does not matter.
* A drain cannot interrupt a check; a check waits for a drain.
  `--no-rise-check` turns checks off.

Change scenarios (80 RTTs; compare the 60-RTT table above):

| change | after the change, with rise checks |
| --- | --- |
| C 20 -> 50 at 25 | 0.93 utilization (was 0.80), 64 lost |
| C 20 -> 40 -> 20 | 0.81 after the rise (was 0.60), C tight 15 RTTs after it |
| B 10 -> 60 at 25 | detected by the first lossy check (restart at 57; was never) |
| C 30 -> 60 at 30, K=4 random | detected at 47 (was 59), 0.79 utilization (was 0.43) |
| C 50 -> 20, B 40 -> 10 | unchanged (falls are detected anyway) |

No belief excluded the true (C, B) after a restart.  On the 16 static links
(80 RTTs, counting RTTs 20-80) there was no restart.  Utilization went
from 0.893 to 0.901 (checks also tighten C on `random` links).  Unacked
data averaged 1.58 BDP instead of 1.51 (it includes the one-BDP delay
box).  Loss went from 88 to 243 packets: the lossy checks on buffers under
a BDP cost about `r - B` each (C=100 B=50: 52 packets in one check).

## Distilled policy (Nov 2026)

*Superseded:* `--distilled` and `distilled_actions` were removed with the
redesign in "Search CCA" below, which searches every RTT.  This section
records what the rule was and how it was checked.

`--distilled` runs the CCA without any search: beliefs, K tracking, change
detection and the planner's memory are computed exactly as before, and the
action comes from `distilled_actions` in `cbrdelay_cca_config.rs`: a list of
actions, of which the CCA sends the **highest rate** (the engine's candidate
formulas, with the mode's loss budget, queue target and cap).  The CSV
column `rule` is `dist-ok` when that list is exactly the planner's
candidate set, `dist-search` where it defers to the search (below).

### The rules

`BDP` is `min_c`; `K` the largest assumed burstiness; `target` =
`max(1 + K, queue_floor)`.  First match wins:

| # | condition (belief / planner memory) | mode | action (rate) |
| --- | --- | --- | --- |
| 1 | fewer than 7 observations since the belief (re)started | Recover | `MatchDelivery` (delivery per RTT over the last 1-2 RTTs) |
| 2 | no static link explains the newest QE window | (any) | `HoldQueue` (`min_c - max(0, max_q - target)`) |
| 3 | a rise check is running: started < `ceil(r/(r - max_c)) + 1` RTTs ago, and (lossless) `max_q + r - min_c <= min_b` or (lossy) no new loss | RiseCheck | `AboveMaxC` (`r = max_c + max(max_c/4, 1)`) |
| 4 | draining and `max_q > target` for < 3 RTTs, or no min-RTT sample (`max_q <= 1 + K`) for 40 RTTs | Drain | `HoldQueue` |
| 5a | `max_c > 1.25 min_c` and `max_q > 2 BDP + max(queue_floor, K)` | Explore | `HoldQueue` at that cap |
| 5b | `max_c > 1.25 min_c` otherwise | Explore | **highest of** `MaxAllowedRate`, `HoldQueue` (= `min_c` here) and, if >= 2 RTTs (4 if `max_b < min_c`) since the last probe, `ProbeAboveLimit` (`MaxAllowedRate` + max(1 BDP, 2), or + 1 if `max_b < min_c`); every rate capped at `min_c + 2 BDP + max(queue_floor, K) - max_q` |
| 6 | C tight, a rise check due (not app-limited; 10, 20, 40 RTTs after the last for lossless checks, 40 otherwise) | RiseCheck | `AboveMaxC` |
| 7 | C tight, >= 10 RTTs since the last probe | Probe | `ProbeAboveLimit` (budget 0 if cautious, else 1) |
| 8 | otherwise | Cruise | `HoldQueue` |

`MaxAllowedRate` is the model's no-loss rate, clamped to
`max(max(min_cb, min_c + min_b) - max_q, min_c)`.

Rows 1-4 and 6-8 are the planner (`state_policy`) restated: there the
planner leaves one candidate and nothing is searched.  Row 5b is the only
place the depth-4 search chooses, and the distilled rule for it is a single
sentence: **send the most the candidates allow** -- except in one region,
where `--distilled` still runs the search:

* **Deferral region**: more than one distinct candidate rate, `max_c`
  bounded (`< LARGEST_BW`), and the highest rate is the queue cap's
  (`min_c + 2 BDP + max(queue_floor, K) - max_q`, i.e. the probe would fill
  the Explore queue cap).  26 of 725 searched RTTs (3.6%) in the final labels; the search picked the
  highest rate in 25 of them (see below).

### Data and agreement

Traces: `--decisions FILE --exact-root` now logs every RTT (searched or
not) with the belief (exact rationals), the planner's memory and derived
quantities (`PlannerFeatures`: C tight, small buffer, cautious, drain/rise/
probe timers and flags, queue cap, ...), the candidate set with exact rates,
the chosen rate and, where the search ran, every root move's exact value and
the set of optimal rates (a move is optimal unless the best value is
strictly smaller; equal values compare as `None`).  Scripts:
`scripts/distill/` (`run.sh` runs a scenario file, `evalp.py` scores the
policy, `objs.py` says which objective separated the root moves,
`tree.py` fits a generic decision tree, `cmpcsv.py`/`simcmp.py` compare
CSVs, `pv.py` prints principal variations of a `--tree` dump).

Training: `scripts/distill/train.sc`, 37 links × 70 RTTs (C 10-100, B
0.25-8 BDP, sim K 1/4/16, max/min/random/mixed, backlogged, on-off, rate
and random apps, 8 with C or B changes), depth 4, bursty QE.  New:
`new.sc`, 8 other links.


### Search fixes found by the distillation

Three searched decisions went against "highest rate" in the first labels.
Tree dumps (`--tree`, `scripts/distill/pv.py`) traced each one:

* **Speculative beliefs could be broader than their parent's** (s21 t48,
  s36 t49).  A speculative state's belief was intersected with the committed
  root only, so a leaf could forget what the node above it had proved: on
  s21 the probe's worst leaf had `min_c` 33.24 under a parent with 34.41,
  on s36 `max_c` went back from 17.5 to the root's 18.75.  `c` then
  preferred the lower rate.  Now every speculative belief is also
  intersected with its parent's (default; `--no-spec-monotone` restores the
  old behaviour).  The link is static within the tree, so this is sound.
* **Replies inconsistent with the speculative history were scored** (the
  same s36 dump: leaves at C = 12.5 under a parent with C in [13.33, 17.5]).
  When a reply leaves C or B disjoint from the parent's interval,
  `intersect_c_b` kept the leaf's own interval, i.e. the search scored a link
  that cannot exist.  Now such a reply is pruned: `game_over` scores it as
  the network's worst value (`NegInf`), so the adversary never picks it; if
  every reply is such, the network node has no move and is scored as a leaf,
  as before (`--no-prune-disjoint` keeps them).  There is no legitimate
  disjoint case in speculation: the planner, K set and belief start are
  frozen for the tree, and change detection (`locate_change`) runs only on
  committed history, which keeps its own handling.
* **n2 t68 is the search being right.**  Every root rate has the same 8
  first replies and no leaf on the deciding line is broader than its
  parent.  The worst reply serves exactly `min_c`.  The probe now (89.8)
  fills the Explore queue cap, so the next move is held near `min_c`;
  holding now (64.2) leaves room to probe next RTT.  Both orders send the
  same over the horizon (2631.5), so `sent` ties and `c` decides: the worst
  C width at the horizon is 20.8 for probe-then-hold and 14.7 for
  hold-then-probe.  Whether the later probe teaches more depends on the
  worst-case service lag at the horizon end, which no root feature
  predicted (one case in 725 searched RTTs; not enough to fit a rule).

Effect on the labels: over the common prefixes of the 45 runs (691 searched
RTTs where the old and new search saw the same state), the optimal set
changed at 2: s21 t48 (parent intersection; now the probe) and s22 t46
(pruning; {30, 60} became {60}, same choice).  s36 t49 changed too (seen
with parent intersection alone; under `--tie-high` that run diverges
earlier, at a tie at t34).  On s36 (C 40 -> 15) the fixed search probes
more after the fall: same utilization, 42 packets lost instead of 4.

### Agreement

Final labels: `--exact-root --tie-high`, both search fixes, 45 runs (37
training + 8 new) × 70 RTTs, `/tmp/cca_distill/search_v3`
(`scripts/distill/evalp.py`):

| | RTTs | searched | candidate set = planner's | rule's rate optimal | = search's own pick |
| --- | --- | --- | --- | --- | --- |
| all 45 runs | 3150 | 725 | 3150 | 3150 (26 deferred) | 3150 |

* Outside the deferral region the highest rate was optimal at all 699
  searched RTTs.  Inside it (26 RTTs) the search picked the highest rate 25
  times and `HoldQueue` once (n2 t68).  The n2 t68 pattern (hold first so
  the next move can probe) appears nowhere else, in the old or the new
  labels.
* What separated the root moves: `sent` in ~93% of searched RTTs, exact
  ties on every objective in ~5%, `c` in the rest.  With the search's
  default tie-break (last candidate in planner order) 9 ties went to a lower
  rate (s14, s36, n2, n4): equal total sent because the second move's
  queue-limited rate gives back what the first added.  `--tie-high` breaks
  ties towards the highest rate (all moves are optimal), which is what the
  distilled rule does; with the default tie-break the traces would differ
  at those ties.
* The rule has no fitted thresholds, so leave-one-run-out is the same.  A
  generic scikit-learn tree on ~30 features fits the old labels 100% but
  scores 0.985 leave-one-run-out (it memorises the exceptions).

### Simulation

`--distilled --exact-root --tie-high` (`/tmp/cca_distill/distilled_v3`) vs
the fixed search with `--exact-root --tie-high`
(`/tmp/cca_distill/search_v3`): **45/45 CSVs identical** on every column
but the step time, i.e. the distilled CCA reproduced every trace exactly
(3124 RTTs by the rule, 26 by the deferred search).  Mean step 0.14 s
instead of 1.42 s.  Per-run utilization, loss and queue are therefore
equal; over the 45 runs (RTTs 10-70): utilization 0.82, 2139 packets lost,
unacked data 1.72 BDP, belief `max_q` 0.97 BDP (old search: 0.82, 2101,
1.71, 0.96; only s14, s21 and s36 differ).  Table:
`/tmp/cca_distill/final/simcmp.txt`.

Held out (after the rule was fixed): 7 further links × 100 RTTs (C 100
B 400; C 40 B 20 random; C 50 B 100 K=4 random; C 20 -> 50 -> 25; B
10 -> 60; on-off app; C 60 -> 30 K=4 mixed), search and `--distilled` both
with `--exact-root --tie-high`: 700/700 RTTs optimal and equal to the
search's pick, 7/7 CSVs identical, 6 RTTs deferred (all exact ties that
include the highest rate).  So over 805 searched RTTs the search strictly
preferred a lower rate only once (n2 t68).

### Caveats

* The rule is empirical: on unseen links a decision could go against the
  highest rate, and `--distilled` cannot notice.  Re-check new scenario sets
  with the search (`run.sh ... --exact-root --tie-high`, then `evalp.py`).
* All labels are depth 4, bursty QE, default planner (`--explore-free` is
  not supported by `--distilled`), default `--param`s.
* Since the changes below, `--distilled` no longer searches in the
  cap-bound region: it sends the highest rate there too, and the CSV marks
  the RTT `dist-capbound`.  The one labelled exception (n2 t68: the search
  holds at 64.2 instead of probing to 89.8, on the worst-case width of C at
  the horizon) is ignored on purpose; `--distilled-search` restores the
  deferral.

## Depth 6, probe timing, Cruise queue, runtime parameters (Nov 2026)

Notes and reports: `reports/2026-11-22-depth6-probes/`.

### Depth 6

Same 7 links × 100 RTTs as the held-out set, `--depth 6 --exact-root
--tie-high` (search fixes on):

* **Cost.** A searched RTT takes 13-26 s (median per link), up to 72 s,
  about 10× depth 4 per extra move/reply pair.  Depth 8 would be minutes per
  searched RTT; not run.
* **Behaviour.** Utilization is the same (within 0.02) and loss is lower
  (brise 35 -> 15, cupdown 162 -> 107, cdrop_bursty 141 -> 96, onoff 25 -> 5,
  smallbuf 80 -> 8), but for a reason that is an artifact of the horizon:
  `lt` counts all loss since the belief started against one probe's budget.
  A probe at the root shows its whole worst case inside six plies (on
  bigbuf t8: the probe's 100 packets plus 11 that the service lag overflows
  afterwards = 111 > 100), while a probe one RTT later shows less, so the
  search postpones probes: Explore probes every 3 RTTs instead of 2.  On
  smallbuf it never probes again after t13: C stays at [38, 56] for 87 RTTs
  and the CCA never leaves Explore.  That is lower loss by not learning.
* With `--lt-slack 2` (`lt` allows two budgets) depth 6 is back to depth 4:
  the highest rate is optimal at 49 of 51 searched RTTs (was 67 of 106),
  the loss is depth 4's.  The two exceptions take the one-packet probe on
  the first probe.
* The bursty large-buffer link does not improve with depth (0.71 vs 0.72):
  its loss of utilization is in Cruise, which has one candidate.

So depth 6 shows no real gain; the CCA stays the depth-4 distilled rule.

### Why the bursty large-buffer link is below full utilization

On C 50, B 100, K 4 with random service, Cruise's `HoldQueue` sends
`min_c - (max_q - target)`.  The link may serve up to an RTT late, so the
worst-case queue `max_q` swings between 4 and 28 packets while `min_q` is 0,
and Cruise drains a queue that may not exist: it sends 25-48 at C 50.  Depth
does not change it (one candidate).  The trade-off is worst-case delay for
utilization, now `CRUISE_QUEUE_BDP` (`--cruise-queue`, `--param
cruise_queue_bdp=`): Cruise holds the worst-case queue up to that many BDPs.

| Cruise queue | util (from RTT 10) | mean / max worst-case queue (pkts) | final C belief |
| --- | --- | --- | --- |
| 0 | 0.78 | 20 / 57 | [47.9, 53.2] |
| 0.25 BDP | 0.81 | 22 / 59 | [48.5, 52.9] |
| 0.5 BDP (default) | 0.87 | 29 / 88 | [48.8, 50.3] |
| 1 BDP | 0.92 | 38 / 97 | [49.9, 50.1] |

A busier link also pins C.  On a link that serves on time this is a real
standing queue of up to 0.5 BDP.  The rest of the gap is Explore: with K
unconfirmed it takes ~30 RTTs.

### Probe timing and Recover

* **Probes are timed from the last probe or the last growth of the C
  interval, whichever is later** (`PlannerState::c_widened`): `min_c` falls
  or `max_c` rises at a restart, a larger K, or an expired `max_c`.
* **Why Recover lasts 7 observations.**  QE tables exist only for windows
  of 7 observations (T = 6), and a window that reaches back past a change
  mixes two links, so for 6 RTTs after a restart there is no window: the
  belief is the chained C bounds alone and the queue is bounded only by
  what is in flight (including an RTT of service lag).  Shorter windows
  would need new QE runs.
* **Recover now probes** (`ProbeDelivery`, every `EXPLORE_PROBE_GAP_RTTS`
  from the restart): it sends the delivery plus as much again as keeps the
  in-flight queue bound within `RECOVER_QUEUE_DELIVERIES` (2) deliveries.
  After a rise the bound is about one RTT of sending and this doubles the
  rate; after a fall the link is backlogged, the bound is 4-6 deliveries and
  it sends the delivery.  cupdown (C 20 -> 50 -> 25): Explore starts at
  `min_c` 50 instead of 25 after the rise; utilization 0.84 -> 0.87.
  `--no-recover-probe` turns it off.
* **One-packet probes** (`ProbeOnePacket`, `MaxAllowedRate + 1`) are
  offered next to the 1-BDP Explore probe whenever the budget exceeds one
  packet (`--no-probe-one-packet`).  At depth 4 the search never strictly
  prefers it (tied 6 times, loses 35 times: loss within the budget is free
  and the larger probe sends and learns more).  At depth 6 with `lt` slack it
  wins the two cases above.

Labels with these changes (`/tmp/cca_new/d4_all`, 45 runs × 70 RTTs, 714
searched): candidate set = planner's 3150/3150, distilled rate optimal
3150/3150; "highest" is not optimal only at n2 t68, as before.  On the 7
held-out links `--distilled` reproduces the search's CSV on every behaviour
column.

Default changes vs the previous section (7 links × 100 RTTs, depth 4):

| link | util before -> after | loss before -> after |
| --- | --- | --- |
| bigbuf | 0.95 -> 0.95 | 0 -> 0 |
| smallbuf | 0.81 -> 0.88 | 80 -> 80 |
| bursty | 0.72 -> 0.81 | 0 -> 0 |
| cupdown | 0.84 -> 0.87 | 162 -> 172 |
| brise | 0.98 -> 0.99 | 35 -> 35 |
| onoff | 0.49 -> 0.50 | 25 -> 25 |
| cdrop_bursty | 0.74 -> 0.75 | 141 -> 137 |

### Runtime parameters

Every planner constant is a field of `Tunables` (`tun()`), defaulting to
the constant; `--param NAME=VALUE` (repeatable) overrides it, e.g.
`--param c_tight_ratio=1.5 --param k_hypotheses=1/4/16`.  The strategy
switches are also `--param`s (`probe_interval_rtts`, `drain_interval_rtts`,
`k_confirm_rtts`, `explore_budget_bdp`, `explore_max_queue_bdp`,
`cruise_queue_bdp`, `rise_check`, `probe_one_packet`, `recover_probe`).  The
distilled rule was checked against the search with the defaults only.

### 64-bit rationals and long runs

`RealNumRep` is now `Ratio<i64>` (was `i32`).  Every run so far was at most
100 RTTs; at C 500 the `i32` engine overflowed (a panic, since release
builds check overflow) at RTT 396, with cumulative arrivals near 174k.  The
7 held-out traces are identical under `i64` (it can only differ where
`i32` overflowed).  The exact layer still rounds new bounds outward to
dyadic denominators up to 1024, which keeps denominators small; cumulative
counts still grow, so very long runs should rebase (A and S shifted by the
same amount leave every constraint unchanged).

Per-RTT cost grows with the history: every RTT, change detection
(`locate_change`) checks that the whole history since the belief started
is consistent, which includes the chained C bounds over every pair of RTTs,
and the belief folds every window.  A step takes 0.1 s at RTT 100 and 1.4 s
at RTT 900 (C 500, random service).  `--belief-history N` alone does not
help (change detection still reads everything) and forgets: the retained
belief is only kept while the window holds an app-limited RTT.
`--history-window N` bounds the cost: the belief folds the newest N
observations and is always intersected with the previous committed belief
(until a restart), and change detection checks only the newest N (an older
change was checked when it was newer; the K tracker still accumulates every
window).  With N = 64 the 7 held-out links give identical traces.

## In cc_sim (Nov 2026)

cc_sim (`~/Projects/cc_sim`, branch `minimax-qe`, `src/minimax_qe.rs`, CCA `"mmqe"`) runs this
CCA itself over packet-level links: it drives `cbrdelay-cca --external --external-json
--distilled --history-window 64` and only converts packets to the model's per-round
observations and back.  Engine side:

* `--external-json` prints each RTT's decision record after the rate (`rate R {...}`): belief,
  planner memory and features, candidates, mode, `rule`, restarts.
* Observation lines take exact fractions (`n/d`); a line ending in `replay` is a round replayed
  to a fresh engine (after a change of units), and what was sent counts as the CCA's own action,
  so the planner's memory follows what happened.
* `window_check` (`network_model_nc/examples`) reads fractions too.

What the packet-level link taught (each made the belief restart on a constant link, i.e. the
observations did not fit the model):

* **Boundary losses.**  cc_sim's propagation delay is before the bottleneck, so a packet sent at
  the start of a round reaches the queue at the boundary of QE time.  If the previous round left
  the queue full, the link drops it although the new round sends less than C.  The model checks
  overflow only at the end of a round, where a round that sends less than C cannot overflow, so
  it needs K >= 6 packets to explain one such loss (the next round's losses then need 56 packets
  of service in a round of C = 50).  cc_sim now paces a round's packets at `(k+1)/(quota+1)` of
  the round: the first one arrives a gap after the boundary, after the busy link has finished a
  packet.
* **Pacing gaps.**  A round boundary computed in floating point could fall a hair after the
  timer meant to end the round, which skipped a round; the pacer then sent the next round's
  quota as a burst (a BDP at once).  Boundaries are now whole microseconds.
* **Units.**  The prior is 5 to 1000 units per round; cc_sim keeps C inside it by choosing the
  unit (`2^u` packets, never below one packet: a packet is indivisible) and the round (`2^m`
  RTTs, for links below about 10 packets per RTT).  The K hypotheses are physical (1, 4, 16, 64
  packets) and are passed in units, rounded up.  Rescaling is only approximately scale-invariant:
  the 1/1024 rounding grid and the one-unit constants (`DRAIN_MAX_Q`, `CAUTIOUS_LOSS_BUDGET`, the
  one-packet probe) are in units, so after a rescale a decision can differ (on 1 Gbit/s, C was
  just tight at [4000, 4984] packets and just loose at [3999, 5016] after the replay).

From 1 kbit/s to 100 Gbit/s: below about 10 packets per RTT the rounds get long (1 kbit/s with
1500-byte packets is a packet every 12 s, so the handshake RTT itself is 12 s and rounds are
minutes), and cc_sim's clock (1 us) cannot pace more than a packet per microsecond, so
100 Gbit/s needs packets of 12.5 kB or more.

Report: `reports/2026-11-22-cc-sim/`.

## Search CCA: the planner picks the objective, the search picks the rate (Nov 2026)

Principle: **trust the planner.**  Hand-designed rules (probe gaps, rise
checks, Cruise's queue target, the Explore cap) are replaced by candidate
actions and objectives, and minimax decides.  The CCA searches every RTT
(no distilled rule), so every decision is minimax-optimal for its mode's
objective by construction.

**Modes** (`state_policy`) only select the objective and the candidates:

| mode | when | objective (`*_OBJECTIVE_ORDER`) | candidates |
| --- | --- | --- | --- |
| Probe | C loose (`max_c > 1.25 min_c`) | `ld`, `schadd`, `ttscbadd`, `scb`, `c`, `l`, `s`, `sent` | `MaxAllowedRate`, `ProbeHalf`, `ProbeAboveLimit` |
| Cruise | C tight | `ld`, `s`, `l`, `sent` | `DrainQueue`, `MaxAllowedRate`, `ProbeAboveLimit` |
| Drain | no min-RTT sample for `DRAIN_INTERVAL_RTTS` | `dq`, `ld`, `s`, `l`, `sent` | `DrainQueue`, `DrainHalf`, `MaxAllowedRate` |
| Recover | under a QE window since a restart | (no search: no window, no network moves) | `MatchDelivery` or, every 2 RTTs, `ProbeDelivery` |

`RiseCheck`, `Explore`, `HoldQueue`, `AboveMaxC` and `ProbeOnePacket` are
gone.  `MatchDelivery`/`ProbeDelivery` are offered only in Recover, where
no QE window exists.

**Actions.**  `MaxAllowedRate` is now the belief's own bound,
`max(min_cb - max_q, min_c)`, capped so the worst-case queue (at
`C = min_c`) stays within `MAX_QUEUE_BDP` (one BDP) plus K or the floor: a
queue beyond a BDP adds delay and no throughput, and above the cap it
drains to it (which replaces `HoldQueue`).  The model's
`compute_max_allowed_rate` is no longer used: it minimizes over the newest
window's links alone, which include links older windows ruled out (C below
`min_c`), so `min(model, belief)` kept Cruise below `min_c` and the link
never provably backlogged.  `ProbeAboveLimit` is `MaxAllowedRate` plus what
is left of the loss budget at that node of the tree, `ProbeHalf` plus half
of it; `DrainQueue` drains `max_q` to `queue_target` in one RTT,
`DrainHalf` half of the excess.

**Loss budget over a window, across restarts.**  `ld`, `l` and `lt` count
the loss reported since the start of the newest `LOSS_WINDOW_RTTS` (20)
committed RTTs, plus the horizon, not since the belief started: a restart
no longer forgives loss, and repeated or overlapping probes are charged
together.  `ld` (loss beyond the budget, smaller is better) comes first,
not the boolean `lt`: once the window is over budget every move tied on
`lt`, and the search then probed freely.  The budget is a BDP (`min_c`)
while C has not been tight since the belief started (startup or a detected
change), otherwise `CONSTANT_LOSS_PKTS` (1 packet, via `pkts_per_unit`,
which cc_sim passes; at least 1/16 unit).

**`max_c` expires** (`C_EXPIRY_RTTS` = 20): the belief uses the newest 20
RTTs' windows, and the retained prior keeps `min_c` and B but not `max_c`.
C is assumed constant only that long; when nothing in the last 20 RTTs
bounds C from above the CCA is in Probe again (with the 1-packet budget,
since C was tight before).  On a lossless link the windows bound `max_c`
only around a loss, so in cc_sim this happens about 25 RTTs after the last
loss: the constant cost of looking for a rise.

**K < BDP** (`K_BELOW_BDP`): a K hypothesis is ruled out once its windows
prove `C <= K`, and `min_c` is at least the smallest assumed K.

**B carried across restarts** (`CARRY_BUFFER`): after a restart the prior
keeps the old `min_b` (a rate change rarely changes the buffer in packets);
a window that contradicts it wins, as for any conflicting bound.

**New objective `dq`**: the smallest `max_q` reached in the horizon, with
anything at or below `queue_target` counting as zero (a drain completed
within the horizon, however spread).

**Depth 6 is required.**  A probe's loss is reported two rounds after it is
sent.  At depth 4 (two rounds) the root probe's own loss never appears in
the tree, so every root move tied on `ld` and the search probed every RTT:
on a C=20, B=20 link it sent three probes in a row and lost 3 BDP in
startup (and 2.5 BDP on cc_sim's 12 Mbit/s, 50 ms, 1 BDP link).  At depth 6
it probes, waits for the result, and probes again: the old 2-RTT probe gap,
found by the search.  The cost in cc_sim with seven runs in parallel is
30-55 s per RTT in startup (all four K alive), about 60 MB per process.

**All objectives in every mode.**  Each mode lists its priorities and
`full_order` appends every other objective (`ALL_OBJECTIVES`), `sent` last:
an extra objective only breaks ties the earlier ones leave.  This changed
Cruise: where worst-case delivery ties, `dq` now prefers a one-round drain
and refill (rate 5 then 25-35 on C=20, B=20) over holding at C, since a
BDP of queue keeps the link busy through the drain round.

**`--replay N`**: the first N observations are replays (cc_sim re-sending a
segment in new units); their decisions are discarded, so they are not
searched.

Evaluation: `reports/2026-11-23-search-cca/` (cc_sim focus set, depth 6,
against Reno, Copa and the ack-window CCA).

## Variable-depth search: quiescence on outstanding losses (design, not built)

**Why.**  Depth 6 is needed only because a probe's loss is reported two
rounds after it is sent: at depth 4 a leaf can hold unreported loss, which
`ld` cannot see, so the search treats a probe at the root as free (the
horizon effect).  Everything else the CCA decides is about as good at depth
4, and depth 6 costs 30-55 s per RTT.  So search to depth 4 everywhere and
extend only the leaves where loss is still outstanding, as chess
quiescence search extends only positions with captures pending.

**Outstanding risk.**  A round's worst-case loss is what it sent above the
rate its node proved lossless: at `C = min_c`, `B = min_b` and the largest
queue, a round sending `r` overflows by at most
`max(0, r - (min_cb - max_q))`, the excess of `r` over the uncapped
`MaxAllowedRate` at that node.  Record it per CCA move: a `risk_stack` in
`CongCtrlState` next to `history_stack` (pushed in `make_move` for a CCA
move, popped in `pop_move`), and for committed rounds in `PlannerState`
(the last `LOSS_LAG` rounds' risk, `LOSS_LAG = 2`).  A leaf is **quiet**
when the rounds whose loss is not yet reported (the newest `LOSS_LAG`,
speculative or committed) carry no risk, or, the cheaper test, when even
all of it could not change `ld`:
`window_loss + outstanding_risk <= loss_tolerance_abs`.

**Search.**  In `deepen_tree` (`minimax/src/lib.rs`), at `depth == 0`:
if `state.quiescent()` (a new `State` method, default `true`) or the
extension budget is spent, return `state.value()` as now.  Otherwise
continue for one more round (2 plies) with the CCA restricted to moves that
add no risk, i.e. `MaxAllowedRate` alone (a `quiet_only` flag that
`compute_cca_moves` honours; `DrainQueue` could be added), and the full set
of network replies, so the adversary reports the outstanding loss.  With
no new risk added, quiet is reached within `LOSS_LAG` rounds: cap the
extension at 4 plies.  The transposition table must key or depth-tag
extended entries so a depth-0 value of a non-quiet node is not reused as
exact.

**Cost.**  The extension branches only on network replies, and only on
paths that took a risk: in Cruise and Drain (no probe in flight) nothing
extends and the cost is depth 4's.  A probe's subtree costs depth 4's times
about `n^2` network replies over its extension, against depth 6's `(3n)`
per extra round, so it is a saving only where few paths are risky.  If
that is not enough, a static bound instead of the extension (charge
`outstanding_risk` to `ld` at the leaf, no search) is cheap and
conservative: it can only overstate the loss, so it never breaks the
budget, but it may probe less than depth 6.  The hybrid extends only
leaves whose static bound would push `ld` over the budget.

**Validation.**  Log `--decisions --exact-root` at depth 6 on the focus
set; replay the same histories with depth 4 + quiescence and require the
same chosen rate in every RTT (or a rate with equal depth-6 value), and
report the time per RTT of each.  Then the cc_sim focus set again.

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
