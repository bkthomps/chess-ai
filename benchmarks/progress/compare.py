"""Compare snapshot intervals using temporary copies of the production engine."""
import hashlib
import json
import pathlib
import platform
import random
import runpy
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
common = runpy.run_path(str(ROOT / 'benchmarks/memory/compare.py'))
cases = common['CASES']
source = (ROOT / 'src/engine.rs').read_text()
driver = common['DRIVER'].replace('memory: usize)', 'memory: usize, thread_limit: usize)')
driver = driver.replace('let threads = search_threads(thread::available_parallelism().map_or(1, usize::from)).min(64);', 'let available = SearchCache::default().threads; let threads = if thread_limit == 0 { available } else { thread_limit.min(available).max(1) };')
driver = driver.replace('__CONSTRUCTOR__', 'SearchCache::with_threads_and_memory(threads, memory)').replace('__INITIALIZE__', 'cache.table.initialize();').replace('__BYTES__', 'cache.table.bytes()').replace('__TOUCH__', '')
driver = driver.replace('let (depth, score, best, nodes, hits, elapsed) = if mode == "time" {', 'let progress = Mutex::new(SearchProgress::default());\n    let (depth, score, best, nodes, hits, elapsed) = if mode == "time" {')
driver = driver.replace('cache.search(&p, &keys, Duration::from_secs(seconds), Arc::default());\n        (r.depth', 'cache.search_with_progress(&p, &keys, Duration::from_secs(seconds), Arc::default(), &progress);\n        (r.depth')
driver = driver.replace('cancelled: Arc::default(), progress: None,', 'cancelled: Arc::default(), progress: Some(&progress), evaluation: None,')
data = {'machine': platform.platform(), 'rust': subprocess.check_output(['rustc', '--version'], text=True).strip(), 'flags': '-C opt-level=3 -C codegen-units=1 -C lto', 'memory_budget': 100000000, 'repetitions': 3, 'seconds': 2, 'cases': cases, 'source_sha256': {n: hashlib.sha256((ROOT / ('src/' + n + '.rs')).read_bytes()).hexdigest() for n in ['engine','chess','tables','cache','ordering']}, 'runs': []}
with tempfile.TemporaryDirectory(prefix='chess-progress-') as tmp:
    folder = pathlib.Path(tmp)
    for name in ['cache', 'ordering']:
        (folder / (name + '.rs')).write_text((ROOT / ('src/' + name + '.rs')).read_text())
    for interval in [50, 250]:
        marker = 'now.duration_since(self.published) >= Duration::from_millis(150)'
        assert source.count(marker) == 1
        variant = source.replace(marker, f'now.duration_since(self.published) >= Duration::from_millis({interval})')
        (folder / f'engine{interval}.rs').write_text(variant + driver)
    modules = [(n, ROOT / ('src/' + n + '.rs')) for n in ['tables', 'chess']] + [(f'engine{i}', folder / f'engine{i}.rs') for i in [50,250]]
    rust = '#![allow(dead_code, unused_variables)]\n' + '\n'.join(f'#[path={json.dumps(str(p))}] mod {n};' for n,p in modules)
    rust += '''\nfn main() { let a: Vec<_> = std::env::args().collect(); let run = if a[1] == "50" { engine50::memory_benchmark } else { engine250::memory_benchmark }; run(&a[2], &a[3], 2, a[4].parse().unwrap(), 100_000_000, a[5].parse().unwrap()); }\n'''
    (folder / 'main.rs').write_text(rust)
    binary = folder / 'bench'
    subprocess.run(['rustc', '--edition=2021', '-C', 'opt-level=3', '-C', 'codegen-units=1', '-C', 'lto', str(folder / 'main.rs'), '-o', str(binary)], check=True)
    rng = random.Random(7581)
    for threads in [1, 0]:
        for name, fen, depth in cases:
            for repetition in range(3):
                variants = [50, 250]
                rng.shuffle(variants)
                for interval in variants:
                    for mode in ['depth', 'time']:
                        row = common['measure']([str(binary), str(interval), fen, mode, str(depth), str(threads)])
                        row.update(case=name, mode=mode, interval_ms=interval, repetition=repetition, requested_threads=threads)
                        data['runs'].append(row)
                        print(json.dumps(row), flush=True)
                        (ROOT / 'benchmarks/progress/results.json').write_text(json.dumps(data, indent=2) + '\n')
