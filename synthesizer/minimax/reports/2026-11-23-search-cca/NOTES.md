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

## Minimax at depth 6

Pending: the runs in `/tmp/focus_d6` were started 2026-11-23 10:33 CST
(~37 s per RTT in startup with 7 in parallel).  Fill in the table and the
findings when they finish.
