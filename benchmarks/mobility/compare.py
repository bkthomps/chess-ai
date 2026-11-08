"""Isolate piece mobility evaluation; stdlib and rustc only."""
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

ROOT = pathlib.Path(__file__).resolve().parents[2]
BASELINE = '80abdf6954393e0a4caffe85b602db40deedd317'
COMMON = runpy.run_path(str(ROOT / 'benchmarks/memory/compare.py'))
CASES = [
    ('opening', 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1', 8),
    ('after_e4', 'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1', 7),
    ('after_d4', 'rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1', 7),
    ('middlegame', 'r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1', 6),
    ('endgame', '8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1', 10),
    ('tactics', 'rnbqk2r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQkq - 1 8', 6),
    ('pawns', '8/5pk1/2p3p1/1p1pP2p/1P1P1P1P/2P3K1/8/8 w - - 0 1', 10),
]
FLAGS = ['--edition=2021', '-C', 'opt-level=3', '-C', 'codegen-units=1', '-C', 'lto=fat']


def compile_benchmark(folder, candidate_source=None):
    driver = COMMON['DRIVER'].replace('memory: usize)', 'memory: usize, thread_limit: usize)')
    driver = driver.replace('let threads = search_threads(thread::available_parallelism().map_or(1, usize::from)).min(64);', 'let available = SearchCache::default().threads; let threads = if thread_limit == 0 { available } else { thread_limit.min(available).max(1) };')
    driver = driver.replace('__CONSTRUCTOR__', 'SearchCache::with_threads_and_memory(threads, memory)')
    driver = driver.replace('__INITIALIZE__', 'cache.table.initialize();').replace('__BYTES__', 'cache.table.bytes()').replace('__TOUCH__', '')
    driver = driver.replace('Duration::from_secs(seconds)', 'Duration::from_millis(seconds)')
    driver = driver.replace('started, published: started, depth: 0, searching_depth: 0,', 'started, published: started, depth: 0, searching_depth: 0, evaluation: None,')
    shared = {}
    for name in ['chess', 'tables', 'cache', 'ordering']:
        source = (ROOT / f'src/{name}.rs').read_bytes()
        old = subprocess.check_output(['git', 'show', f'{BASELINE}:src/{name}.rs'], cwd=ROOT)
        if source != old:
            raise RuntimeError(f'{name} differs from baseline; isolate evaluation first')
        shared[name] = source
    hashes = {'shared': {n: hashlib.sha256(s).hexdigest() for n, s in shared.items()}}
    binaries = {}
    for variant in ['baseline', 'mobility']:
        dest = folder / variant
        dest.mkdir()
        source = subprocess.check_output(['git', 'show', f'{BASELINE}:src/engine.rs'], cwd=ROOT) if variant == 'baseline' else (candidate_source or ROOT / 'benchmarks/mobility/initial-engine.rs').read_bytes()
        hashes[variant] = hashlib.sha256(source).hexdigest()
        diagnostics = r"""
pub fn static_diagnostics(fen: &str) {
 let p = Position::from_fen(fen).unwrap();
 let rows: Vec<_> = p.legal_moves().iter().map(|m| format!("\"{}\":{}", m, -evaluate(&p.play(*m)))).collect();
 println!("{{{}}}", rows.join(","));
}
"""
        (dest / 'engine.rs').write_bytes(source + driver.encode() + diagnostics.encode())
        for name, contents in shared.items():
            (dest / f'{name}.rs').write_bytes(contents)
        (dest / 'main.rs').write_text('''#![allow(dead_code, unused_variables)]
mod chess; mod tables; mod engine;
fn main() {
 let a: Vec<_> = std::env::args().collect();
 if a[2] == "static" { engine::static_diagnostics(&a[1]); return; }
 engine::memory_benchmark(&a[1], &a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), 100_000_000, a[5].parse().unwrap());
}
''')
        binary = dest / 'bench'
        subprocess.run(['rustc', *FLAGS, str(dest / 'main.rs'), '-o', str(binary)], check=True)
        binaries[variant] = binary
    return binaries, hashes


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--repetitions', type=int, default=5)
    parser.add_argument('--candidate', type=pathlib.Path, help='Explicit alternative engine source; requires a distinct --output')
    parser.add_argument('--output', type=pathlib.Path, default=ROOT / 'benchmarks/mobility/results.json')
    args = parser.parse_args()
    if args.candidate and args.output == ROOT / 'benchmarks/mobility/results.json':
        parser.error('--candidate requires a distinct --output to preserve historical measurements')
    data = dict(baseline=BASELINE, rust=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                platform=platform.platform(), flags=FLAGS, memory_budget=100_000_000, cases=CASES,
                repetitions=args.repetitions, candidate=str(args.candidate or 'archived initial-engine.rs'), runs=[])
    rng = random.Random(2401)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='chess-mobility-') as tmp:
        binaries, data['source_sha256'] = compile_benchmark(pathlib.Path(tmp), args.candidate)
        data['static_root_scores'] = {name: {v: json.loads(subprocess.check_output([str(binary), fen, 'static'], text=True)) for v, binary in binaries.items()} for name, fen, _ in CASES}
        # Compile both variants before timing; every measurement starts a fresh process.
        for name, fen, depth in CASES:
            for threads in [1, 0]:
                for mode, budget in [('depth', 0), ('time', 100), ('time', 500)]:
                    for rep in range(args.repetitions):
                        variants = list(binaries)
                        rng.shuffle(variants)
                        for variant in variants:
                            row = COMMON['measure']([str(binaries[variant]), fen, mode, str(budget), str(depth), str(threads)])
                            row.update(case=name, variant=variant, mode=mode, budget_ms=budget, requested_threads=threads, repetition=rep)
                            data['runs'].append(row)
                        args.output.write_text(json.dumps(data, indent=2) + '\n')
                    print(name, threads, mode, budget, flush=True)
    # Each variant must be repeatable at fixed depth in serial; evaluation intentionally differs.
    for name, _, _ in CASES:
        for variant in binaries:
            rows = [r for r in data['runs'] if r['case'] == name and r['variant'] == variant and r['mode'] == 'depth']
            assert len({r['score'] for r in rows}) == 1, (name, variant, 'serial/parallel disagreement')
            serial = [r for r in rows if r['requested_threads'] == 1]
            assert len({(r['score'], r['best'], r['nodes']) for r in serial}) == 1
    lines = ['# Measured results', '', 'Fresh processes, cold caches; medians and ranges. Scores intentionally may differ between evaluators.', '',
             '| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |',
             '|---|---:|---|---|---|---|---|---|---:|---:|']
    groups = {}
    for r in data['runs']:
        key = (r['case'], r['requested_threads'], r['mode'], r['budget_ms'], r['variant'])
        groups.setdefault(key, []).append(r)
    for (name, threads, mode, budget, variant), rows in groups.items():
        span = lambda k: f"{min(r[k] for r in rows):g}–{max(r[k] for r in rows):g}"
        elapsed = statistics.median(r['elapsed_ms'] for r in rows)
        moves = ', '.join(sorted({r['best'] for r in rows}))
        rss = max((r['peak_rss_bytes'] or 0) for r in rows) / 1e6
        cache = max(r['cache_bytes'] for r in rows) / 1e6
        lines.append(f'| {name} | {rows[0]["threads"]} | {mode} / {budget} | {variant} | {span("depth")} | {span("score")} | {moves} | {elapsed:.2f} ({span("elapsed_ms")}) | {cache:.2f} | {rss:.2f} |')
    args.output.with_suffix('.md').write_text('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
