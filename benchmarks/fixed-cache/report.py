"""Summarize direct cache-ceiling trials without hiding timing ranges."""
import json
import math
import pathlib
import statistics

HERE = pathlib.Path(__file__).resolve().parent
p = json.loads((HERE / 'results.json').read_text())
rows = p['runs']
med = statistics.median
lines = ['# Fixed cache results', '',
         'Decimal MB; five fresh-process cold-cache trials per combination. Medians and full ranges are retained below. LTO enabled, one codegen unit. Fixed-depth times include cache initialization; all sizes start at the same 4 MiB and grow on demand. Cache capacity is the only search parameter varied within a thread setting.', '',
         '| Depth | Threads | Cache ceiling MB | Geometric mean time relative to 60 MB |',
         '|---:|---:|---:|---:|']
for d in [6, 8, 10]:
    for t in p['threads']:
        for mb in p['cache_mb']:
            ratios = []
            for name, _, _ in p['cases']:
                group = [r for r in rows if r['mode']=='depth' and r['depth']==d and r['threads']==t and r['case']==name]
                base = med(r['elapsed_ms'] for r in group if r['cache_limit']==60_000_000)
                value = med(r['elapsed_ms'] for r in group if r['cache_limit']==mb*1_000_000)
                ratios.append(value/base)
            ratio = math.exp(sum(map(math.log, ratios))/len(ratios))
            lines.append(f'| {d} | {t} | {mb} | {ratio:.3f} |')
lines += ['', '## All measurements', '',
          '| Position | Threads | Mode / depth target | Ceiling MB | Completed depth range | Median ms (range) | Scores | Max allocated cache MB | Max RSS MB |',
          '|---|---:|---|---:|---|---|---|---:|---:|']
for name, _, _ in p['cases']:
    for t in p['threads']:
        for mode,d in [('depth',6),('depth',8),('depth',10),('time',0)]:
            for mb in p['cache_mb']:
                g=[r for r in rows if (r['case'],r['threads'],r['mode'],r['target_depth'],r['cache_limit'])==(name,t,mode,d,mb*1_000_000)]
                span=lambda key: f"{min(r[key] for r in g):.2f}–{max(r[key] for r in g):.2f}"
                lines.append(f"| {name} | {t} | {mode} / {d} | {mb} | {min(r['depth'] for r in g)}–{max(r['depth'] for r in g)} | {med(r['elapsed_ms'] for r in g):.2f} ({span('elapsed_ms')}) | {', '.join(map(str,sorted({r['score'] for r in g})))} | {max(r['cache_bytes'] for r in g)/1e6:.2f} | {max((r['peak_rss_bytes'] or 0) for r in g)/1e6:.2f} |")
for name,_,_ in p['cases']:
    for d in [6,8,10]:
        g=[r for r in rows if r['case']==name and r['mode']=='depth' and r['depth']==d]
        assert len({r['score'] for r in g})==1, (name,d)
        for mb in p['cache_mb']:
            s=[r for r in g if r['threads']==1 and r['cache_limit']==mb*1_000_000]
            assert len({(r['nodes'],r['hits'],r['score'],r['best'],r['cache_bytes']) for r in s})==1, (name,d,mb)
(HERE / 'results.md').write_text('\n'.join(lines)+'\n')

confirmation = HERE / 'confirmation.json'
if confirmation.exists():
    b = json.loads(confirmation.read_text())
    assert b['source_sha256'] == p['source_sha256']
    lines = ['# Confirmation: 60 versus 140 MB at depth 10', '',
             'Five additional fresh-process trials per size, six threads, randomized size order. These are separate from the main five trials. Absolute times changed between batches, so raw times are not pooled. All scores match the main study.', '',
             '| Position | Cache MB | Median ms (range) |', '|---|---:|---:|']
    ratios = []
    for name, _, _ in b['cases']:
        values = {}
        for mb in [60, 140]:
            g = [r for r in b['runs'] if r['case'] == name and r['cache_limit'] == mb * 1_000_000]
            assert len(g) == 5
            scores = {r['score'] for r in rows + b['runs'] if r['case'] == name and r['mode'] == 'depth' and r['depth'] == 10}
            assert len(scores) == 1
            times = [r['elapsed_ms'] for r in g]
            values[mb] = med(times)
            lines.append(f'| {name} | {mb} | {values[mb]:.2f} ({min(times):.2f}–{max(times):.2f}) |')
        ratios.append(values[140] / values[60])
    ratio = math.exp(sum(map(math.log, ratios)) / len(ratios))
    lines += ['', f'Geometric mean time ratio, 140 / 60 MB: **{ratio:.3f}** ({(1-ratio)*100:.1f}% lower median time in this batch). Main study ratio: 0.967 (3.3% lower). The confirmation opening reverses the main opening ranking; results depend on position and batch. Timing ranges overlap. Neither batch demonstrates a universal optimum or an additional completed ply under equal time budgets.']
    (HERE / 'confirmation.md').write_text('\n'.join(lines) + '\n')
