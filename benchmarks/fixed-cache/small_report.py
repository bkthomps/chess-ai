"""Report depth-10 small-cache trials relative to a same-batch 60 MB control."""
import argparse
import json
import math
import pathlib
import statistics

HERE = pathlib.Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--input', type=pathlib.Path, default=HERE / 'small-depth10.json')
    args = parser.parse_args()
    data = json.loads(args.input.read_text())
    rows = data['runs']
    expected = len(data['cases']) * len(data['threads']) * len(data['cache_mb']) * data['repetitions']
    assert len(rows) == expected, (len(rows), expected)
    assert all(r['mode'] == 'depth' and r['depth'] == r['target_depth'] == 10 for r in rows)
    assert all(r['cache_bytes'] <= r['cache_limit'] for r in rows)
    identities = [(r['case'], r['threads'], r['cache_limit'], r['repetition']) for r in rows]
    assert len(set(identities)) == len(rows)
    med = statistics.median
    lines = ['# Depth 10: 20, 40, and 60 MB cache ceilings', '',
             f"{data['repetitions']} fresh-process, cold-cache repetitions per size, position, and thread count. All trials complete depth 10. Cache ceilings are decimal MB. Speed is the reciprocal of elapsed time; 60 MB is 1.000×. Each aggregate is the geometric mean of per-position median speed ratios, weighting positions equally. Controls are from this batch; historical timings are not pooled.", '',
             '| Cache MB | One-thread speed | Change | Six-thread speed | Change |',
             '|---:|---:|---:|---:|---:|']
    for mb in data['cache_mb']:
        cells = []
        for threads in data['threads']:
            ratios = []
            for name, _, _ in data['cases']:
                group = [r for r in rows if r['case'] == name and r['threads'] == threads]
                baseline = med(r['elapsed_ms'] for r in group if r['cache_limit'] == 60_000_000)
                elapsed = med(r['elapsed_ms'] for r in group if r['cache_limit'] == mb * 1_000_000)
                ratios.append(baseline / elapsed)
            speed = math.exp(sum(map(math.log, ratios)) / len(ratios))
            cells += [f'{speed:.4f}×', f'{(speed-1)*100:+.2f}%']
        lines.append(f"| {mb} | {' | '.join(cells)} |")
    lines += ['', '## Per-position measurements', '',
              '| Position | Threads | Cache MB | Speed vs 60 MB | Median ms (range) | Node range | Hit range | Max cache MB | Max RSS MB | Score | Best moves |',
              '|---|---:|---:|---:|---|---|---|---:|---:|---:|---|']
    for name, _, _ in data['cases']:
        scores = {r['score'] for r in rows if r['case'] == name}
        assert len(scores) == 1, (name, scores)
        for threads in data['threads']:
            baseline = med(r['elapsed_ms'] for r in rows if r['case'] == name and r['threads'] == threads and r['cache_limit'] == 60_000_000)
            for mb in data['cache_mb']:
                group = [r for r in rows if (r['case'], r['threads'], r['cache_limit']) == (name, threads, mb * 1_000_000)]
                assert len(group) == data['repetitions']
                if threads == 1:
                    assert len({(r['nodes'], r['hits'], r['score'], r['best'], r['cache_bytes']) for r in group}) == 1, (name, mb, 'serial repeatability')
                elapsed = med(r['elapsed_ms'] for r in group)
                times = [r['elapsed_ms'] for r in group]
                span = lambda k: f"{min(r[k] for r in group)}–{max(r[k] for r in group)}"
                moves = ', '.join(sorted({r['best'] for r in group}))
                rss = max((r['peak_rss_bytes'] or 0) for r in group) / 1e6
                cache = max(r['cache_bytes'] for r in group) / 1e6
                lines.append(f"| {name} | {threads} | {mb} | {baseline/elapsed:.4f}× | {elapsed:.2f} ({min(times):.2f}–{max(times):.2f}) | {span('nodes')} | {span('hits')} | {cache:.2f} | {rss:.2f} | {next(iter(scores))} | {moves} |")
    lines += ['', '## Scope and validation', '',
              'FENs, source hashes, base revision, compiler, flags, thread configuration, experimental RAM allowance, direct cache ceilings, allocations, RSS, and timestamps are recorded in the JSON. The source-copy driver uses full iterative deepening and release-equivalent optimization including LTO and one codegen unit. Size order is randomized within each repetition. No agent builds, tests, or other performance studies run during collection. The application default remains 60 MB under its 100 MB total server RAM budget.', '',
              'Fixed-depth scores agree across sizes and thread counts. Serial scores, moves, node counts, hits, and allocations repeat for each size. Parallel scheduling can change node counts and tie-breaking. Full timing ranges are shown; small differences with overlapping ranges do not establish a reliable optimum. These depth-only measurements do not measure completed depth under equal time allowances, warm-cache pondering, or playing strength.']
    args.input.with_suffix('.md').write_text('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
