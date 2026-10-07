#!/bin/sh
# Regenerate synthesizer/network_model_bursty_qe_output/src/lib.rs from every
# bursty CBR-delay QE result that exists, at all trace lengths.  The Rust
# model picks the largest T available for each quantity at runtime.
#
# Transpiling writes .smt2.z3 debug dumps next to its inputs, so it reads
# from scratch copies and leaves the solved QE directories untouched.
#
# Run from qe/:  sh scripts/transpile_bursty.sh
set -eu
QE_DIR=$(cd "$(dirname "$0")/.." && pwd)
OUT=${OUT:-$QE_DIR/../synthesizer/network_model_bursty_qe_output/src/lib.rs}
PYTHON=${PYTHON:-$QE_DIR/.venv/bin/python}

# T -> solved-results directory.  qe/bursty_cbrdelay_T6/ holds the T=5 run.
T6=$QE_DIR/outputs/qe/bursty_cbrdelay_T6
T5=$QE_DIR/bursty_cbrdelay_T6
T4=$QE_DIR/outputs/qe/bursty_cbrdelay_T4

SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
for t in 6 5 4; do
    eval src=\$T$t
    mkdir -p "$SCRATCH/T$t"
    if [ -d "$src" ]; then
        find "$src" -maxdepth 1 -name "T=$t,*" -exec cp {} "$SCRATCH/T$t/" \;
    fi
done

cd "$QE_DIR"
"$PYTHON" -m qe.main --mode transpile --config bursty_cbrdelay \
    -t 6 --input-dir "$SCRATCH/T6" \
    --extra-input "5=$SCRATCH/T5" \
    --extra-input "4=$SCRATCH/T4" \
    --output-path "$OUT"
