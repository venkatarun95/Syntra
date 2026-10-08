#!/usr/bin/env python3
"""Summarize cbrdelay-cca simulation CSVs compactly (one block per run).

Usage: python3 scripts/summarize_cca.py DIR [DIR ...]

Finds every CSV below each DIR.  The true C and B are read from the file
name (`..._c20_b10_...`).  The output is meant to be short enough to hand to
a smaller model for interpretation.
"""
import csv
import glob
import os
import re
import statistics
import sys

MODE_LETTER = {"Explore": "E", "Cruise": "c", "Probe": "P", "Drain": "D"}
ACTION_LETTER = {
    "MaxAllowedRate": "m",
    "DrainQueue": "d",
    "ProbeAboveLimit": "p",
    "HoldQueue": "h",
    "none": "?",
}


def runs(seq):
    """Run-length encode a sequence: [(value, count), ...]."""
    out = []
    for x in seq:
        if out and out[-1][0] == x:
            out[-1][1] += 1
        else:
            out.append([x, 1])
    return out


def rle_string(seq):
    return " ".join(f"{v}x{n}" if n > 1 else str(v) for v, n in runs(seq))


def summarize(path):
    rows = list(csv.DictReader(open(path)))
    if not rows:
        return f"{path}: empty"
    m = re.search(r"_c([0-9.]+)_b([0-9.]+)", os.path.basename(path))
    true_c, true_b = (float(m.group(1)), float(m.group(2))) if m else (None, None)
    f = lambda r, k: float(r[k])
    n = len(rows)
    ld = [f(r, "ld") for r in rows]
    s = [f(r, "s") for r in rows]
    rate = [f(r, "r") for r in rows]
    min_c = [f(r, "min_c") for r in rows]
    max_c = [f(r, "max_c") for r in rows]
    max_q = [f(r, "max_q") for r in rows]
    qdel = [int(r["qdel"]) for r in rows]
    modes = [r.get("mode", "?") for r in rows]
    actions = [r.get("action", "none") for r in rows]
    sims = [r.get("sim", "?") for r in rows]
    qe_ks = [r.get("qe_ks", "") for r in rows]

    out = [f"== {os.path.basename(path)}  (true C={true_c} B={true_b}, {n} RTTs)"]

    tight = next((i for i in range(n) if max_c[i] <= 1.25 * min_c[i]), None)
    sound = true_c is None or min_c[-1] - 1e-9 <= true_c <= max_c[-1] + 1e-9
    min_b, max_b = f(rows[-1], "min_b"), f(rows[-1], "max_b")
    b_sound = true_b is None or min_b - 1e-9 <= true_b <= max_b + 1e-9
    out.append(
        f"C: first within 25% at t={tight}; final [{min_c[-1]:.2f}, {max_c[-1]:.2f}]"
        f"{'' if sound else '  ** true C OUTSIDE final bound **'};"
        f" final B [{min_b:.1f}, {max_b:.1f}]"
        f"{'' if b_sound else '  ** true B OUTSIDE final bound **'}"
    )

    if true_c:
        util = (s[-1] - s[0]) / (true_c * (n - 1)) if n > 1 else 0
        half = n // 2
        util2 = (s[-1] - s[half]) / (true_c * (n - 1 - half)) if n - 1 > half else 0
        out.append(f"util: all {util:.2f}, last half {util2:.2f}")

    loss_steps = [i for i in range(1, n) if ld[i] > ld[i - 1] + 1e-9]
    gaps = [b - a for a, b in zip(loss_steps, loss_steps[1:])]
    after = [i for i in loss_steps if tight is not None and i > tight]
    after_gaps = [b - a for a, b in zip(after, after[1:])]
    out.append(
        f"loss: total {ld[-1]:.1f} pkts in {len(loss_steps)} RTTs with loss;"
        f" at t={loss_steps[:12]}{'...' if len(loss_steps) > 12 else ''};"
        f" gaps after C tight: min {min(after_gaps) if after_gaps else '-'}"
        f" median {statistics.median(after_gaps) if after_gaps else '-'}"
    )
    out.append(
        f"queue: belief max_q mean {statistics.mean(max_q):.1f} max {max(max_q):.1f};"
        f" observed qdel mean {statistics.mean(qdel):.2f} max {max(qdel)}"
        f"; last half max_q mean {statistics.mean(max_q[n // 2:]):.1f}"
    )

    drains = []
    i = 0
    while i < n:
        if modes[i] == "Drain":
            j = i
            while j < n and modes[j] == "Drain":
                j += 1
            drains.append((i, j - i, max_q[j - 1]))
            i = j
        else:
            i += 1
    out.append(
        "drains (start,len,max_q at end): "
        + (", ".join(f"({a},{b},{q:.1f})" for a, b, q in drains) or "none")
    )

    out.append("modes:   " + "".join(MODE_LETTER.get(x, "?") for x in modes))
    out.append("actions: " + "".join(ACTION_LETTER.get(x, "?") for x in actions))
    out.append("rate:    " + rle_string([round(x, 1) for x in rate]))
    if len(set(sims)) > 1:
        out.append("sim:     " + rle_string(sims))
    if any(qe_ks):
        out.append(
            "qe_ks:   " + rle_string(qe_ks) + f"; final alive {rows[-1].get('alive_ks', '')}"
        )
    times = []
    for r in rows:
        t = r["time"]
        if t.endswith("ms"):
            times.append(float(t[:-2]) / 1000)
        elif t.endswith("µs"):
            times.append(float(t[:-2]) / 1e6)
        elif t.endswith("s"):
            times.append(float(t[:-1]))
    if times:
        out.append(f"step time: mean {statistics.mean(times):.2f}s max {max(times):.2f}s")
    return "\n".join(out)


def main():
    paths = []
    for d in sys.argv[1:]:
        paths += sorted(glob.glob(os.path.join(d, "**", "*.csv"), recursive=True))
    for p in paths:
        print(summarize(p))
        print()


if __name__ == "__main__":
    main()
