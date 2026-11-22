"""The distilled policy in Python, over the logged features (mirror of
`distilled_actions` in cbrdelay_cca_config.rs)."""
from load import F
QE_WINDOW_OBSERVATIONS = 7

def distilled_actions(pf):
    if pf["since_start"] < QE_WINDOW_OBSERVATIONS:
        # Recover
        return ["ProbeDelivery" if pf["probe_ok"] else "MatchDelivery"]
    if not pf["newest_window_feasible"]:
        return ["HoldQueue"]
    if pf["rise_continues"]:
        return ["AboveMaxC"]
    draining = pf["prev_mode"] == "Drain" and not pf["drained"] and not pf["drain_gave_up"]
    if draining or (pf["drain_due"] and not pf["drain_gave_up"]):
        return ["HoldQueue"]
    if not pf["c_tight"]:
        if pf["queue_high"]:
            return ["HoldQueue"]
        a = ["MaxAllowedRate", "HoldQueue"]
        if pf.get("probe_small_too"):
            a.append("ProbeOnePacket")
        if pf["probe_ok"]:
            a.append("ProbeAboveLimit")
        return a
    if pf["rise_due"]:
        return ["AboveMaxC"]
    if pf["probe_due"]:
        return ["ProbeAboveLimit"]
    return ["HoldQueue"]

LARGEST_BW = 1000

def distilled_rate(r):
    """(actions, rate) of the distilled policy over a logged record; rate is
    None where it defers to the search: >1 distinct rate, max_c bounded, and
    the highest rate equals the queue cap's (min_c + cap - max_q)."""
    acts = distilled_actions(r["pf"])
    d = {}
    for a, x in r["cands"]:
        d.setdefault(a, x)
    rates = sorted(set(d[a] for a in acts if a in d))
    if not rates:
        return acts, None
    top = rates[-1]
    B = r["B"]; cap = r["policy"]["queue_cap"]
    cap_bound = cap is not None and top == max(F(0), B["min_c"] + F(cap) - B["max_q"])
    if len(rates) > 1 and cap_bound and B["max_c"] < LARGEST_BW:
        return acts, None
    return acts, top
