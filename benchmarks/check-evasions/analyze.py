"""Summarize paired trials; no third-party packages."""
import json
import pathlib
import statistics

HERE = pathlib.Path(__file__).resolve().parent
lines = ['# Paired comparisons', '',
         'Time change is the median within-pair candidate/baseline ratio minus one; negative is faster. '
         'Depth W/T/L compares completed depth in each pair. Ranges describe measured variation, not confidence intervals.', '']
for filename in ['screen.json', 'confirmation.json', 'ponder.json']:
    path = HERE / filename
    if not path.exists():
        continue
    data = json.loads(path.read_text())
    lines += [f'## {filename}', '',
              '| Position | Threads | Mode / budget ms | Pairs | Median time change (range) | Baseline / candidate median depth | Depth W/T/L |',
              '|---|---:|---|---:|---|---|---|']
    groups = {}
    for row in data['runs']:
        key = (row['case'], row['threads'], row['mode'], row['budget_ms'])
        groups.setdefault(key, {}).setdefault(row['repetition'], {})[row['variant']] = row
    for (name, threads, mode, budget), trials in groups.items():
        pairs = [p for p in trials.values() if len(p) == 2]
        changes = [100 * (p['evasions']['elapsed_ms'] / p['baseline']['elapsed_ms'] - 1) for p in pairs]
        old = [p['baseline']['depth'] for p in pairs]
        new = [p['evasions']['depth'] for p in pairs]
        win = sum(b > a for a, b in zip(old, new))
        tie = sum(b == a for a, b in zip(old, new))
        loss = len(pairs) - win - tie
        lines.append(f'| {name} | {threads} | {mode} / {budget} | {len(pairs)} | '
                     f'{statistics.median(changes):+.1f}% ({min(changes):+.1f}%–{max(changes):+.1f}%) | '
                     f'{statistics.median(old):g} / {statistics.median(new):g} | {win}/{tie}/{loss} |')
    lines += ['']
(HERE / 'analysis.md').write_text('\n'.join(lines) + '\n')
