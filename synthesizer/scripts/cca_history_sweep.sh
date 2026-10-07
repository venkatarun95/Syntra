#!/bin/sh
# Run the standard scenario set at several belief-history lengths.
# Usage (from synthesizer/): sh scripts/cca_history_sweep.sh OUT_DIR STEPS "10 20 full"
set -eu
out=$1
steps=$2
lengths=$3
scenarios=${SCENARIOS:-"20,10,1,max 20,100,1,max 20,9,4,min 20,9,4,random,1 50,200,1,alternate 20,33,16,alternate"}
for n in $lengths; do
    if [ "$n" = full ]; then
        sh scripts/cca_sweep.sh "$out/full" "$steps" "$scenarios"
    else
        sh scripts/cca_sweep.sh "$out/bh$n" "$steps" "$scenarios" --belief-history "$n"
    fi
done
