"""Fit a decision tree from the CCA's decision-time features to the action it
took, and score it with the any-optimal criterion: train and leave-one-run-out.
Usage: uv run --with scikit-learn python tree.py TRAIN_DIR [TEST_DIR]"""
import sys
import numpy as np
from sklearn.tree import DecisionTreeClassifier, export_text
from load import load, label_of, F

MODES = ["Explore", "Cruise", "Probe", "Drain", "RiseCheck", "Recover"]
ACTS = ["MaxAllowedRate", "HoldQueue", "ProbeAboveLimit", "AboveMaxC", "MatchDelivery", "DrainQueue"]

def feats(r):
    pf, B, pl = r["pf"], r["B"], r["planner"]
    mc = B["min_c"] if B["min_c"] > 0 else F(1)
    sp = pf["since_probe"]
    d = {}
    for a, x in r["cands"]:
        d.setdefault(a, x)
    f = {
        "c_ratio": float(B["max_c"] / mc), "maxb_bdp": float(B["max_b"] / mc), "minb_bdp": float(B["min_b"] / mc),
        "maxq_bdp": float(B["max_q"] / mc), "mincb_bdp": float(B["min_cb"] / mc), "min_c": float(B["min_c"]),
        "since_start": pf["since_start"], "since_probe": 999 if sp is None else sp,
        "k_max": float(F(pf["k_max"])), "app_limited": pl["app_limited"],
        "probe_due": pf.get("probe_due", sp is None or sp >= 10),
    }
    for k in ["newest_window_feasible", "c_tight", "small_buffer", "k_unconfirmed", "cautious", "drained", "drain_due",
              "drain_gave_up", "rise_continues", "rise_due", "rise_fits", "rise_lossless", "queue_high", "probe_ok"]:
        f[k] = pf[k]
    for m in MODES:
        f["prev_" + m] = pf["prev_mode"] == m
    # Candidate rate comparisons (what the CCA computes from its formulas).
    M, H = d.get("MaxAllowedRate"), d.get("HoldQueue")
    P = d.get("ProbeAboveLimit")
    f["M_ge_H"] = M is not None and H is not None and M >= H
    f["P_ge_H"] = P is not None and H is not None and P >= H
    return f

def lab(r):
    return label_of(r, r["chosen"])

def opt_ok(r, label):
    for a, x in r["cands"]:
        if a == label:
            return x in r["opt"]
    return False

def matrix(rows, names=None):
    fs = [feats(r) for r in rows]
    names = names or list(fs[0].keys())
    return np.array([[float(f[n]) for n in names] for f in fs]), names

train = load([sys.argv[1]])
X, names = matrix(train)
y = np.array([lab(r) for r in train])
clf = DecisionTreeClassifier(random_state=0).fit(X, y)
pred = clf.predict(X)
print(f"tree: {clf.get_n_leaves()} leaves, depth {clf.get_depth()}")
print(f"train exact-label {np.mean(pred == y):.4f}, any-optimal {np.mean([opt_ok(r, p) for r, p in zip(train, pred)]):.4f} ({sum(opt_ok(r, p) for r, p in zip(train, pred))}/{len(train)})")
print(export_text(clf, feature_names=names))
runs = sorted(set(r["run"] for r in train))
ok = tot = 0
for run in runs:
    tr = [i for i, r in enumerate(train) if r["run"] != run]
    te = [i for i, r in enumerate(train) if r["run"] == run]
    c = DecisionTreeClassifier(random_state=0).fit(X[tr], y[tr])
    p = c.predict(X[te])
    k = sum(opt_ok(train[i], pi) for i, pi in zip(te, p))
    ok += k; tot += len(te)
    if k < len(te):
        print(f"  LORO {run}: {k}/{len(te)}")
print(f"tree LORO any-optimal: {ok}/{tot} = {ok/tot:.4f}")
if len(sys.argv) > 2:
    test = load([sys.argv[2]])
    Xt, _ = matrix(test, names)
    p = clf.predict(Xt)
    k = sum(opt_ok(r, pi) for r, pi in zip(test, p))
    print(f"tree on new runs any-optimal: {k}/{len(test)}")
