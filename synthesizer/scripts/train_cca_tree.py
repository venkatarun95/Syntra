#!/usr/bin/env python3
"""Fit a small decision tree to the CCA's per-RTT choices.

Usage: python3 scripts/train_cca_tree.py [--label action|mode|rate]
           [--searched-only] [--depth N] [--min-leaf N] DIR [DIR ...]

Needs scikit-learn and pandas (`uv pip install scikit-learn pandas`).

Reads every cbrdelay-cca CSV below each DIR.  Row t of a CSV holds the
belief *after* RTT t's observation, but the decision of RTT t was made from
the belief after RTT t-1, so the features of decision t come from row t-1
and the label from row t.  Features are mostly scale-free (ratios to
`min_c`) so that one tree can cover links of different capacity.

`--searched-only` keeps the RTTs where minimax had more than one candidate
action (the CSV's `candidates` column); every other RTT is the planner's
forced choice, which the tree would just relearn from `state_policy`.

`--label rate` regresses the rate as a multiple of `min_c`.  Accuracy is
reported on the training data and by leave-one-run-out cross-validation,
which is what tells whether the rules generalize across links rather than
memorize one trace.
"""
import argparse
import glob
import os
import sys

import numpy as np
import pandas as pd
from sklearn.model_selection import LeaveOneGroupOut, cross_val_predict
from sklearn.tree import DecisionTreeClassifier, DecisionTreeRegressor, export_text

ACTIONS = ["MaxAllowedRate", "DrainQueue", "ProbeAboveLimit", "HoldQueue"]


def load_samples(path):
    df = pd.read_csv(path)
    if len(df) < 2:
        return None
    prev = df.shift(1)  # the belief each decision was made from
    min_c = prev["min_c"].clip(lower=1e-9)
    ks = prev["qe_ks"].fillna("").astype(str).str.split("|")
    x = pd.DataFrame({
        "min_c": prev["min_c"],
        "c_ratio": prev["max_c"] / min_c,
        "min_b_bdp": prev["min_b"] / min_c,
        "max_b_bdp": (prev["max_b"] / min_c).clip(upper=1e3),
        "min_q_bdp": prev["min_q"] / min_c,
        "max_q_bdp": prev["max_q"] / min_c,
        "k_max": ks.map(lambda v: max((int(k) for k in v if k), default=0)),
        "n_k": ks.map(lambda v: sum(1 for k in v if k)),
        "prev_rate_bdp": prev["r"] / min_c,
        "prev_loss": prev["ld"].diff(),
        "prev_qdel": prev["qdel"],
        "prev_app_limited": prev["app_limited"].astype(str).str.lower().eq("true").astype(float),
    })
    for a in ACTIONS:
        x[f"prev_{a}"] = prev["action"].eq(a).astype(float)
    out = pd.DataFrame({
        "action": df["action"],
        "mode": df["mode"],
        "rate": df["r"] / min_c,
        "searched": df["candidates"] > 1 if "candidates" in df else True,
        "run": path,
    })
    return pd.concat([x, out], axis=1).iloc[2:]  # prev_loss needs two rows


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("dirs", nargs="+")
    ap.add_argument("--label", choices=["action", "mode", "rate"], default="action")
    ap.add_argument("--searched-only", action="store_true")
    ap.add_argument("--depth", type=int, default=4)
    ap.add_argument("--min-leaf", type=int, default=5)
    args = ap.parse_args()

    paths = sorted(p for d in args.dirs for p in glob.glob(os.path.join(d, "**", "*.csv"), recursive=True))
    frames = [f for f in map(load_samples, paths) if f is not None]
    if not frames:
        sys.exit("no samples")
    data = pd.concat(frames, ignore_index=True)
    if args.searched_only:
        data = data[data["searched"]]
    features = [c for c in data.columns if c not in ("action", "mode", "rate", "searched", "run")]
    X, y, groups = data[features].fillna(0).to_numpy(), data[args.label], data["run"]

    regression = args.label == "rate"
    Model = DecisionTreeRegressor if regression else DecisionTreeClassifier
    model = lambda: Model(max_depth=args.depth, min_samples_leaf=args.min_leaf, random_state=0)
    tree = model().fit(X, y)

    print(f"{len(data)} decisions from {groups.nunique()} runs; label={args.label}"
          f"{' (searched RTTs only)' if args.searched_only else ''}")
    if not regression:
        print("label counts:", y.value_counts().to_dict())
    print()
    print(export_text(tree, feature_names=features, decimals=2, show_weights=not regression))
    metric = "RMSE" if regression else "accuracy"
    score = lambda pred: (np.sqrt(np.mean((pred - y) ** 2)) if regression else np.mean(pred == y))
    print(f"train {metric}: {score(tree.predict(X)):.3f}")
    if groups.nunique() > 1:
        cv = cross_val_predict(model(), X, y, groups=groups, cv=LeaveOneGroupOut())
        print(f"leave-one-run-out {metric}: {score(cv):.3f}")
        if not regression:
            print(f"majority-class baseline: {y.value_counts(normalize=True).iloc[0]:.3f}")
    imp = sorted(zip(tree.feature_importances_, features), reverse=True)
    print("importance:", ", ".join(f"{f} {v:.2f}" for v, f in imp if v > 0))


if __name__ == "__main__":
    main()
