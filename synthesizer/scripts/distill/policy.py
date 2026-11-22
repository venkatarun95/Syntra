"""The distilled policy in Python, over the logged features (mirror of
`distilled_actions` in cbrdelay_cca_config.rs)."""
from load import F
QE_WINDOW_OBSERVATIONS = 7

def distilled_actions(pf):
    if pf["since_start"] < QE_WINDOW_OBSERVATIONS:
        return ["MatchDelivery"]
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
        if pf["probe_ok"]:
            a.append("ProbeAboveLimit")
        return a
    if pf["rise_due"]:
        return ["AboveMaxC"]
    if pf["probe_due"]:
        return ["ProbeAboveLimit"]
    return ["HoldQueue"]

def distilled_rate(r):
    """Highest rate of the distilled actions, using the logged candidate rates."""
    acts = distilled_actions(r["pf"])
    d = {}
    for a, x in r["cands"]:
        d.setdefault(a, x)
    rates = [d[a] for a in acts if a in d]
    return acts, (max(rates) if rates else None)
