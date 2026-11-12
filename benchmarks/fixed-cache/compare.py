"""Direct cache ceilings; fresh processes, cold caches, dependency-free."""
import argparse
import hashlib
import json
import pathlib
import platform
import random
import runpy
import subprocess
import tempfile
import datetime

ROOT = pathlib.Path(__file__).resolve().parents[2]
COMMON = runpy.run_path(str(ROOT / 'benchmarks/memory/compare.py'))
FLAGS = ['--edition=2021', '-C', 'opt-level=3', '-C', 'codegen-units=1', '-C', 'lto=fat']


def main():
    p = argparse.ArgumentParser(__doc__)
    p.add_argument('--resume', action='store_true', help='Resume an interrupted output, preserving completed trials')
    p.add_argument('--repetitions', type=int, default=5)
    p.add_argument('--depths', type=int, nargs='+', default=[6, 8, 10])
    p.add_argument('--sizes', type=int, nargs='+', default=[60, 80, 100, 120, 140, 160])
    p.add_argument('--modes', nargs='+', choices=['depth', 'time'], default=['depth', 'time'])
    p.add_argument('--seconds', type=int, default=2)
    p.add_argument('--threads', type=int, nargs='+', default=[1, 6])
    p.add_argument('--cases', nargs='+', default=['opening', 'middlegame', 'endgame'])
    p.add_argument('--output', type=pathlib.Path, default=ROOT / 'benchmarks/fixed-cache/results.json')
    a = p.parse_args()
    cases = [list(c) for c in COMMON['CASES'] if c[0] in a.cases]
    data = dict(revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                rust=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                platform=platform.platform(), flags=FLAGS, cases=cases,
                cache_mb=a.sizes, repetitions=a.repetitions, modes=a.modes,
                seconds=a.seconds, threads=a.threads, depths=a.depths, random_seed=4281, runs=[])
    driver = COMMON['DRIVER'].replace('memory: usize)', 'memory: usize, threads: usize)')
    driver = driver.replace('let threads = search_threads(thread::available_parallelism().map_or(1, usize::from)).min(64);', '')
    driver = driver.replace('__CONSTRUCTOR__', '{ let mut c = SearchCache::with_threads_and_memory(threads, 1_000_000_000); c.table = SharedTable::with_limit(memory); c }')
    driver = driver.replace('__INITIALIZE__', 'cache.table.initialize();').replace('__BYTES__', 'cache.table.bytes()').replace('__TOUCH__', '')
    driver = driver.replace('Duration::from_secs(60)', 'Duration::from_secs(1800)')
    driver = driver.replace('assert!(!s.aborted, "fixed-depth benchmark exceeded 60 seconds");', 'assert!(!s.aborted, "fixed-depth benchmark exceeded 1800 seconds");')
    driver = driver.replace('depth: 0, searching_depth: 0,', 'depth: 0, searching_depth: 0, evaluation: None,')
    data['source_sha256'] = {}
    if a.resume:
        saved = json.loads(a.output.read_text())
        assert all(saved[k] == data[k] for k in ['revision', 'rust', 'flags', 'cases', 'cache_mb', 'repetitions', 'seconds', 'threads'])
        assert set(saved.get('modes', [r['mode'] for r in saved['runs']])) == set(a.modes)
        assert set(saved.get('depths', [r['target_depth'] for r in saved['runs'] if r['mode'] == 'depth'])) == set(a.depths)
        data.update(saved)
    retained_hashes = data.get('source_sha256', {}).copy()
    a.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='fixed-cache-') as tmp:
        folder = pathlib.Path(tmp)
        for name in ['engine', 'cache', 'ordering', 'tables', 'chess']:
            source = (ROOT / f'src/{name}.rs').read_text()
            data['source_sha256'][name] = hashlib.sha256(source.encode()).hexdigest()
            (folder / f'{name}.rs').write_text(source + (driver if name == 'engine' else ''))
        (folder / 'main.rs').write_text('''#![allow(dead_code, unused_variables)]
mod chess; mod tables; mod engine;
fn main() {
 let a: Vec<_> = std::env::args().collect();
 engine::memory_benchmark(&a[1], &a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap());
}
''')
        if retained_hashes:
            assert retained_hashes == data['source_sha256'], 'Source changed since interrupted collection'
        binary = folder / 'bench'
        subprocess.run(['rustc', *FLAGS, str(folder / 'main.rs'), '-o', str(binary)], check=True)
        rng = random.Random(data['random_seed'])
        for name, fen, _ in cases:
            for threads in a.threads:
                tasks = ([('depth', d) for d in a.depths] if 'depth' in a.modes else []) + ([('time', 0)] if 'time' in a.modes else [])
                for mode, depth in tasks:
                    for rep in range(a.repetitions):
                        sizes = data['cache_mb'].copy()
                        rng.shuffle(sizes)
                        for mb in sizes:
                            if any((r['case'], r['threads'], r['mode'], r['target_depth'], r['repetition'], r['cache_limit']) == (name, threads, mode, depth, rep, mb * 1_000_000) for r in data['runs']):
                                continue
                            started_utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
                            row = COMMON['measure']([str(binary), fen, mode, str(a.seconds), str(depth), str(mb * 1_000_000), str(threads)])
                            row.update(started_utc=started_utc, finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), case=name, mode=mode, target_depth=depth, cache_limit=mb * 1_000_000,
                                       repetition=rep, experimental_total_budget=1_000_000_000)
                            data['runs'].append(row)
                            a.output.write_text(json.dumps(data, indent=2) + '\n')
                            print(name, threads, mode, depth, rep, mb, round(row['elapsed_ms'], 1), flush=True)
    for name, _, _ in cases:
        for depth in a.depths:
            rows = [r for r in data['runs'] if r['case'] == name and r['mode'] == 'depth' and r['depth'] == depth]
            assert not rows or len({r['score'] for r in rows}) == 1, (name, depth, 'score disagreement')


if __name__ == '__main__':
    main()
