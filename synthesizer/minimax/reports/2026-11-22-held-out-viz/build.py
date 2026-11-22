"""Build the CCA simulation page from the CSVs in /tmp/cca_viz/<scenario>/."""
import csv, glob, json, os, sys

ROOT = '/tmp/cca_viz'
SCEN = [
    ('bigbuf', 'Fast link, deep buffer', 'C = 100, B = 400 (4 BDP), smooth link (K = 1, serves as fast as allowed).',
     'Exponential start: the rate doubles about every two RTTs because a deep buffer makes each probe cheap.'),
    ('smallbuf', 'Shallow buffer, erratic link', 'C = 40, B = 20 (0.5 BDP), K = 1, the link withholds service at random.',
     'A buffer under one BDP makes every probe risk loss, so the CCA turns cautious and grows by about one buffer per two RTTs.'),
    ('bursty', 'Bursty link', 'C = 50, B = 100, bursts of up to K = 4 packets, random service.',
     'Burstiness blurs what each RTT proves about C, so the belief narrows slowly and K = 1 is never confirmed.'),
    ('cupdown', 'Capacity up, then down', 'C = 20 → 50 at RTT 30 → 25 at RTT 65, B = 40, K = 1.',
     'A rise is invisible until the CCA sends above its old max C; the periodic rise check finds it. The fall contradicts the belief on its own.'),
    ('brise', 'Buffer grows', 'C = 20, B = 10 → 60 at RTT 30, K = 1.',
     'With a small buffer the rise check expects loss; when none comes, the only explanation is that B or C grew, and the belief restarts.'),
    ('onoff', 'On/off application', 'C = 20, B = 40, K = 1; the application sends for 10 RTTs, then idles for 10.',
     'Idle RTTs teach nothing, so beliefs persist across gaps and the CCA resumes at the learned rate.'),
    ('cdrop_bursty', 'Capacity halves on a bursty link', 'C = 60 → 30 at RTT 40, B = 60, K = 4, mixed service.',
     'The drop makes the old belief unexplainable; the CCA restarts at the longest consistent prefix and relearns C.'),
]
CAP = 1000.0


def load(name):
    files = glob.glob(f'{ROOT}/v2_search/{name}/obj_*/*.csv')
    if not files:
        return None
    rows = list(csv.DictReader(open(files[0])))
    f = lambda r, k: float(r[k])
    out = {k: [] for k in ['t', 'rate', 'sent', 'min_c', 'max_c', 'true_c', 'true_b', 'min_b', 'max_b',
                           'min_q', 'max_q', 'loss', 'mode', 'action', 'restart', 'app_limited', 'min_rtt']}
    prev_ld, prev_restarts = 0.0, 0
    for i, r in enumerate(rows):
        out['t'].append(i)
        out['rate'].append(round(f(r, 'r'), 2))
        out['sent'].append(round(f(r, 'sent'), 2))
        out['min_c'].append(round(f(r, 'min_c'), 2))
        out['max_c'].append(round(min(f(r, 'max_c'), CAP), 2))
        out['true_c'].append(f(r, 'true_c'))
        out['true_b'].append(f(r, 'true_b'))
        out['min_b'].append(round(f(r, 'min_b'), 2))
        out['max_b'].append(round(min(f(r, 'max_b'), CAP), 2))
        out['min_q'].append(round(f(r, 'min_q'), 2))
        out['max_q'].append(round(f(r, 'max_q'), 2))
        ld = f(r, 'ld')
        out['loss'].append(round(ld - prev_ld, 2))
        prev_ld = ld
        out['mode'].append(r['mode'])
        out['action'].append(r['action'])
        rs = int(r['restarts'])
        out['restart'].append(rs > prev_restarts)
        prev_restarts = rs
        out['app_limited'].append(r['app_limited'] == 'true')
        out['min_rtt'].append(r['min_rtt_sample'] == 'true')
    n = len(rows)
    s = [f(r, 's') for r in rows]
    a = [f(r, 'a') for r in rows]
    delivered = s[-1] - s[0]
    capacity = sum(out['true_c'][1:])
    stats = {
        'rtts': n,
        'util': delivered / capacity if capacity else 0,
        'loss': prev_ld,
        'sent': a[-1] - a[0],
        'restarts': prev_restarts,
        'mean_q_bdp': sum(out['max_q'][i] / out['true_c'][i] for i in range(n)) / n,
    }
    return {'series': out, 'stats': stats}


def main():
    data = []
    for key, title, setup, story in SCEN:
        d = load(key)
        if d is None:
            print('missing', key, file=sys.stderr)
            continue
        d.update(key=key, title=title, setup=setup, story=story)
        data.append(d)
    extra = {}
    if os.path.exists(f'{ROOT}/extra.json'):
        extra = json.load(open(f'{ROOT}/extra.json'))
    tpl = open(f'{ROOT}/page.tpl.html').read()
    html = tpl.replace('/*DATA*/null', json.dumps(data, separators=(',', ':'))).replace(
        '/*EXTRA*/null', json.dumps(extra, separators=(',', ':')))
    open(f'{ROOT}/cca_sims.html', 'w').write(html)
    print('wrote', len(data), 'scenarios')
    for d in data:
        s = d['stats']
        print(f"{d['key']:13} util {s['util']:.2f} loss {s['loss']:.0f} restarts {s['restarts']} mean maxq {s['mean_q_bdp']:.2f} BDP")


main()
