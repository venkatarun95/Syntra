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
had no restart.  Rises are only seen when the CCA sends above the old C:
the next step is explicit rise checks.

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
