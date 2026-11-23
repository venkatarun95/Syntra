"""Build the cc_sim report page for the minimax CCA (`MinimaxQe`) from a sweep directory written by
cc_sim's `experiments/minimax/run.py` (with `--ccas mmqe,reno,copa`).

    python3 build.py SWEEP_DIR OUT.html [SCENARIO ...]

Per run it reads `scenario.json`, `config.json` and `mmqe_trace.jsonl` (one JSON line per round:
packets sent, delivered and lost, RTT samples, units, and the engine's decision record with its
belief in units per round). Everything is converted to physical units: rates in Mbit/s, queues in
packets. Each round also gets the reason for its decision, from the planner's features, following
the distilled rule (`distilled_actions` in `cbrdelay_cca_config.rs`).
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
    """The decision's reason, from the planner's features (as the distilled rule decides)"""
    pf, b, pl = e["pf"], e["belief"], e["planner"]
    mbps = lambda units: units * u * pkt * 8 / round_s / 1e6
    minc, maxc = q(b["min_c"]), q(b["max_c"])
    cands = {a: q(x) for a, x in e["candidates"]}
    mode = e["mode"]
    if pf["since_start"] < 7:
        s = f"Belief restarted {pf['since_start']} rounds ago; a QE window needs 7 rounds, so only chained C bounds exist."
        if "ProbeDelivery" in cands:
            return s + f" Probe above the delivery rate ({mbps(cands['ProbeDelivery']):.3g} Mbit/s; a rise in C would stay hidden otherwise)."
        return s + " Send what the link delivered."
    if not pf["newest_window_feasible"]:
        return "No static link explains the newest window (the link is changing): hold the queue."
    if mode == "RiseCheck":
        return f"C tight; rise check: send 5/4 of max C ({mbps(maxc * 1.25):.3g} Mbit/s) for a few rounds to see whether C or B grew."
    if mode == "Drain":
        return f"Drain: no min-RTT sample for 40 rounds (or draining): hold the queue at its provable floor ({q(pf['queue_target']) * u:.0f} pkts)."
    ratio = maxc / minc if minc else float("inf")
    if mode == "Explore":
        if pf["queue_high"]:
            return f"C loose (max/min = {ratio:.2f} > 1.25), but the worst-case queue exceeds the Explore cap (2 BDP): hold it there."
        opts = ", ".join(f"{a} {mbps(x):.3g}" for a, x in cands.items())
        probe = "" if pf["probe_ok"] else f" (no probe: last one {pf['since_probe']} rounds ago, gap {pf['probe_gap']})"
        return f"C loose (max/min = {'inf' if maxc >= 1000 else f'{ratio:.2f}'} > 1.25): send the highest candidate [{opts} Mbit/s]{probe}."
    if mode == "Probe":
        return f"C tight; {pf['since_probe']} rounds since the last probe: one round at the no-loss rate plus a packet."
    return f"C tight (max/min = {ratio:.3f}); nothing due: hold the worst-case queue at {q(e['policy']['queue_target']) * u:.0f} pkts (Cruise)."


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


def summary(sweep):
    rows = list(csv.DictReader(open(os.path.join(sweep, "summary.csv"))))
    out = {}
    # After the warmup, or the whole run if it is shorter than the warmup
    have = {(r["scenario"], r["cca"]) for r in rows if r["window"] == "after_warmup"}
    for r in rows:
        if r["flow"] != "0" or r["window"] != ("after_warmup" if (r["scenario"], r["cca"]) in have else "all"):
            continue
        out.setdefault(r["scenario"], {})[r["cca"]] = {
            "util": float(r["link_util"]), "tput": float(r["tput_mbps"]), "loss": float(r["loss_pct"]),
            "q50": float(r["qdelay_p50_ms"]), "q95": float(r["qdelay_p95_ms"]), "group": r["group"]}
    return out


def main():
    sweep, out_html = sys.argv[1], sys.argv[2]
    pick = sys.argv[3:]
    summ = summary(sweep)
    table = []
    runs = []
    for name in sorted(summ):
        d = os.path.join(sweep, name, "mmqe")
        if not trace_path(d):
            continue
        sc = json.load(open(os.path.join(d, "scenario.json")))
        cfg = json.load(open(os.path.join(d, "config.json")))
        run = load_run(d, sc, cfg)
        static = sc["group"] in ("static", "rate_range")
        table.append({"name": name, "group": sc["group"], "res": summ[name], "restarts": run["restarts"],
                      "segments": run["segments"], "units": run["units"], "rounds_ms": run["rounds_ms"],
                      "static": static})
        if not pick or name in pick:
            run.update(key=name, title=name, setup=describe(sc, cfg), group=sc["group"],
                       stats=summ[name].get("mmqe"), others={k: v for k, v in summ[name].items() if k != "mmqe"})
            runs.append(run)
    if pick:
        runs.sort(key=lambda r: pick.index(r["key"]))
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
