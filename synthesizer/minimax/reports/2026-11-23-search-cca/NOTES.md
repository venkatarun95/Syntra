# Search CCA in cc_sim (2026-11-23)

**Asked.**  Redesign the CCA so the planner only picks the mode (and with it
the objective order and candidates) and minimax picks every rate; test it
in cc_sim at depth 6 on a few varied cases, each a little past convergence,
against Reno, Copa and the user's ack-window CCA (window = acks of the last
g min-RTTs, at least 4 packets).  Design: `../../CCA_DEVELOPMENT.md`,
"Search CCA"; status and next steps: `../../HANDOFF.md`.

**Focus set** (cc_sim `experiments/minimax/run.py --set focus`, all at
50 ms): 12 Mbit/s static with 0.5, 1 and 4 BDP of buffer (6 s); step
24 to 6 and 6 to 24 Mbit/s at 3 s (7 s); Poisson 12 Mbit/s; acks released
at exponential 3 ms gaps; 1 Gbit/s (6 s, rescaled units); Mahimahi
TMobile-LTE-short and ATT-LTE-driving downlinks (10 s), 1 BDP.  Results
are for the whole run (startup included); time to 90% is the end of the
first 1/200-of-the-run bucket in which flow 0 got 90% of the link's
transmission opportunities.

**Commands.**

```bash
cd ~/Projects/cc_sim/.claude/worktrees/minimax-qe
MMQE_DEPTH=6 RUN_TIMEOUT=28800 python3 experiments/minimax/run.py /tmp/focus_d6 --set focus --ccas mmqe --jobs 7
python3 experiments/minimax/run.py /tmp/focus_base --set focus --ccas reno,copa,ackwin2,ackwin15 --jobs 1
cd -; python3 build.py search_cca_report.html /tmp/focus_d6 /tmp/focus_base
```

## Baselines (whole run)

| scenario | ack g=1.5 util / loss | ack g=2 util / loss | Reno util / loss | Copa util / loss |
| --- | --- | --- | --- | --- |
| static 0.5 BDP | 0.94 / 1.23% | 0.61 / 36.4% | 0.94 / 0.85% | 0.56 / 0% |
| static 1 BDP | 0.95 / 0% | 0.95 / 0.94% | 0.97 / 1.81% | 0.56 / 0% |
| static 4 BDP | 0.95 / 0% | 0.96 / 0% | 0.97 / 4.20% | 0.56 / 0% |
| step down 24 to 6 | 0.91 / 0.36% | 0.91 / 1.35% | 0.94 / 2.84% | 0.36 / 0% |
| step up 6 to 24 | 0.61 / 23.7% | 0.54 / 46.7% | 0.74 / 0.80% | 0.46 / 0% |
| Poisson | 0.94 / 0.73% | 0.93 / 12.4% | 0.95 / 1.53% | 0.54 / 0% |
| acks at 3 ms gaps | 0.94 / 0% | 0.93 / 7.06% | 0.94 / 2.56% | 0.52 / 0% |
| 1 Gbit/s | 0.87 / 0% | 0.87 / 0.01% | 0.92 / 1.79% | 0.01 / 0% |
| TMobile-LTE-short | 0.43 / 9.02% | 0.22 / 24.4% | 0.58 / 1.37% | 0.27 / 0.02% |
| ATT-LTE-driving | 0.90 / 7.20% | 0.81 / 23.2% | 0.86 / 1.05% | 0.64 / 0.95% |

The ack window holds g - 1 BDPs of queue, so it loses whenever the buffer is
smaller (0.5 BDP; after the step up the buffer is a quarter BDP).  Copa's
~0.56 everywhere matches earlier sweeps and was not investigated.

## Before the cc_sim runs (built-in link, C=20, B=20 units per RTT)

* Depth 4: three probes in a row in startup, 3 BDP lost (2.5 BDP on cc_sim's
  12 Mbit/s, 1 BDP link): a probe's loss is reported two rounds later,
  outside a two-round tree.
* Depth 6: probe, wait, probe (two half probes in flight once), 0.75 BDP
  lost; Cruise at C afterwards, sometimes a one-round drain and refill (the
  `dq` tie-break).

## Minimax at depth 6 (whole run; runs finished 2026-11-23 11:54 CST, ~80 min)

| scenario | util | loss | q p50 / p95 ms | t90 s | after warm-up util / loss |
| --- | --- | --- | --- | --- | --- |
| static 0.5 BDP | 0.93 | 0.71% | 9.0 / 21.0 | 0.45 | 0.99 / 0% |
| static 1 BDP | 0.93 | 1.07% | 33.8 / 44.7 | 0.45 | 0.98 / 0% |
| static 4 BDP | 0.90 | 0% | 14.1 / 81.7 | 0.45 | 0.94 / 0% |
| step down 24 to 6 | 0.82 | 3.74% | 7.8 / 87.6 | 0.84 | 0.90 / 0% |
| step up 6 to 24 | 0.87 | 1.39% | 1.0 / 22.0 | 0.245 | 0.99 / 0% |
| Poisson | 0.92 | 3.89% | 26.6 / 52.4 | 0.36 | 0.98 / 3.28% |
| acks at 3 ms gaps | 0.90 | 0.09% | 14.8 / 32.2 | 0.54 | 0.97 / 0% |
| 1 Gbit/s | 0.67 | 0.80% | 19.9 / 44.0 | 2.22 | 1.00 / 0% |
| TMobile-LTE-short | 0.84 | 10.8% | 15.1 / 36.4 | 0.15 | 0.91 / 8.04% |
| ATT-LTE-driving | 0.80 | 17.4% | 22.9 / 58.5 | 0.2 | 0.69 / 15.5% |

Per-mode breakdown from `mmqe_trace0.jsonl` (lost packets are attributed to
the round in which they are reported, ~2 rounds after sending):

* **Constant links and the step up are lossless after startup**, and the step up
  is the best of the five CCAs (Reno 0.74 / 0.80%, ack g=1.5 0.61 / 23.7%).
* **Cruise sends below C.**  4 BDP: Cruise picks `DrainQueue` in most
  rounds at 38-45 units with min C = 48, because the queue's upper bound stays
  5-10 units and every rate ties on `ld`, `s`, `l` while `dq` breaks the tie.
  Utilization after warm-up is 0.94.  On 1 BDP, `DrainQueue` was chosen in 23
  of 23 Cruise rounds.  Next: `--decisions --exact-root` on these
  histories before reordering (e.g. `dq` after a utilization objective).
* **Cellular loss is Recover's.**  TMobile: 21 restarts, 124 of 199 rounds
  in Recover, 1916 of 2037 lost packets reported there.  ATT: 18 restarts,
  106 of 139 rounds, 699 of 707.  The links refute the CBR belief every 6-8
  rounds and Recover's rule probe (~1.5x delivery every 2 rounds) meets a
  falling rate.  Recover is the rule flagged as an open question.
* **Step down** (one restart): 3.7% loss, mostly reported in Probe after the
  change; Reno 2.8%, ack g=1.5 0.36%.
* **1 Gbit/s startup** grows ~1.5x per round, so it takes 2.2 s to reach 90% (Reno 0.6 s).
