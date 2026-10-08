#!/bin/sh
# Run the CBR-delay CCA over a grid of simulated links, in parallel.
#
# Usage (from synthesizer/):
#   sh scripts/cca_sweep.sh OUT_DIR STEPS "C,B,SIM_K,POLICY[,SEED[,APP]] ..." [extra cbrdelay-cca args]
# APP is a --app value, e.g. onoff:10:10 or rate:10 (default backlogged).
#
# Each scenario writes OUT_DIR/obj_*/<sim tag>.csv and OUT_DIR/logs/<name>.log.
# Summarize with: python3 scripts/summarize_cca.py OUT_DIR
set -eu
out=$1
steps=$2
scenarios=$3
shift 3
bin=${CCA_BIN:-./target/release/cbrdelay-cca}
jobs=${JOBS:-8}
mkdir -p "$out/logs"
for sc in $scenarios; do
    echo "$sc"
done | xargs -P "$jobs" -I{} sh -c '
    IFS=, read -r c b k policy seed app <<EOF
{}
EOF
    seed=${seed:-1}
    app=${app:-backlogged}
    steps=$1
    out=$2
    shift 2
    name="c${c}_b${b}_k${k}_${policy}_s${seed}_$(echo "$app" | tr : -)"
    SYNTRA_LOG=${SYNTRA_LOG:-info} timeout ${CCA_TIMEOUT:-3600} "$0" --sim --sim-c "$c" --sim-b "$b" --sim-k "$k" \
        --sim-policy "$policy" --seed "$seed" --app "$app" --steps "$steps" --csv --out "$out" --qe-model bursty "$@" \
        > "$out/logs/$name.log" 2>&1 || echo "FAILED: $name (exit $?)"
' "$bin" "$steps" "$out" "$@"
