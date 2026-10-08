"""Convergence and rate-tracking metrics for cbrdelay-cca CSVs.

    python3 scripts/convergence_cca.py RUN_DIR [RUN_DIR ...]

Each RUN_DIR holds obj_*/<sim tag>.csv (one run).  Per run it prints:

* tight: first RTT at which the belief holds the true C and is tight
  (max_c <= 1.25 min_c + 2), and again after every change in `true_c`;
* util: delivered / true C over the run (and per segment between changes);
* loss: packets lost / packets sent;
* restarts: belief restarts (`--change-restart`).
"""
import glob
import os
import sys

import pandas as pd


def load(run_dir):
    paths = glob.glob(os.path.join(run_dir, "obj_*", "*.csv"))
    if not paths:
        return None
    d = pd.read_csv(paths[0])
    d["delivered"] = d.s.diff().fillna(d.s)
    d["lost"] = d.ld.diff().fillna(d.ld)
    if "true_c" not in d:
        d["true_c"] = float("nan")
    return d


def tight_at(d, start, end):
    seg = d.iloc[start:end]
    ok = seg[(seg.min_c <= seg.true_c + 1) & (seg.max_c >= seg.true_c - 1)
             & (seg.max_c <= 1.25 * seg.min_c + 2)]
    return None if ok.empty else int(ok.index[0]) - start


def summarize(run_dir):
    d = load(run_dir)
    if d is None:
        return None
    changes = [0] + [i for i in range(1, len(d)) if d.true_c.iloc[i] != d.true_c.iloc[i - 1]] + [len(d)]
    segs = []
    for a, b in zip(changes, changes[1:]):
        seg = d.iloc[a:b]
        segs.append({
            "from": a, "c": seg.true_c.iloc[0], "tight_after": tight_at(d, a, b),
            # Skip the first RTT of a segment: its delivery still reflects the old rate
            "util": seg.delivered.iloc[1:].sum() / seg.true_c.iloc[1:].sum() if len(seg) > 1 else float("nan"),
            "loss": seg.lost.sum(),
        })
    return {
        "run": os.path.basename(run_dir.rstrip("/")),
        "rtts": len(d),
        "util": d.delivered.sum() / d.true_c.sum(),
        "loss_pct": 100 * d.lost.sum() / max(d.r.sum(), 1),
        "restarts": int(d.restarts.iloc[-1]) if "restarts" in d else 0,
        "segments": segs,
        "searched": int((d.candidates > 1).sum()),
    }


def main():
    rows = [r for r in (summarize(p) for p in sys.argv[1:]) if r]
    for r in sorted(rows, key=lambda r: r["run"]):
        segs = "  ".join(f"[t{s['from']} C={s['c']:.0f}: tight+{s['tight_after']} util={s['util']:.2f} lost={s['loss']:.0f}]"
                         for s in r["segments"])
        print(f"{r['run']:28s} rtts={r['rtts']:3d} util={r['util']:.2f} loss={r['loss_pct']:.2f}% "
              f"restarts={r['restarts']} searched={r['searched']}  {segs}")


if __name__ == "__main__":
    main()
