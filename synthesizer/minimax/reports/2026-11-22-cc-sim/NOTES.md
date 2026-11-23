# The distilled CCA in cc_sim (2026-11-22)

**Asked.**  Run the Syntra CCA itself (exact QE belief, planner, the
distilled decision rule `--distilled`, automatic units) over cc_sim's
packet-level links, next to Reno and Copa, and show why each round's rate
was chosen.

**Data.**  `/tmp/ccsim_sweep4` (mmqe, after the rise-check fix `a9284fe`,
146 rows) and `/tmp/ccsim_sweep1` (Reno, Copa); cc_sim
`experiments/minimax/run.py` (all scenarios, 7 jobs).  Page:
`python3 build.py SWEEP_DIR cc_sim_report.html [SCENARIO ...]`.

## Means after the 5 s warmup (utilization / loss % / q50 ms)

| group | n | minimax (distilled) | Reno | Copa |
| --- | --- | --- | --- | --- |
| static | 18 | 0.98 / 0.8 / 3 | 0.99 / 0.1 / 55 | 0.79 / 0.0 / 1 |
| bursty acks | 6 | 0.98 / 2.1 / 20 | 0.97 / 0.0 / 87 | 0.99 / 0.0 / 5 |
| changing | 4 | 0.95 / 2.9 / 11 | 0.96 / 0.2 / 24 | 0.86 / 0.4 / 3 |
| app-limited | 4 | 0.59 / 0.8 / 2 | 0.59 / 0.3 / 2 | 0.57 / 0.0 / 1 |
| two flows | 2 | 0.97 / 4.1 / 65 | 1.00 / 0.1 / 81 | 1.00 / 0.0 / 4 |
| rate range | 4 | 0.89 / 21.3 / 2939 | 0.99 / 10.3 / 6049 | 0.69 / 5.9 / 3818 |
| cellular | 32 | 0.57 / 22.7 / 81 | 0.89 / 2.0 / 73 | 0.63 / 1.1 / 16 |

## Findings

* Static links: near-full utilization at a few ms of queue; the 1.2% loss on
  1-BDP links came from lossy rise checks (a check at B <= 1 BDP cannot be
  lossless).
* Cellular and Poisson: heavy loss from slow drop detection.  K up to 64 and
  the one-RTT service lag let the adversary explain low delivery, so the CCA
  over-sent until the belief became infeasible and restarted (23-55 restarts
  per run).  Led to K < BDP, carrying B across restarts and the windowed loss
  budget in the search CCA.
* Rate range: 1 kbit/s and 100 kbit/s were degenerate (BDP under a packet;
  removed from the test set); 10 and 100 Gbit/s ran 4 s, dominated by
  startup (now 20 s).
* Startup is slow (doubling every two rounds): only Explore offered more than
  one rate, so no depth could find faster growth.

Superseded by `../2026-11-23-search-cca/` (search every RTT, per-mode
objectives).
