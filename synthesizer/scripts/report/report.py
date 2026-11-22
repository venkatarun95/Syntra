"""Markdown report of CBR-delay CCA simulation runs, with the belief per RTT.

usage: report.py OUT.md TITLE LABEL=RUN_DIR [LABEL=RUN_DIR ...] [--rtts]

Each RUN_DIR holds `<scenario>/obj_*/*.csv` (as written by
`scripts/distill/run.sh` or `--out`).  The summary compares every label on
each scenario; `--rtts` adds, per scenario and label, a table of every RTT:
mode, action, rate, sent, delivered, loss, and the belief on C, B and the
queue (with the true C and B).
"""
import csv, glob, os, statistics, sys


def step_secs(x):
    return float(x[:-2]) / 1000 if x.endswith('ms') else float(x[:-1])


def load(run_dir):
    runs = {}
    for path in sorted(glob.glob(os.path.join(run_dir, '*', 'obj_*', '*.csv'))):
        name = path[len(run_dir):].strip('/').split('/')[0]
        runs[name] = list(csv.DictReader(open(path)))
    return runs


def stats(rows, warm=10):
    f = lambda r, k: float(r[k])
    n = len(rows)
    cap = sum(f(r, 'true_c') for r in rows[1:])
    cap_w = sum(f(r, 'true_c') for r in rows[warm + 1:])
    steps = [step_secs(r['time']) for r in rows if r['searched'] == 'true']
    return {
        'rtts': n,
        'util': (f(rows[-1], 's') - f(rows[0], 's')) / cap if cap else 0,
        'util_w': (f(rows[-1], 's') - f(rows[warm], 's')) / cap_w if cap_w else 0,
        'loss': f(rows[-1], 'ld'),
        'restarts': int(rows[-1]['restarts']),
        'maxq_bdp': sum(f(r, 'max_q') / f(r, 'true_c') for r in rows) / n,
        'searched': len(steps),
        'step_med': statistics.median(steps) if steps else 0,
        'step_max': max(steps) if steps else 0,
    }


def fmt_c(x):
    x = float(x)
    return 'inf' if x >= 1000 else f'{x:.1f}'


def rtt_table(rows):
    out = ['| t | C | B | mode | action | rate | sent | deliv | loss | C belief | B belief | q belief | K |',
           '|---|---|---|---|---|---|---|---|---|---|---|---|---|']
    prev_s = prev_ld = 0.0
    prev_rs = 0
    for i, r in enumerate(rows):
        s, ld, rs = float(r['s']), float(r['ld']), int(r['restarts'])
        mark = ' **restart**' if rs > prev_rs else ''
        out.append(f"| {i} | {float(r['true_c']):g} | {float(r['true_b']):g} | {r['mode']}{mark} | {r['action']} | "
                   f"{float(r['r']):.1f} | {float(r['sent']):.1f} | {s - prev_s:.1f} | {ld - prev_ld:.1f} | "
                   f"[{fmt_c(r['min_c'])}, {fmt_c(r['max_c'])}] | [{fmt_c(r['min_b'])}, {fmt_c(r['max_b'])}] | "
                   f"[{float(r['min_q']):.1f}, {float(r['max_q']):.1f}] | {r['qe_ks'].replace('|', ',')} |")
        prev_s, prev_ld, prev_rs = s, ld, rs
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    out_path, title, sets = args[0], args[1], [a.split('=', 1) for a in args[2:]]
    data = {label: load(d) for label, d in sets}
    names = sorted({n for runs in data.values() for n in runs})
    md = [f'# {title}', '',
          'Utilization = delivered / sum of the true C (`util10`: from RTT 10 on).  `maxq` = mean belief '
          '`max_q` in BDPs (true C).  Step time over the RTTs where the search ran.', '',
          '| scenario | run | RTTs | util | util10 | loss | restarts | maxq | searched | step median | step max |',
          '|---|---|---|---|---|---|---|---|---|---|---|']
    for n in names:
        for label, _ in sets:
            rows = data[label].get(n)
            if not rows:
                continue
            s = stats(rows)
            md.append(f"| {n} | {label} | {s['rtts']} | {s['util']:.2f} | {s['util_w']:.2f} | {s['loss']:.0f} | "
                      f"{s['restarts']} | {s['maxq_bdp']:.2f} | {s['searched']} | {s['step_med']:.2f} s | {s['step_max']:.1f} s |")
    if '--rtts' in sys.argv:
        md += ['', '## Belief per RTT', '',
               'C, B: the true link.  rate: what the CCA chose; sent: what the application let it send; deliv, '
               'loss: delivered and newly lost this RTT.  Beliefs are `[min, max]` as the CCA chose the rate (from the observations up to that RTT); K: the burstiness '
               'hypotheses the belief assumes.']
        for n in names:
            for label, _ in sets:
                rows = data[label].get(n)
                if rows:
                    md += ['', f'### {n} ({label})', ''] + rtt_table(rows)
    open(out_path, 'w').write('\n'.join(md) + '\n')


main()
