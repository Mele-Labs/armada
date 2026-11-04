#!/usr/bin/env python3
"""Spike 39: would a fresh session at each workflow step save tokens?

Reads this machine's Claude Code transcripts for this repository and a copy of Fleet's store, both
read-only, and prints aggregates only. Run under `nice`; one pass over the transcripts.

    nice python3 docs/spikes/039-run.py [--since 2026-10-21] [--store path/to/armada.db]

The store defaults to the newest file in Fleet's backups folder, opened immutable, so the live
database is never touched.
"""
import argparse, glob, json, os, re, sqlite3, statistics as st, subprocess, collections, time

ap = argparse.ArgumentParser()
ap.add_argument('--since', default='2026-10-21')
ap.add_argument('--store')
ap.add_argument('--manifest', default='armada')
args = ap.parse_args()
SINCE = args.since + 'T00:00:00'

# Weights from the token usage measurement: model price, then cache read 0.1, write by cache life,
# output 5. Hosted Jobs write the hour cache.
def price(m):
    m = (m or '').lower()
    for k, v in (('opus', 5.0), ('fable', 5.0), ('sonnet', 3.0), ('haiku', 1.0)):
        if k in m:
            return v
    return 3.0
WRITE = {'main': 2.0, 'subagent': 1.25}
TTL = {'main': 3600, 'subagent': 300}
START = 56_000        # a main thread's first request, measured
POST = 70_000         # what a compaction rewrites, as the earlier simulation used
SUMMARY_OUT = 6_000   # a compaction's own output

def projects():
    common = subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            capture_output=True, text=True, check=True).stdout.strip()
    repo = os.path.dirname(common)
    return os.path.expanduser('~/.claude/projects/') + re.sub(r'[^A-Za-z0-9]', '-', repo) + '*'

def store_path():
    if args.store:
        return args.store
    backups = sorted(glob.glob(os.path.expanduser('~/Library/Application Support/Armada/backups/*.db')))
    return backups[-1]

def stamp(ts):
    try:
        return time.mktime(time.strptime(ts[:19], '%Y-%m-%dT%H:%M:%S'))
    except Exception:
        return 0.0

MARKER = re.compile(r'Fleet: this Job follows the .+? workflow, and you are on its "(.+?)" step')

def text_of(c):
    if isinstance(c, str):
        return c
    if isinstance(c, list) and not any(isinstance(b, dict) and b.get('type') == 'tool_result' for b in c):
        return ''.join(b.get('text', '') for b in c if isinstance(b, dict))
    return None

def read_thread(paths):
    """One thread's requests in order, deduplicated by message id across copies of the file."""
    reqs, markers, seen = {}, [], set()
    for path in paths:
        with open(path, errors='replace') as f:
            for line in f:
                try:
                    o = json.loads(line)
                except Exception:
                    continue
                t, ts = o.get('type'), o.get('timestamp', '')
                if t == 'assistant':
                    m = o.get('message', {})
                    mid, u = m.get('id'), m.get('usage') or {}
                    if not mid or not u:
                        continue
                    r = reqs.setdefault(mid, {'id': mid, 'ts': ts, 'model': m.get('model'), 'tools': []})
                    r['u'] = u
                    for b in m.get('content') or []:
                        if isinstance(b, dict) and b.get('type') == 'tool_use':
                            r['tools'].append(b.get('name'))
                elif t == 'user' and o.get('uuid') not in seen:
                    seen.add(o.get('uuid'))
                    txt = text_of(o.get('message', {}).get('content'))
                    hit = MARKER.search(txt or '')
                    if hit:
                        markers.append((ts, hit.group(1)))
    seq = []
    for r in sorted(reqs.values(), key=lambda r: r['ts']):
        u = r['u']
        i, cr, cw = (u.get(k, 0) or 0 for k in ('input_tokens', 'cache_read_input_tokens', 'cache_creation_input_tokens'))
        o = u.get('output_tokens', 0) or 0
        if i + cr + cw == 0:
            continue
        seq.append({'id': r['id'], 'C': i + cr + cw, 'cr': cr, 'cw': cw, 'i': i, 'o': o, 'p': price(r['model']) / 1e6,
                    't': stamp(r['ts']), 'ts': r['ts'], 'tools': r['tools']})
    return seq, sorted(markers)

def cost(kind, r):
    return (r['cr'] * .1 + r['cw'] * WRITE[kind] + r['i'] + r['o'] * 5) * r['p']

def in_window(seq):
    return [r for r in seq if r['ts'] >= SINCE]

def unclaimed(seq, claimed):
    """A request is counted in the first thread that holds it: a fork or a copied file repeats its parent's."""
    own = [r for r in seq if r['id'] not in claimed]
    claimed.update(r['id'] for r in own)
    return own

def boundaries_at(seq, moves):
    """Request indexes a step move lands before: the first request at or after each move, past the first."""
    out = set()
    for ts, _ in moves:
        for n, r in enumerate(seq):
            if r['ts'] >= ts:
                if n > 0:
                    out.add(n)
                break
    return out

def moves_of(entries):
    """Entries `(time, step)` with a repeat of the step it stood on dropped; the first is the start."""
    out, last = [], object()
    for ts, step in entries:
        if step != last:
            out.append((ts, step))
        last = step
    return out[1:]

def sim(seq, cuts=(), window=None, brief=0, reread=0):
    """Weighted cost of one main thread, re-run from its own growth.

    A real compaction is followed where no smaller window already compacted. `window` compacts when the
    carried context passes it. At a cut the old session writes the brief, and the new one starts from
    start-up, the brief and the re-read, then grows as the real thread did.
    """
    tot, cur, prev, prevt = 0.0, None, None, None
    wr, ttl = WRITE['main'], TTL['main']
    for n, r in enumerate(seq):
        C, o, P, t = r['C'], r['o'], r['p'], r['t']
        if prev is None:
            cur = C
            tot += (C * wr + o * 5) * P
            prev, prevt = C, t
            continue
        delta = None if C < .6 * prev else C - prev
        cold = bool(t and prevt and t - prevt >= ttl)
        if n in cuts:
            tot += (cur * (1.0 if cold else .1) + brief * 5) * P
            cur = START + brief + reread + max(0, delta or 0)
            tot += (cur * wr + o * 5) * P
        elif delta is None:
            newc = min(cur, C) if window else C
            if window is None or newc == C:
                tot += (cur * .1 + o * 5) * P + newc * wr * P
            else:
                tot += (newc * .1 + o * 5) * P
            cur = newc
        else:
            newc = cur + max(0, delta)
            if window and newc > window:
                tot += (cur * (1.0 if cold else .1) + SUMMARY_OUT * 5) * P
                newc = POST
                tot += (newc * wr + o * 5) * P
            elif cold:
                tot += (newc * wr + o * 5) * P
            else:
                tot += (cur * .1 + max(0, delta) * wr + o * 5) * P
            cur = newc
        prev, prevt = C, t
    return tot

def main():
    db = sqlite3.connect('file:' + store_path() + '?mode=ro&immutable=1', uri=True)
    hosted = {sid for (sid,) in db.execute('SELECT session_id FROM hosted_sessions WHERE manifest_id = ?', (args.manifest,))}
    visits = {}
    for sid, raw in db.execute("SELECT session_id, visits FROM session_rail WHERE visits <> ''"):
        visits[sid] = [(ts, int(step)) for step, ts in (line.split('\t') for line in raw.splitlines() if '\t' in line)]
    person = collections.defaultdict(list)
    for sid, at in db.execute("SELECT session_id, json_extract(body, '$.at') FROM session_rows "
                              "WHERE json_extract(body, '$.kind') = 'message' AND json_extract(body, '$.from.kind') = 'you'"):
        person[sid].append(at)
    handoffs = [(new, old, len(json.loads(detail).get('context', '')) + len(json.loads(detail).get('brief', '')))
                for new, old, detail in db.execute("SELECT holder_id, target, detail FROM ledger_attachments WHERE kind = 'replaces'")]
    briefs_written = [len(json.loads(d).get('brief') or '') for (d,) in
                      db.execute("SELECT detail FROM ledger_attachments WHERE kind = 'handoff_starting'")]

    files = collections.defaultdict(list)
    subs = []
    for d in glob.glob(projects()):
        for p in glob.glob(d + '/*.jsonl'):
            files[os.path.basename(p)[:-6]].append(p)
        subs += glob.glob(d + '/*/subagents/*.jsonl')

    # Every request in the window, once, for the denominator.
    total = {'main': 0.0, 'subagent': 0.0}
    nreq = {'main': 0, 'subagent': 0}
    threads = {}
    claimed = set()
    read = {sid: read_thread(paths) for sid, paths in files.items()}
    for sid in sorted(read, key=lambda sid: read[sid][0][0]['ts'] if read[sid][0] else ''):
        seq, markers = read[sid]
        seq = unclaimed(seq, claimed)
        w = in_window(seq)
        total['main'] += sum(cost('main', r) for r in w)
        nreq['main'] += len(w)
        if sid in hosted and w:
            threads[sid] = (seq, markers)
    for p in subs:
        if os.path.getmtime(p) < stamp(SINCE):
            continue
        w = in_window(unclaimed(read_thread([p])[0], claimed))
        total['subagent'] += sum(cost('subagent', r) for r in w)
        nreq['subagent'] += len(w)
    everything = total['main'] + total['subagent']
    print(f"window from {args.since}: {nreq['main']} main requests, {nreq['subagent']} subagent requests")
    print(f"weighted spend: main {total['main']:.0f}, subagent {total['subagent']:.0f}, all {everything:.0f}")

    # Step boundaries, from the store's rail visits, else from Fleet's step messages in the transcript.
    chosen, source, cross = {}, collections.Counter(), []
    for sid, (seq, markers) in threads.items():
        w = in_window(seq)
        by_visit = boundaries_at(w, moves_of(visits.get(sid, [])))
        by_marker = boundaries_at(w, moves_of(markers))
        if sid in visits:
            chosen[sid] = by_visit
            source['visits'] += 1
            if markers:
                cross.append((len(by_visit), len(by_marker)))
        else:
            chosen[sid] = by_marker
            source['markers'] += 1
    multi = {sid for sid, cuts in chosen.items() if cuts}
    hosted_cost = sum(sum(cost('main', r) for r in in_window(threads[sid][0])) for sid in threads)
    multi_cost = sum(sum(cost('main', r) for r in in_window(threads[sid][0])) for sid in multi)
    print(f"hosted {args.manifest} Jobs with requests in the window: {len(threads)}, spend {hosted_cost:.0f} "
          f"({100 * hosted_cost / everything:.1f}% of all)")
    print(f"boundary source: {dict(source)}; Jobs with at least one boundary: {len(multi)}, "
          f"boundaries {sum(len(chosen[s]) for s in multi)}, spend {multi_cost:.0f}")
    split = collections.Counter('visits' if s in visits else 'markers' for s in multi for _ in chosen[s])
    print(f"boundaries by source: {dict(split)}")
    if cross:
        print(f"visits vs step messages on Jobs holding both: {sum(a for a, _ in cross)} vs {sum(b for _, b in cross)} boundaries over {len(cross)} Jobs")
    sizes = [len(in_window(threads[s][0])) for s in multi]
    gaps = []
    for s in multi:
        cuts = sorted(chosen[s]) + [len(in_window(threads[s][0]))]
        gaps += [b - a for a, b in zip([0] + cuts, cuts)]
    print(f"requests a Job: median {st.median(sizes)}; requests a step: median {st.median(gaps)}, mean {st.mean(gaps):.0f}")
    at_cut = [in_window(threads[s][0])[n - 1]['C'] for s in multi for n in chosen[s]]
    print(f"context carried across a boundary: median {st.median(at_cut):.0f}, mean {st.mean(at_cut):.0f}")

    brief_tok = st.mean(n / 3.6 for _, _, n in handoffs) if handoffs else 3000
    print(f"handoffs: {len(handoffs)}; what a new Job starts from (context and brief) {st.mean(n for *_, n in handoffs):.0f} chars, "
          f"{brief_tok:.0f} tokens; briefs the old Job wrote: {[n for n in briefs_written]} chars")

    # The simulation, over the Jobs with boundaries; the rest of the spend is unchanged.
    print('\nscenario | window | re-read | Jobs with steps | saving of their spend | of all spend')
    for window, wname in ((None, '400k (today)'), (250_000, '250k')):
        base = sum(sim(in_window(threads[s][0]), window=window) for s in multi)
        actual = multi_cost
        if window is None:
            print(f"calibration: simulated {base:.0f} vs actual {actual:.0f} ({100 * (base - actual) / actual:+.1f}%)")
        else:
            b400 = sum(sim(in_window(threads[s][0])) for s in multi)
            print(f"250k window alone on these Jobs | saves {100 * (b400 - base) / b400:.1f}% of their spend, "
                  f"{100 * (b400 - base) / everything:.1f}% of all")
        for reread in (5_000, 10_000, 20_000):
            fresh = sum(sim(in_window(threads[s][0]), cuts=chosen[s], window=window, brief=brief_tok, reread=reread) for s in multi)
            print(f"fresh per step | {wname} | {reread // 1000}k | {base:.0f} -> {fresh:.0f} | "
                  f"{100 * (base - fresh) / base:.1f}% | {100 * (base - fresh) / everything:.1f}%")
        variants = (
            ('brief 3k', lambda s, w: chosen[s], 3_000),
            ('only past 150k', lambda s, w: {n for n in chosen[s] if w[n - 1]['C'] >= 150_000}, brief_tok),
        )
        for name, pick, brief in variants:
            fresh = sum(sim(w, cuts=pick(s, w), window=window, brief=brief, reread=10_000)
                        for s in multi for w in [in_window(threads[s][0])])
            print(f"  {name} | {wname} | 10k | {base:.0f} -> {fresh:.0f} | "
                  f"{100 * (base - fresh) / base:.1f}% | {100 * (base - fresh) / everything:.1f}%")
        if window:
            fresh = sum(sim(in_window(threads[s][0]), cuts=chosen[s], window=window, brief=brief_tok, reread=10_000) for s in multi)
            print(f"both against 400k alone | 10k re-read | {100 * (b400 - fresh) / b400:.1f}% of their spend, "
                  f"{100 * (b400 - fresh) / everything:.1f}% of all")
    past = sum(1 for s in multi for n in chosen[s] if in_window(threads[s][0])[n - 1]['C'] >= 150_000)
    short = sum(1 for g in gaps if g < 3)
    print(f"boundaries carrying 150k or more: {past}; steps under 3 requests: {short} of {len(gaps)}")

    # The cost side: what a handed-off Job spent finding its place, against other Jobs' first 30 and
    # the 30 after a step boundary.
    def stretch(seq, at, n=30):
        part = seq[at:at + n]
        if len(part) < n:
            return None
        reads = sum(1 for r in part for t in r['tools'] if t in ('Read', 'Grep', 'Glob', 'Bash', 'Agent'))
        return {'grow': part[-1]['C'] - part[0]['C'], 'start': part[0]['C'],
                'cost': sum(cost('main', r) for r in part), 'calls': reads}
    def show(label, rows):
        rows = [r for r in rows if r]
        if not rows:
            print(f"{label}: none")
            return
        print(f"{label}: n {len(rows)}, starts at {st.median(r['start'] for r in rows):.0f}, grows {st.median(r['grow'] for r in rows):.0f} "
              f"(mean {st.mean(r['grow'] for r in rows):.0f}), costs {st.median(r['cost'] for r in rows):.2f}, "
              f"read and run calls {st.median(r['calls'] for r in rows):.0f} (medians)")
    new_ids = {new for new, _, _ in handoffs}
    print('\nfirst 30 requests')
    show('handed-off Jobs', [stretch(threads[s][0], 0) for s in new_ids if s in threads])
    show('other hosted Jobs', [stretch(threads[s][0], 0) for s in threads if s not in new_ids and threads[s][0][0]['ts'] >= SINCE])
    show('after a step boundary, same session', [stretch(in_window(threads[s][0]), n) for s in multi for n in chosen[s]])

    # What a boundary would drop: the person's own messages inside the step it ends.
    told, cuts_with = 0, 0
    for s in multi:
        w = in_window(threads[s][0])
        edges = [w[0]['ts']] + [w[n]['ts'] for n in sorted(chosen[s])]
        for a, b in zip(edges, edges[1:]):
            k = sum(1 for at in person.get(s, []) if a <= at < b)
            told += k
            cuts_with += k > 0
    nb = sum(len(chosen[s]) for s in multi)
    print(f"\nboundaries whose ending step held a message from the person: {cuts_with} of {nb}; messages {told}")

main()
