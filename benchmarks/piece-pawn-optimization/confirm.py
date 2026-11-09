"""Confirm all three score-preserving proposals against the frozen current engine."""
import argparse
import json
import pathlib
import platform
import random
import runpy
import statistics
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
STUDY = runpy.run_path(str(ROOT / 'benchmarks/piece-pawn-relationships/compare.py'))
SOURCES = {'baseline': 'baseline-engine.rs', 'files': 'files-engine.rs',
           'shared': 'shared-engine.rs', 'skip': 'skip-engine.rs'}
CASES = [c for c in STUDY['CASES'] if c[0] in ['opening', 'middlegame', 'endgame', 'relationships']]


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--repetitions', type=int, default=15)
    parser.add_argument('--output', type=pathlib.Path, default=HERE / 'confirmation.json')
    args = parser.parse_args()
    if args.repetitions < 1:
        parser.error('positive repetitions required')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    data = dict(parent_revision=STUDY['BASELINE'], baseline_source=SOURCES['baseline'],
                rust=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                platform=platform.platform(), flags=STUDY['FLAGS'], memory_budget=100_000_000,
                cases=CASES, repetitions=args.repetitions, source_sha256={}, runs=[])
    rng = random.Random(1602)
    with tempfile.TemporaryDirectory(prefix='chess-pawn-confirm-') as tmp:
        binaries = {}
        # All compilation completes before the first timing measurement.
        for variant in ['files', 'shared', 'skip']:
            folder = pathlib.Path(tmp) / variant
            folder.mkdir()
            pair, hashes = STUDY['compile_benchmark'](folder, HERE / SOURCES[variant], HERE / SOURCES['baseline'])
            binaries.setdefault('baseline', pair['baseline'])
            binaries[variant] = pair['relationships']
            data['source_sha256']['baseline'] = hashes['baseline']
            data['source_sha256'][variant] = hashes['relationships']
            data['shared_sha256'] = hashes['shared']
        data['static_root_scores'] = {}
        for name, fen, depth in CASES:
            static = {v: json.loads(subprocess.check_output([str(b), fen, 'static'], text=True))
                      for v, b in binaries.items()}
            assert len({json.dumps(x, sort_keys=True) for x in static.values()}) == 1, name
            data['static_root_scores'][name] = static
            for mode, budget in [('depth', 0), ('time', 100)]:
                for rep in range(args.repetitions):
                    order = list(binaries)
                    rng.shuffle(order)
                    for variant in order:
                        row = STUDY['COMMON']['measure']([str(binaries[variant]), fen, mode, str(budget), str(depth), '1'])
                        row.update(case=name, variant=variant, mode=mode, budget_ms=budget,
                                   requested_threads=1, repetition=rep)
                        data['runs'].append(row)
                    args.output.write_text(json.dumps(data, indent=2) + '\n')
                print(name, mode, flush=True)
            rows = [r for r in data['runs'] if r['case'] == name and r['mode'] == 'depth']
            assert len({(r['score'], r['best'], r['nodes'], r['hits']) for r in rows}) == 1, name
    lines = ['# Confirmation measurements', '',
             'Fresh processes, cold caches, one thread, equal 100 MB total budget. Medians and ranges.', '',
             '| Position | Mode | Variant | Depth range | Median ms (range) | Peak RSS MB |',
             '|---|---|---|---|---|---:|']
    for name, _, _ in CASES:
        for mode in ['depth', 'time']:
            for variant in SOURCES:
                rows = [r for r in data['runs'] if r['case'] == name and r['mode'] == mode and r['variant'] == variant]
                elapsed = [r['elapsed_ms'] for r in rows]
                depths = [r['depth'] for r in rows]
                rss = max(r['peak_rss_bytes'] or 0 for r in rows) / 1e6
                lines.append(f'| {name} | {mode} | {variant} | {min(depths)}–{max(depths)} | '
                             f'{statistics.median(elapsed):.2f} ({min(elapsed):.2f}–{max(elapsed):.2f}) | {rss:.2f} |')
    args.output.with_suffix('.md').write_text('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
