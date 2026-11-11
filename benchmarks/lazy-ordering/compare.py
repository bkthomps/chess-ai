"""Reproduce the isolated lazy-ordering study with Rust/Python standard libraries."""
import argparse
import hashlib
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
BASELINE = '162cd89'
COMMON = runpy.run_path(str(ROOT / 'benchmarks/memory/compare.py'))
CASES = runpy.run_path(str(ROOT / 'benchmarks/piece-pawn-relationships/compare.py'))['CASES']
FLAGS = ['--edition=2021', '-C', 'opt-level=3', '-C', 'codegen-units=1', '-C', 'lto=fat']


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--repetitions', type=int, default=9)
    parser.add_argument('--threads', nargs='+', type=int, default=[1, 0])
    parser.add_argument('--modes', nargs='+', default=['depth', 'time', 'ponder'], choices=['depth', 'time', 'ponder'])
    parser.add_argument('--budget', type=int, default=1000)
    parser.add_argument('--patch', type=pathlib.Path, default=HERE / 'candidate.patch')
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    driver = COMMON['DRIVER'].replace('memory: usize)', 'memory: usize, thread_limit: usize)')
    driver = driver.replace('let threads = search_threads(thread::available_parallelism().map_or(1, usize::from)).min(64);', 'let available = SearchCache::default().threads; let threads = if thread_limit == 0 { available } else { thread_limit.min(available).max(1) };')
    driver = driver.replace('__CONSTRUCTOR__', 'SearchCache::with_threads_and_memory(threads, memory)')
    driver = driver.replace('__INITIALIZE__', 'cache.table.initialize();').replace('__BYTES__', 'cache.table.bytes()').replace('__TOUCH__', '')
    driver = driver.replace('Duration::from_secs(seconds)', 'Duration::from_millis(seconds)')
    driver = driver.replace('best.unwrap()', 'best.map(|m| m.to_string()).unwrap_or_else(|| "-".into())')
    driver = driver.replace('started, published: started, depth: 0, searching_depth: 0,', 'started, published: started, depth: 0, searching_depth: 0, evaluation: None,')
    data = dict(baseline_revision=subprocess.check_output(['git', 'rev-parse', BASELINE], cwd=ROOT, text=True).strip(),
                rust=subprocess.check_output(['rustc', '--version'], text=True).strip(), platform=platform.platform(),
                flags=FLAGS, patch=str(args.patch), patch_sha256=hashlib.sha256(args.patch.read_bytes()).hexdigest(), memory_budget=100_000_000, cases=CASES, repetitions=args.repetitions, source_sha256={}, runs=[])
    rng = random.Random(1711)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='chess-lazy-') as tmp:
        binaries = {}
        for variant in ['baseline', 'lazy']:
            dest = pathlib.Path(tmp) / variant
            dest.mkdir()
            for name in ['chess', 'tables', 'cache', 'ordering', 'engine']:
                source = subprocess.check_output(['git', 'show', f'{BASELINE}:src/{name}.rs'], cwd=ROOT)
                (dest / f'{name}.rs').write_bytes(source)
            if variant == 'lazy':
                subprocess.run(['git', 'apply', str(args.patch.resolve())], cwd=dest, check=True)
            data['source_sha256'][variant] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in dest.glob('*.rs')}
            with (dest / 'engine.rs').open('a') as f:
                f.write(driver)
            (dest / 'main.rs').write_text('''#![allow(dead_code, unused_variables)]
mod chess; mod tables; mod engine;
fn main() {
 let a: Vec<_> = std::env::args().collect();
 engine::memory_benchmark(&a[1], &a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), 100_000_000, a[5].parse().unwrap());
}
''')
            binary = dest / 'bench'
            subprocess.run(['rustc', *FLAGS, str(dest / 'main.rs'), '-o', str(binary)], check=True)
            binaries[variant] = binary
        # Compile both executables before all timings; each trial is a fresh process.
        for name, fen, depth in CASES:
            for threads in args.threads:
                for mode in args.modes:
                    budget = 0 if mode == 'depth' else args.budget
                    for rep in range(args.repetitions):
                        order = list(binaries)
                        rng.shuffle(order)
                        for variant in order:
                            row = COMMON['measure']([str(binaries[variant]), fen, mode, str(budget), str(depth), str(threads)])
                            row.update(case=name, variant=variant, mode=mode, budget_ms=budget, requested_threads=threads, repetition=rep)
                            data['runs'].append(row)
                        args.output.write_text(json.dumps(data, indent=2) + '\n')
                    print(name, threads, mode, flush=True)
        for name, _, _ in CASES:
            rows = [r for r in data['runs'] if r['case'] == name and r['mode'] == 'depth']
            assert len({r['score'] for r in rows}) <= 1, (name, 'score disagreement')
            serial = [r for r in rows if r['requested_threads'] == 1]
            assert len({(r['score'], r['best'], r['nodes'], r['hits']) for r in serial}) <= 1, (name, 'serial tree disagreement')
    groups = {}
    for row in data['runs']:
        key = (row['case'], row['requested_threads'], row['mode'], row['variant'])
        groups.setdefault(key, []).append(row)
    lines = ['# Measurements', '', 'Fresh processes, seeded shuffled order, equal 100 MB total budget. Ponder: warmup then 2 s reply.', '',
             '| Position | Threads | Mode | Variant | Depth range | Median ms (range) | Max cache MB | Peak RSS MB |',
             '|---|---:|---|---|---|---|---:|---:|']
    for (name, threads, mode, variant), rows in groups.items():
        times = [r['elapsed_ms'] for r in rows]
        depths = [r['depth'] for r in rows]
        lines.append(f'| {name} | {rows[0]["threads"]} | {mode} | {variant} | {min(depths)}–{max(depths)} | '
                     f'{statistics.median(times):.2f} ({min(times):.2f}–{max(times):.2f}) | '
                     f'{max(r["cache_bytes"] for r in rows)/1e6:.2f} | {max(r["peak_rss_bytes"] or 0 for r in rows)/1e6:.2f} |')
    args.output.with_suffix('.md').write_text('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
