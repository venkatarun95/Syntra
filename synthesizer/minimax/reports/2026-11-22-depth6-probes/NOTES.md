# Depth 6, probes, the bursty link, and a perfect fit (22 Nov 2026)

Questions asked, and what we found.  Details and numbers are also in
`CCA_DEVELOPMENT.md` ("Depth 6, probe timing, Cruise queue, runtime
parameters").  Each report here has a summary table and then every RTT of
every run: what the CCA chose, what it sent, what was delivered and lost,
and its belief on C, B and the queue as it chose (with the true C and B).

| report | runs |
| --- | --- |
| `depth4-vs-depth6.md` | 7 links × 100 RTTs at depth 4, depth 6, and depth 6 with `--lt-slack 2` (planner of `a91f087`) |
| `cruise-queue.md` | C 50, B 100, K 4 random link with Cruise queue 0, 0.25, 0.5, 1 BDP |
| `defaults-before-after.md` | the 7 links with the old defaults and the new ones (Recover probe, one-packet probe, probe timer, Cruise queue 0.5 BDP) |

## 1. Depth 6 (and depth 8)

Depth 6 costs 13-26 s per searched RTT (median per link), up to 72 s,
about 10× depth 4.  Depth 8 would be another ~10×: minutes per searched
RTT, so offline only; not run.

Depth 6 had the same utilization and less loss, but the reason is a horizon
artifact, not foresight.  The `lt` objective allows one probe's budget of
loss over the whole history and horizon.  A probe sent now shows all of its
worst case inside the 6-ply horizon (bigbuf t8: 100 packets for the probe
plus 11 that the service lag overflows afterwards = 111 > 100), while the
same probe one RTT later shows only part of it.  So the search keeps
postponing probes: Explore probes every 3 RTTs instead of 2, and on the
small-buffer link it stops probing at t13 and never learns C (it stays in
Explore at [38, 56] for 87 RTTs).  Allowing two budgets (`--lt-slack 2`)
removes the artifact and depth 6 behaves like depth 4.  **No real gain; the
CCA stays depth 4.**

## 2. The bursty large-buffer link

Not a search problem: the loss of utilization is in Cruise, which has a
single candidate (`HoldQueue = min_c - (max_q - target)`).  On a link that
may serve up to an RTT late, the worst-case queue `max_q` swings between 4
and 28 packets while the queue may really be 0, so Cruise drains a queue
that may not exist.  Depth 6 does not change it (0.71 vs 0.72).

Fixing it is a trade-off of worst-case delay for utilization: let Cruise
hold the worst-case queue up to X BDP.  Utilization after RTT 10 is 0.78,
0.81, 0.87, 0.92 for X = 0, 0.25, 0.5, 1, with mean worst-case queues of
20, 22, 29, 38 packets.  **Default now 0.5 BDP.**  On a link that serves on
time this is a real standing queue of up to half a BDP.

## 3. Probe timing, and the 7 RTTs after a restart

Probes are now timed from the later of the last probe and the last time the
C interval grew (a restart, a larger K, an expired `max_c`).

The 7 RTTs: QE tables exist only for windows of 7 observations, and a
window reaching back past a change mixes two links.  So for 6 RTTs after a
restart there is no window and the CCA only has chained C bounds and an
in-flight queue bound.  It used to just match the delivery then, which
after a rise in C never grows.  **Recover now probes** every 2 RTTs: the
delivery plus as much again as keeps the in-flight bound within two
deliveries.  After a rise this doubles the rate; after a fall the bound is
already 4-6 deliveries and it holds.  On C 20 -> 50 -> 25 Explore now starts
at `min_c` 50 instead of 25 (utilization 0.84 -> 0.87).

## 4. One-packet probes

`ProbeOnePacket` (no-loss rate + 1) is offered whenever the Explore budget
is larger than one packet.  The depth-4 search never strictly prefers it
(loss within the budget is free, and the 1-BDP probe sends and learns
more); depth 6 with `lt` slack picked it twice, on the first probe.

## 5. Reports

`scripts/report/report.py` writes these reports (belief per RTT included).

## 6. A perfect fit with no search

With the new planner, relabelled at depth 4 (45 links × 70 RTTs, 714
searched RTTs): the rule "send the highest candidate rate" is optimal on
every RTT but one, n2 t68, as before.  There the search holds (64.2) rather
than probe to the queue cap (89.8) because probing next RTT leaves a
narrower worst-case C at the horizon (14.7 vs 20.8), which no feature of
the state predicts.  **`--distilled` now never searches**: it sends the
highest rate there too (the CSV marks such RTTs `dist-capbound`), so it is
exactly minimax-optimal on every labelled RTT except n2 t68.

Also: every planner constant is now a runtime `--param`, and the engine
uses 64-bit rationals (the 32-bit engine overflowed at RTT 396 of a C 500
run).
