#!/bin/sh
# usage: run.sh OUT SCENARIO_FILE [extra args]
# SCENARIO_FILE lines: NAME ARGS...   (cbrdelay-cca args, without --steps/--out)
# Writes OUT/<NAME>/obj_*/<tag>.csv, OUT/dec/<NAME>.jsonl, OUT/logs/<NAME>.log
out=$1; scen=$2; shift 2
bin=${CCA_BIN:-./target/release/cbrdelay-cca}
mkdir -p $out/logs $out/dec
grep -v '^#' $scen | grep -v '^ *$' | xargs -P ${JOBS:-8} -I{} sh -c 'set -- {}; name=$1; shift; SYNTRA_LOG=info timeout 3600 '"$bin"' --sim --qe-model bursty --steps ${STEPS:-70} --csv --out '"$out"'/$name --decisions '"$out"'/dec/$name.jsonl "$@" '"$*"' > '"$out"'/logs/$name.log 2>&1 || echo FAILED $name'
