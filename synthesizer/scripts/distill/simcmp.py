"""Per-run utilization / loss / queue for CSV trees, side by side.
Usage: simcmp.py LABEL=DIR [LABEL=DIR ...]
util = delivered / sum(true_c) over RTTs 10..end; loss = packets lost (final ld);
unacked = mean (a - s - ld) / true_c over RTTs 10..end (in BDPs, includes the
one-BDP delay box); maxq = mean of the belief's max_q / true_c."""
import sys, csv, glob, os
dirs = [a.split("=", 1) for a in sys.argv[1:]]
def stats(p):
    rows = list(csv.DictReader(open(p)))
    f = lambda r, k: float(r[k])
    lo = 10
    c = sum(f(r, "true_c") for r in rows[lo:])
    util = (f(rows[-1], "s") - f(rows[lo - 1], "s")) / c
    loss = f(rows[-1], "ld")
    un = sum((f(r, "a") - f(r, "s") - f(r, "ld")) / f(r, "true_c") for r in rows[lo:]) / len(rows[lo:])
    mq = sum(f(r, "max_q") / f(r, "true_c") for r in rows[lo:]) / len(rows[lo:])
    return len(rows), util, loss, un, mq
runs = sorted(set(os.path.relpath(p, d).split("/")[0] for _, d in dirs for p in glob.glob(os.path.join(d, "*", "obj_*", "*.csv"))))
print("run".ljust(6) + "".join(f"| {l}: n util loss unacked maxq ".ljust(44) for l, _ in dirs))
tot = {l: [0, 0, 0, 0, 0] for l, _ in dirs}
for run in runs:
    line = run.ljust(6)
    for l, d in dirs:
        ps = glob.glob(os.path.join(d, run, "obj_*", "*.csv"))
        if not ps:
            line += "| -".ljust(44); continue
        n, u, lo, un, mq = stats(ps[0])
        t = tot[l]; t[0] += 1; t[1] += u; t[2] += lo; t[3] += un; t[4] += mq
        line += f"| {n:3d} {u:5.2f} {lo:7.1f} {un:5.2f} {mq:5.2f}".ljust(44)
    print(line)
line = "mean".ljust(6)
for l, _ in dirs:
    t = tot[l]; k = max(t[0], 1)
    line += f"| {t[0]:3d} {t[1]/k:5.2f} {t[2]:7.1f}(sum) {t[3]/k:5.2f} {t[4]/k:5.2f}".ljust(44)
print(line)
