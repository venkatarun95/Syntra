"""First RTT where two label sets (same scenarios) choose differently, and why."""
import sys, json, glob, os
from fractions import Fraction as F
a, b = sys.argv[1], sys.argv[2]
for pa in sorted(glob.glob(os.path.join(a, "dec", "*.jsonl"))):
    run = os.path.basename(pa)[:-6]
    pb = os.path.join(b, "dec", run + ".jsonl")
    if not os.path.exists(pb): continue
    ra = [json.loads(l) for l in open(pa)]; rb = [json.loads(l) for l in open(pb)]
    for x, y in zip(ra, rb):
        if x["chosen_rate"] != y["chosen_rate"]:
            same_state = x["candidates"] == y["candidates"] and x["belief"] == y["belief"]
            ya = set(y["optimal_rates"]); xa = set(x["optimal_rates"])
            why = "tie-break (same optimal set)" if same_state and xa == ya else ("optimal set changed" if same_state else "state differs")
            print(f"{run} t={x['t']} {a.split('/')[-1]}:{x['chosen_rate']} {b.split('/')[-1]}:{y['chosen_rate']}  {why}  opt {sorted(xa)} vs {sorted(ya)}")
            break
# Same state, different optimal set, over the common prefix of every run.
n = tot = 0
for pa in sorted(glob.glob(os.path.join(a, "dec", "*.jsonl"))):
    run = os.path.basename(pa)[:-6]
    pb = os.path.join(b, "dec", run + ".jsonl")
    if not os.path.exists(pb): continue
    for x, y in zip(map(json.loads, open(pa)), map(json.loads, open(pb))):
        if not (x["candidates"] == y["candidates"] and x["belief"] == y["belief"]): break
        if x["searched"]:
            tot += 1
            if set(x["optimal_rates"]) != set(y["optimal_rates"]):
                n += 1; print("  optimal set differs:", run, x["t"], sorted(x["optimal_rates"]), sorted(y["optimal_rates"]))
        if x["chosen_rate"] != y["chosen_rate"]: break
print(f"searched RTTs on the common prefixes: {tot}; optimal set differs at {n}")
