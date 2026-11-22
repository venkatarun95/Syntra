import sys
from load import *
from policy import distilled_actions, distilled_rate
from collections import Counter, defaultdict
rows = load(sys.argv[1:])
names_ok = rate_opt = rate_exact = 0
bad = []
per_run = defaultdict(lambda: [0,0,0])
for r in rows:
    if "probe_due" not in r["pf"]:
        # Older traces: the planner's `since_probe >= PROBE_INTERVAL_RTTS` (10).
        sp = r["pf"]["since_probe"]
        r["pf"]["probe_due"] = sp is None or sp >= 10
    acts, rate = distilled_rate(r)
    planner = [a for a, _ in r["cands"]]
    n_ok = acts == planner
    o_ok = rate in r["opt"]
    e_ok = rate == r["chosen"]
    names_ok += n_ok; rate_opt += o_ok; rate_exact += e_ok
    pr = per_run[r["run"]]; pr[0]+=1; pr[1]+=o_ok; pr[2]+=e_ok
    if not (n_ok and o_ok):
        bad.append((r["run"], r["t"], r["mode"], acts, planner, float(rate), [float(x) for x in r["opt"]]))
n = len(rows)
print(f"RTTs {n}  runs {len(per_run)}  searched {sum(r['searched'] for r in rows)}")
print(f"candidate set == planner's: {names_ok}/{n}")
print(f"rate is minimax-optimal (any optimum): {rate_opt}/{n}")
print(f"rate == search's own choice (tie-break): {rate_exact}/{n}")
for b in bad[:40]: print("BAD", b)
for run,(a,b,c) in sorted(per_run.items()):
    if c < a: print(" tie-break differs:", run, f"{c}/{a}")
