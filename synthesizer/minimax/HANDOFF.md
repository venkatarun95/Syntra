# Handoff: the search CCA (2026-11-23)

Read this first, then `CCA_DEVELOPMENT.md` sections "Search CCA" and
"Variable-depth search" (the design and its rationale), and
`reports/2026-11-23-search-cca/NOTES.md` (the evaluation in progress).

## Goal and ground rules (from the user)

* A provably performant CBR-delay CCA from the minimax synthesizer: sound
  beliefs (exact QE), every decision minimax-optimal for a stated objective.
  Commit, document and test the work.
* **Trust the planner.**  Do not hand-design CCA rules (probe gaps, rise
  checks, queue targets).  Offer candidate actions and objectives and let the
  search decide; add depth where it helps.  "We can iterate on designs forever
  and not converge.  CCA is a 40 year old field.  I developed this methodology
  because intuitive design has failed."
* The planner picks a **mode**, the mode picks the **objective order** (every
  mode uses all objectives; only the order differs) and the candidates.
  Cruise maximizes utilization; Probe shrinks the belief while limiting loss
  (a BDP while C is uncertain, otherwise a constant); Drain proves the queue
  empty, otherwise maximizes performance.
* No time-varying link model: C is constant for short periods (`max_c`
  expires after 20 RTTs) and changes are detected.  K < BDP may be assumed.
* No queue beyond one BDP.  `MatchDelivery`/`ProbeDelivery` only where no QE
  window exists (Recover).
* Always build and run with `--release`.  Run independent minimax runs in
  parallel on all CPUs but one (7 of 8); parallelizing one search is futile.
* Run expensive settings (depth 6) only on a few varied cases, each a little
  past convergence, not on repetitive steady states.
* Ask before combining plain and bursty QE results.

## Where things are

| what | where |
| --- | --- |
| Syntra worktree | `~/Projects/Syntra/.claude/worktrees/cca-applimited-beliefs/synthesizer`, branch `cca-convergence` |
| planner (modes, actions, objectives, constants) | `minimax/src/cbrdelay_cca_config.rs` |
| engine (metrics, candidate rates, beliefs, CLI) | `minimax/src/cong_ctrl.rs` (built as `cbrdelay-cca`) |
| minimax search | `minimax/src/lib.rs` (`deepen_tree`) |
| metrics struct | `cc_common/src/lib.rs` (`Metrics`; `drain_q` is new) |
| cc_sim worktree | `~/Projects/cc_sim/.claude/worktrees/minimax-qe`, branch `minimax-qe` |
| cc_sim driver of the CCA | `src/minimax_qe.rs` (`MinimaxQe`) |
| ack-window baseline | cc_sim `src/ack_window.rs` (`AckWindow { gain }`) |
| scenarios and runner | cc_sim `experiments/minimax/run.py` (`--set focus`) |
| report builder | `minimax/reports/2026-11-23-search-cca/{build.py,page.tpl.html}` |

Commits so far: Syntra `2685bd6` (redesign) and the commit after it (all
objectives in every mode, this handoff, reports); cc_sim `97a61ca`
(baseline, live search, focus set, time to 90%).  `git` needs
`-c user.name="Venkat Arun" -c user.email="venkat@utexas.edu"` here (no
global identity).  `cargo` is `~/.cargo/bin/cargo`.

## Build and run

```bash
cd ~/Projects/Syntra/.claude/worktrees/cca-applimited-beliefs/synthesizer
~/.cargo/bin/cargo build --release -p minimax --bin cbrdelay-cca

# Built-in simulated link (C=20, B=20 units per RTT), 14 RTTs at depth 6:
target/release/cbrdelay-cca --qe-model bursty --sim-c 20 --sim-b 20 --sim-k 1 \
  --steps 14 --depth 6 --history-window 64 --csv --out /tmp/x
# Add --decisions FILE --exact-root for every root move's value.

cd ~/Projects/cc_sim/.claude/worktrees/minimax-qe
~/.cargo/bin/cargo build --release
MMQE_DEPTH=6 RUN_TIMEOUT=28800 python3 experiments/minimax/run.py /tmp/focus_d6 \
  --set focus --ccas mmqe --jobs 7
python3 experiments/minimax/run.py /tmp/focus_base --set focus \
  --ccas reno,copa,ackwin2,ackwin15 --jobs 1

cd ~/Projects/Syntra/.claude/worktrees/cca-applimited-beliefs/synthesizer/minimax/reports/2026-11-23-search-cca
python3 build.py search_cca_report.html /tmp/focus_d6 /tmp/focus_base
```

Rebuilding `cbrdelay-cca` while cc_sim runs is unsafe: cc_sim spawns a new
engine from the same path on every rescale, so a run would mix versions.

## State at handoff

* **Done:** depth-6 `mmqe` on the 10 focus scenarios (`/tmp/focus_d6`,
  finished 2026-11-23 11:54 CST, ~80 min with 7 in parallel) and baselines
  (`/tmp/focus_base`).  Report built (`search_cca_report.html`; publishing as an Artifact needs a claude.ai login); numbers and per-mode
  breakdown in `reports/2026-11-23-search-cca/NOTES.md`.  `/tmp` does not
  survive a reboot; rerun if missing.

## Findings so far

1. **Depth 6 is needed** for the loss budget to work without a hand-coded
   probe gap: a probe's loss is reported two rounds later, so at depth 4 the
   root probe's loss is outside the tree and every move ties on `ld`; the
   search then probed every RTT and lost 2.5-3 BDP in startup.  At depth 6 it
   probes, waits, probes (0.75 BDP lost on C=20, B=20, using two half
   probes in flight).
2. **`MaxAllowedRate` must be the belief's own bound** `max(min_cb - max_q,
   min_c)`.  The model's `compute_max_allowed_rate` uses the newest window
   alone (links with C below `min_c`), and `min(model, belief)` kept Cruise
   below C, so the link was never provably backlogged.
3. On a lossless link the windows bound `max_c` only around a loss, so `max_c`
   expires ~25 RTTs after the last loss and the CCA probes with the 1-packet
   budget: the agreed constant cost of looking for a rise.
4. With all objectives in every mode, Cruise drains below C: on 4 BDP it
   sends 38-45 units with min C = 48 for many rounds (utilization after
   warm-up 0.94), because the queue's upper bound stays 5-10 units and `dq`
   breaks the tie.  Check the root values (`--decisions --exact-root`) and
   then fix the objective order.
5. Baselines (focus set): the ack window with g=2 loses heavily whenever the
   buffer is below about a BDP of the current rate (36% at 0.5 BDP, 47% after
   the step up, 23-24% on cellular); g=1.5 is lossless on the static links
   and 1 Gbit/s but loses 7-9% on cellular and 24% after the step up.  Copa
   sits at ~0.56 utilization (as in earlier sweeps; suspicious, not
   investigated).
6. Focus results: lossless after startup on constant links and after the
   step up (best of five there); cellular loss (10.8%, 17.4%) is almost all
   in Recover (rule mode, 18-21 restarts); 1 Gbit/s startup is slow (2.2 s
   to 90%).

## Next steps (in order)

1. Cruise draining below C (finding 4), then cellular: replace the Recover
   rule (ask the user first; see open questions) and the restart rate on
   non-CBR links.  Compare against the old distilled CCA
   (`reports/2026-11-22-cc-sim/`, sweep4).
2. **Variable-depth search** (design in `CCA_DEVELOPMENT.md`, "Variable-depth
   search: quiescence on outstanding losses"): depth 4 plus extension of
   leaves with unreported risk.  Validate against depth 6 decisions on the
   focus traces, then measure the time per RTT.  This is what makes the full
   ~70-scenario set affordable.
3. Full sweep (`run.py` without `--set focus`, 7 jobs) once the cost allows,
   and rebuild the report from it.
4. Open questions to raise with the user rather than decide alone:
   Recover is still a rule (no QE window, so no search); Drain's timing is a
   rule (every 40 RTTs without a min-RTT sample); the loss window (20 RTTs)
   and constant budget (1 packet, at least 1/16 unit, i.e. 4 packets at 64
   packets per unit) are parameters, not derived.

## Known breakage

* The `#[cfg(test)]` module of `cong_ctrl.rs` does not compile, and did not
  before this work (it uses the removed z3 model and an old `CCValue`).
  `cargo test --bin cbrdelay-cca` therefore fails; the binary builds.
* `scripts/distill/` and the distilled-rule sections of `CCA_DEVELOPMENT.md`
  describe the removed `--distilled` rule (kept as history).
