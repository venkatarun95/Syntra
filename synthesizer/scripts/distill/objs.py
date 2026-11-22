"""For each searched RTT: which objective separated the root moves."""
import sys
from load import *
from collections import Counter
ORDER = ["lt", "schadd", "ttscbadd", "scb", "c", "l", "sent"]
def key(v):
    m = v["Value"]["metrics"]; b = v["Value"]["belief_bounds"]
    return {
        "lt": not m["loss_tol"], "schadd": not m["shrink_maxc_add"],
        "ttscbadd": m["time_to_shrink_minc_minb_add"], "scb": not m["shrink_minc_minb"],
        "c": b["max_c"] - b["min_c"], "l": m["total_loss"], "sent": m["neg_sent"],
    }
rows = load(sys.argv[1:])
dec = Counter()
for r in rows:
    if not r["searched"]: continue
    vals = {F(x["rate"]): key(x["value"]) for x in r["root_values"]}
    rates = sorted(vals)
    # objective deciding between the top rate and the best move
    best = r["chosen"]; top = rates[-1]
    for o in ORDER:
        if vals[top][o] != vals[best][o]:
            dec[("top-vs-chosen", o)] += 1; break
    else:
        dec[("top-vs-chosen", "tie" if top != best else "top")] += 1
    # what separated the moves at all
    for o in ORDER:
        if len(set(vals[x][o] for x in rates)) > 1:
            dec[("first-differing", o)] += 1; break
    else:
        dec[("first-differing", "all-equal")] += 1
for k, v in sorted(dec.items()): print(k, v)
print()
for r in rows:
    if not r["searched"]: continue
    vals = {F(x["rate"]): key(x["value"]) for x in r["root_values"]}
    if len(set(v["c"] for v in vals.values())) > 1 and all(len(set(vals[x][o] for x in vals))==1 for o in ORDER[:4]):
        B = r["B"]
        print(r["run"], r["t"], "chosen", float(r["chosen"]), {label_of(r,x): (round(float(x),2), round(vals[x]["c"],3)) for x in sorted(vals)}, "belief c", float(B["min_c"]), float(B["max_c"]), "q", float(B["max_q"]), "K", r["planner"]["k_assumed"])
