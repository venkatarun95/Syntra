"""Principal variations of each root move in a recorded search tree."""
import json, sys
t = json.load(open(sys.argv[1]))
def bb(n):
    b = n["data"]["belief_bounds"]
    return f'c=[{b["min_c"]:.3f},{b["max_c"]:.3f}] b=[{b["min_b"]:.2f},{b["max_b"]:.0f}]'
def obs(n):
    o = n["data"]["observation"]
    return f'a={o["a"]:.2f} s={o["s"]:.2f} lo={[(x["t"], round(x["l"],2)) for x in o["lo"]]}'
def val(n):
    v = n["data"]["chosen_value"]
    if not v or "Value" not in v: return str(v)
    m = v["Value"]["metrics"]; b = v["Value"]["belief_bounds"]
    return f'[lt={m["loss_tol"]} schadd={m["shrink_maxc_add"]} ttscb={m["time_to_shrink_minc_minb_add"]} scb={m["shrink_minc_minb"]} cw={b["max_c"]-b["min_c"]:.3f} l={m["total_loss"]:.2f} sent={m["neg_sent"]:.2f}]'
def walk(n, d):
    D = n["data"]
    print("  " * d, ("CCA rate %.3f" % D["cca_rate"]) if D["cca_rate"] is not None else "net", obs(n), bb(n), "val", val(n), "children", len(n["children"]))
for c in t["children"]:
    print("=== root move")
    walk(c, 0)
    for g in c["children"]:
        walk(g, 1)
        for h in g["children"]:
            walk(h, 2)
            if "--deep" in sys.argv:
                for i in h["children"]:
                    walk(i, 3)
