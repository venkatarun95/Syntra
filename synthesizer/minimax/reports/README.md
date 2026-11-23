# CBR-delay CCA reports

One folder per study, named by date and topic, each with a `NOTES.md` (what
was asked, what we found) and generated reports.  The development log is
`../CCA_DEVELOPMENT.md`; these folders hold the evidence it cites.

Generated reports come from `scripts/report/report.py` (run from
`synthesizer/`):

```bash
python3 scripts/report/report.py OUT.md "Title" LABEL=RUN_DIR [LABEL=RUN_DIR ...] --rtts
```

`RUN_DIR` holds `<scenario>/obj_*/*.csv`, as written by
`scripts/distill/run.sh` (or `cbrdelay-cca --csv --out`).  The summary
compares the labels per scenario; `--rtts` adds every RTT with the belief
on C, B and the queue.  Raw runs are not committed (they live under `/tmp`
when the report is made); the commands are in each `NOTES.md` or the
scenario files.

| folder | what |
| --- | --- |
| `2026-11-22-held-out-viz/` | Interactive page of 7 held-out links (search, depth 4), with the distilled rules explained; `build.py` + `page.tpl.html` build it from run CSVs |
| `2026-11-22-depth6-probes/` | Depth 6, the bursty large-buffer link, probe timing, Recover probes, one-packet probes, perfect fit |
| `2026-11-22-cc-sim/` | The distilled CCA over cc_sim's packet-level links (all scenarios) next to Reno and Copa; why it lost on cellular links |
| `2026-11-23-search-cca/` | The search CCA (a depth-6 search every RTT, per-mode objectives) on a focus set in cc_sim, next to Reno, Copa and the ack-window CCA |
