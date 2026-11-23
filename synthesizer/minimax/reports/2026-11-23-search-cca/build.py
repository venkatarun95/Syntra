"""Build the cc_sim report page for the minimax CCA (`MinimaxQe`) from sweep directories written by
cc_sim's `experiments/minimax/run.py` (e.g. one `--ccas mmqe` sweep and one of the baselines, both
`--set focus`); their summaries are merged, and every scenario with an `mmqe` trace gets a panel.

    python3 build.py OUT.html SWEEP_DIR [SWEEP_DIR ...]

Per run it reads `scenario.json`, `config.json` and `mmqe_trace.jsonl` (one JSON line per round:
packets sent, delivered and lost, RTT samples, units, and the engine's decision record with its
belief in units per round). Everything is converted to physical units: rates in Mbit/s, queues in
packets. Each round also gets the reason for its decision: the planner's mode (which sets the
objective and the candidate actions) and what the search chose among them.
"""
import csv, json, math, os, sys
from fractions import Fraction as F

HERE = os.path.dirname(os.path.abspath(__file__))


def q(x):
    return float(F(x)) if x is not None else None


def capacity_fn(sc, cfg):
    """Packets the link can deliver in [t0, t1) (s), and a short description"""
    pkt = cfg["pkt_size"]
    link = cfg["topo"]["link"]
    if "Const" in link:
        r = link["Const"] / pkt
        return (lambda t0, t1: r * (t1 - t0)), "constant"
    if "Piecewise" in link:
        segs = [(rate / pkt, dur / 1e6) for rate, dur in link["Piecewise"]]
        period = sum(d for _, d in segs)

        def f(t0, t1):
            tot, t = 0.0, t0
            while t < t1 - 1e-12:
                tm = t % period
                acc = 0.0
                for r, d in segs:
                    if tm < acc + d:
                        step = min(t1 - t, acc + d - tm)
                        tot += r * step
                        t += step
                        break
                    acc += d
            return tot
        return f, "piecewise"
    if "Random" in link:
        lam = link["Random"]["Exponential"]  # opportunities per us
        return (lambda t0, t1: lam * 1e6 * (t1 - t0)), "Poisson (mean shown)"
    if "MahimahiFile" in link:
        ts = [float(x) / 1e3 for x in open(link["MahimahiFile"]) if x.strip()]
        period = max(ts[-1], 1e-3)
        import bisect

        def count(a, b):
            n = 0
            k0 = math.floor(a / period)
            k1 = math.floor(b / period)
            for k in range(k0, k1 + 1):
                lo = max(a - k * period, 0)
                hi = min(b - k * period, period)
                if hi > lo:
                    n += bisect.bisect_left(ts, hi) - bisect.bisect_left(ts, lo)
            return n
        return count, "Mahimahi trace"
    raise ValueError(link)


def why(e, u, round_s, pkt):
    """The decision's reason: the planner's mode (which sets the objective and the candidates) and
    the rate the search chose among them"""
    pf, b = e["pf"], e["belief"]
    mbps = lambda units: units * u * pkt * 8 / round_s / 1e6
    minc, maxc = q(b["min_c"]), q(b["max_c"])
    cands = {a: q(x) for a, x in e["candidates"]}
    opts = ", ".join(f"{a} {mbps(x):.3g}" for a, x in cands.items())
    chosen = [a for a, x in cands.items() if x == q(e["chosen_rate"])]
    pick = f" The search chose {chosen[0] if chosen else mbps(q(e['chosen_rate']))} among [{opts} Mbit/s]." if len(cands) > 1 else ""
    mode = e["mode"]
    budget = q(pf.get("loss_budget", 0)) * u
    if pf["since_start"] < 7:
        s = f"Belief restarted {pf['since_start']} rounds ago; a QE window needs 7 rounds, so only chained C bounds exist."
        if "ProbeDelivery" in cands:
            return s + f" Probe above the delivery rate ({mbps(cands['ProbeDelivery']):.3g} Mbit/s; a rise in C would stay hidden otherwise)."
        return s + " Send what the link delivered."
    if not pf["newest_window_feasible"]:
        return "No static link explains the newest window (the link is changing): drain the queue."
    ratio = maxc / minc if minc else float("inf")
    r = "inf" if maxc >= 1000 else f"{ratio:.2f}"
    if mode == "Drain":
        return f"Drain: no min-RTT sample for 40 rounds (or draining): prove the queue at most {q(pf['queue_target']) * u:.0f} pkts within the horizon, otherwise deliver." + pick
    if mode == "Probe":
        kind = "a BDP (C not yet tight since the belief started)" if not pf.get("c_known") else "the constant budget (C was tight before; max C expired)"
        return f"C loose (max/min = {r} > 1.25): learn C and B, losing at most {budget:.0f} pkts per 20 rounds, {kind}." + pick
    return f"C tight (max/min = {r}): maximize delivery, losing at most {budget:.0f} pkts per 20 rounds." + pick


def trace_path(d):
    """The run's trace (flow 0's, if it has several flows)"""
    for name in ("mmqe_trace.jsonl", "mmqe_trace0.jsonl"):
        if os.path.exists(os.path.join(d, name)):
            return os.path.join(d, name)
    return None


def load_run(d, sc, cfg):
    pkt = cfg["pkt_size"]
    cap, cap_kind = capacity_fn(sc, cfg)
    buf = cfg["topo"]["bufsize"].get("Finite")
    rows = [json.loads(l) for l in open(trace_path(d))]
    base = sc["rtt_ms"] / 1e3
    S = {k: [] for k in ["t", "rate", "deliv", "cap", "min_c", "max_c", "min_q", "max_q", "min_b", "max_b",
                         "qd_lo", "qd_hi", "loss", "mode", "action", "why", "restart", "seg", "unit", "round_ms",
                         "app_limited", "k"]}
    prev_lost, prev_seg, prev_rs = 0, 0, 0
    # Row j's engine record is the decision after round j, i.e. for round j + 1: each round is
    # shown with the record (belief, mode, reason) its rate was chosen with
    prev_e, prev_u = None, None
    for r in rows:
        if "replay_end" in r:
            # A rescale replayed the segment to a fresh engine: its last decision sets the next
            # round's rate
            prev_e, prev_u = r["engine"], r["pkts_per_unit"]
            continue
        e, prev_e = prev_e, r["engine"]
        u_dec, prev_u = prev_u, r["pkts_per_unit"]
        if e is None:
            continue
        u, R = u_dec, r["round_s"]
        to_mbps = lambda pk: pk * pkt * 8 / R / 1e6
        b = e["belief"]
        S["t"].append(round(r["t"], 4))
        S["rate"].append(to_mbps(r["sent"]))
        S["deliv"].append(to_mbps(r["delivered"]))
        # (sent and delivered are packets; the belief is in the units it was decided in)
        S["cap"].append(to_mbps(cap(r["t"], r["t"] + R)))
        S["min_c"].append(to_mbps(q(b["min_c"]) * u))
        S["max_c"].append(to_mbps(min(q(b["max_c"]), 999.0) * u))
        S["min_q"].append(q(b["min_q"]) * u)
        S["max_q"].append(q(b["max_q"]) * u)
        S["min_b"].append(q(b["min_b"]) * u)
        S["max_b"].append(min(q(b["max_b"]), 999.0) * u)
        S["qd_lo"].append(None if r["rtt_min"] is None else max(0, (r["rtt_min"] - base) * 1e3))
        S["qd_hi"].append(None if r["rtt_max"] is None else max(0, (r["rtt_max"] - base) * 1e3))
        seg = r["seg"]
        lost = r["cum_lost"]
        S["loss"].append(max(0, lost - (prev_lost if seg == prev_seg else 0)))
        rs = prev_e.get("restarts", 0)
        S["restart"].append(seg == prev_seg and rs > prev_rs)
        prev_lost, prev_seg, prev_rs = lost, seg, rs
        S["mode"].append(e["mode"])
        acts = [a for a, x in e["candidates"] if q(x) == q(e["chosen_rate"])]
        S["action"].append(acts[0] if acts else "")
        S["why"].append(why(e, u, R, pkt))
        S["seg"].append(seg)
        S["unit"].append(u)
        S["round_ms"].append(R * 1e3)
        S["app_limited"].append(r["app_limited"])
        S["k"].append(",".join(str(k) for k in e["planner"]["k_assumed"]) or "fixed")
    for k in ("rate", "deliv", "cap", "min_c", "max_c", "min_q", "max_q", "min_b", "max_b"):
        S[k] = [round(v, 4) if v is not None else None for v in S[k]]
    restarts = sum(S["restart"])
    return {"series": S, "buf_pkts": buf, "cap_kind": cap_kind, "pkt": pkt, "restarts": restarts,
            "segments": len(set(S["seg"])), "units": sorted(set(S["unit"])),
            "rounds_ms": sorted(set(round(x, 3) for x in S["round_ms"]))}


def summary(sweeps):
    """Whole-run results per scenario and CCA, merged over the sweep directories"""
    out = {}
    for sweep in sweeps:
        for r in csv.DictReader(open(os.path.join(sweep, "summary.csv"))):
            if r["flow"] != "0" or r["window"] != "all":
                continue
            out.setdefault(r["scenario"], {})[r["cca"]] = {
                "util": float(r["link_util"]), "tput": float(r["tput_mbps"]), "loss": float(r["loss_pct"]),
                "q50": float(r["qdelay_p50_ms"]), "q95": float(r["qdelay_p95_ms"]), "group": r["group"],
                "t90": float(r["t90_s"]) if r.get("t90_s") else None}
    return out


def main():
    out_html, sweeps = sys.argv[1], sys.argv[2:]
    summ = summary(sweeps)
    table = []
    runs = []
    order = ["static", "changing", "bursty", "rate_range", "cellular"]
    for name in sorted(summ, key=lambda n: (order.index(next(iter(summ[n].values()))["group"]), n)):
        d = next((os.path.join(sw, name, "mmqe") for sw in sweeps if trace_path(os.path.join(sw, name, "mmqe"))), None)
        if d is None:
            continue
        sc = json.load(open(os.path.join(d, "scenario.json")))
        cfg = json.load(open(os.path.join(d, "config.json")))
        run = load_run(d, sc, cfg)
        static = sc["group"] in ("static", "rate_range")
        table.append({"name": name, "group": sc["group"], "res": summ[name], "restarts": run["restarts"],
                      "segments": run["segments"], "units": run["units"], "rounds_ms": run["rounds_ms"],
                      "static": static})
        run.update(key=name, title=name.replace("focus_", ""), setup=describe(sc, cfg), group=sc["group"],
                   stats=summ[name].get("mmqe"), others={k: v for k, v in summ[name].items() if k != "mmqe"})
        runs.append(run)
    tpl = open(os.path.join(HERE, "page.tpl.html")).read()
    html = tpl.replace("/*DATA*/null", json.dumps(runs, separators=(",", ":"))).replace(
        "/*TABLE*/null", json.dumps(table, separators=(",", ":")))
    open(out_html, "w").write(html)
    print("wrote", out_html, len(runs), "runs,", len(table), "scenarios in the table")


def describe(sc, cfg):
    link = cfg["topo"]["link"]
    kind = next(iter(link))
    where = {"Const": "constant", "Piecewise": "piecewise-constant", "Random": "Poisson (exponential gaps)",
             "MahimahiFile": "Mahimahi trace " + os.path.basename(str(link.get("MahimahiFile", "")))}[kind]
    apps = sc.get("apps", ["Backlogged"])
    app = "" if apps == ["Backlogged"] else f"; app {json.dumps(apps[0])}"
    flows = "" if len(sc.get("starts", [0])) == 1 else f"; {len(sc['starts'])} flows"
    agg = f"; acks released at exponential gaps of {sc['agg_us'] / 1e3:g} ms" if sc.get("agg_us") else ""
    return (f"{where} link, nominal {sc['link_mbps']:g} Mbit/s, {sc['rtt_ms']} ms RTT, buffer {sc['buf_pkts']} packets "
            f"({sc['buf_bdp']} BDP), {cfg['pkt_size']} B packets, {sc['dur_s']} s{app}{flows}{agg}")


main()
