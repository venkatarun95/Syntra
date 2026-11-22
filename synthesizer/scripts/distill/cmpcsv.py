"""Compare two CSV trees (same relative paths) row by row, ignoring time/searched/rule/candidates."""
import sys, csv, glob, os
IGN = {"time", "searched", "rule", "candidates"}
a, b = sys.argv[1], sys.argv[2]
tot = same = 0
for pa in sorted(glob.glob(os.path.join(a, "*", "obj_*", "*.csv"))):
    rel = os.path.relpath(pa, a)
    pb = os.path.join(b, rel)
    if not os.path.exists(pb):
        print("missing", rel); continue
    ra = list(csv.DictReader(open(pa))); rb = list(csv.DictReader(open(pb)))
    first = None
    for i, (x, y) in enumerate(zip(ra, rb)):
        d = [k for k in x if k not in IGN and x[k] != y.get(k)]
        if d and first is None:
            first = (i, d[:6], {k: (x[k], y[k]) for k in d[:3]})
    tot += 1
    ok = first is None and len(ra) == len(rb)
    same += ok
    print(("SAME " if ok else "DIFF ") + rel.split("/")[0], len(ra), len(rb), "" if ok else first)
print(f"{same}/{tot} identical")
