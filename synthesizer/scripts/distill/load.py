"""Load --decisions traces (one JSON line per RTT) into flat rows."""
import json, glob, os
from fractions import Fraction as F

ORDER = ["MaxAllowedRate", "HoldQueue", "ProbeAboveLimit", "DrainQueue", "MatchDelivery", "AboveMaxC", "ProbeOnePacket", "ProbeDelivery"]

def q(x):
    return None if x is None else F(x)

def load(dirs):
    rows = []
    for d in dirs:
        for path in sorted(glob.glob(os.path.join(d, "dec", "*.jsonl"))):
            run = os.path.basename(path)[:-6]
            for line in open(path):
                r = json.loads(line)
                r["run"] = run
                r["src"] = d
                r["cands"] = [(a, F(x)) for a, x in r["candidates"]]
                r["roots"] = [F(x) for x in r["root_rates"]]
                r["chosen"] = F(r["chosen_rate"])
                r["opt"] = set(F(x) for x in r["optimal_rates"])
                r["B"] = {k: q(v) for k, v in r["belief"].items()}
                rows.append(r)
    return rows

def label_of(r, rate):
    """The action label the engine records: the first candidate with that rate."""
    for a, x in r["cands"]:
        if x == rate:
            return a
    return None
