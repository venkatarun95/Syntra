#!/bin/sh
cd /home/venkat/Projects/Syntra/.claude/worktrees/cca-applimited-beliefs/synthesizer
out=$1; shift; mkdir -p $out/logs
cat <<X | xargs -P 4 -I{} sh -c 'set -- {}; name=$1; shift; SYNTRA_LOG=info timeout 5400 ./target/release/cbrdelay-cca --sim --qe-model bursty --steps 100 --csv --out '"$out"'/$name --decisions '"$out"'/$name.jsonl "$@" '"$*"' > '"$out"'/logs/$name.log 2>&1 || echo FAILED $name'
bigbuf  --sim-c 100 --sim-b 400 --sim-k 1 --sim-policy max
smallbuf --sim-c 40 --sim-b 20 --sim-k 1 --sim-policy random --seed 7
bursty  --sim-c 50 --sim-b 100 --sim-k 4 --sim-policy random --seed 2
cupdown --sim-c 20 --sim-b 40 --sim-k 1 --sim-policy max --sim-c-schedule 30:50,65:25
brise   --sim-c 20 --sim-b 10 --sim-k 1 --sim-policy max --sim-b-schedule 30:60
onoff   --sim-c 20 --sim-b 40 --sim-k 1 --sim-policy max --app onoff:10:10
cdrop_bursty --sim-c 60 --sim-b 60 --sim-k 4 --sim-policy mixed --seed 5 --sim-c-schedule 40:30
X
echo all done
