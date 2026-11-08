"""Compare the pre-growth engine with bounded adaptive caches; stdlib only.

Run from any directory: python3 benchmarks/memory/compare.py
Use --quick for a short smoke run. Full results include three cold repetitions
of both fixed-time and fixed-depth searches, with identical thread limits.
"""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import random
import statistics
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
BASELINE = "9288315"
DEFAULT_BUDGETS = [30, 100, 250, 500, 1000]
CASES = [
    ("opening", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 9),
    ("middlegame", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 7),
    ("endgame", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 11),
]

# Added only to temporary source copies: production search has no benchmark
# branches or extra instrumentation. Both versions use the same depth driver.
DRIVER = r'''
pub fn memory_benchmark(fen: &str, mode: &str, seconds: u64, depth: u8, memory: usize) {
    let p = Position::from_fen(fen).unwrap();
    let keys = [p.repetition_key()];
    let threads = search_threads(thread::available_parallelism().map_or(1, usize::from)).min(64);
    let started = Instant::now();
    let mut cache = __CONSTRUCTOR__;
    if mode == "ponder" {
        // The same search used by pondering, bounded here for repeatable runs.
        let warm = cache.search(&p, &keys, Duration::from_secs(seconds), Arc::default());
        let legal = p.legal_moves();
        let human = ["e2e4", "a2a3"].iter().find_map(|uci| legal.iter().find(|m| m.to_string() == *uci)).copied().unwrap_or(*legal.last().unwrap());
        let child = p.play(human);
        let child_keys = [keys[0], child.repetition_key()];
        let r = cache.search(&child, &child_keys, Duration::from_secs(2), Arc::default());
        let allocated: usize = __BYTES__;
        println!("{{\"ponder_depth\":{},\"human_move\":\"{human}\",\"depth\":{},\"score\":{},\"best\":\"{}\",\"nodes\":{},\"hits\":{},\"elapsed_ms\":{},\"threads\":{threads},\"cache_bytes\":{allocated}}}", warm.depth, r.depth, r.score, r.best.unwrap(), r.nodes, r.hits, r.elapsed.as_secs_f64() * 1000.0);
        return;
    }
    if mode == "stress" {
        let rounds = seconds.max(1);
        let mut allocated = 0usize;
        for _ in 0..rounds {
            __INITIALIZE__
            for key in 0..3_000_000u64 {
                cache.table.store(key << 32, Entry { depth: 8, score: 17, best: None, bound: Bound::Exact });
            }
            __TOUCH__
            allocated = allocated.max(__BYTES__);
            cache = __CONSTRUCTOR__;
        }
        println!("{{\"stress\":true,\"rounds\":{rounds},\"cache_bytes\":{allocated},\"elapsed_ms\":{}}}", started.elapsed().as_secs_f64() * 1000.0);
        return;
    }
    let (depth, score, best, nodes, hits, elapsed) = if mode == "time" {
        let r = cache.search(&p, &keys, Duration::from_secs(seconds), Arc::default());
        (r.depth, r.score, r.best, r.nodes, r.hits, r.elapsed)
    } else {
        __INITIALIZE__
        cache.table.next_generation();
        let mut helpers: Vec<_> = (1..threads).map(|_| MoveBuffers::default()).collect();
        let peers: Vec<_> = (1..threads).map(|_| WorkerStats::default()).collect();
        let mut s = Search {
            table: &cache.table, buffers: &mut cache.buffers, keys: keys.to_vec(),
            history_context: reversible_context(&p, &keys),
            deadline: Some(started + Duration::from_secs(60)), nodes: 0, hits: 0,
            aborted: false, cancelled: Arc::default(), progress: None,
            started, published: started, depth: 0, searching_depth: 0,
            threads, peers: &peers, reporting: None, reported: (0, 0), rest_at: None,
        };
        let mut result = (0, None);
        let legal = p.legal_moves();
        for d in 1..=depth {
            result = if threads > 1 && d >= 4 {
                s.parallel_root(&p, d, &legal, &mut helpers)
            } else { s.negamax(&p, d, 0, -INF, INF) };
            assert!(!s.aborted, "fixed-depth benchmark exceeded 60 seconds");
        }
        let (nodes, hits) = s.totals();
        (depth, result.0, result.1, nodes, hits, started.elapsed())
    };
    let allocated: usize = __BYTES__;
    println!("{{\"depth\":{depth},\"score\":{score},\"best\":\"{}\",\"nodes\":{nodes},\"hits\":{hits},\"elapsed_ms\":{},\"threads\":{threads},\"cache_bytes\":{allocated}}}", best.unwrap(), elapsed.as_secs_f64() * 1000.0);
}
'''


def compile_benchmark(directory):
    old = subprocess.check_output(["git", "show", f"{BASELINE}:src/engine.rs"], cwd=ROOT, text=True)
    new = (ROOT / "src/engine.rs").read_text()
    for name, source, constructor, initialize, allocated in [
        ("baseline", old, "SearchCache::with_threads(threads)", "cache.table.resize(TABLE_SIZE);", "28_000_000"),
        ("optimized", new, "SearchCache::with_threads_and_memory(threads, memory)", "cache.table.initialize();", "cache.table.bytes()"),
    ]:
        driver = DRIVER.replace("__CONSTRUCTOR__", constructor).replace("__INITIALIZE__", initialize).replace("__BYTES__", allocated).replace("__TOUCH__", "cache.table.benchmark_touch();" if name == "optimized" else "")
        (directory / f"{name}.rs").write_text(source + driver)
    touch = '''
impl SharedTable {
 pub(super) fn benchmark_touch(&self) {
  let mut state = 17u64;
  for shard in &self.shards {
   let mut shard = shard.lock().unwrap();
   for page in &mut shard.pages { for bucket in page {
    for slot in 0..BUCKET_SIZE {
     state = super::random_key(state);
     bucket.keys[slot] = state;
     bucket.entries[slot] = PackedEntry { score: state as i16, best: (state >> 16) as u16, depth: (state >> 32) as u8, flags: (state >> 40) as u8 | 1 };
    }
    bucket.epoch = state;
   }}
  }
  std::hint::black_box(self);
 }
}
'''
    (directory / "cache.rs").write_text((ROOT / "src/cache.rs").read_text() + touch)
    modules = [("tables", ROOT / "src/tables.rs"), ("chess", ROOT / "src/chess.rs"),
               ("baseline", directory / "baseline.rs"), ("optimized", directory / "optimized.rs")]
    rust = "#![allow(dead_code, unused_variables)]\n" + "\n".join(
        f"#[path={json.dumps(str(path), ensure_ascii=False)}] mod {name};" for name, path in modules)
    rust += '''
fn main() {
 let a: Vec<_> = std::env::args().collect();
 let run = if a[1] == "baseline" { baseline::memory_benchmark } else { optimized::memory_benchmark };
 run(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap());
}
'''
    source = directory / "main.rs"
    source.write_text(rust)
    binary = directory / ("memory-bench.exe" if os.name == "nt" else "memory-bench")
    subprocess.run(["rustc", "--edition=2021", "-C", "opt-level=3", "-C", "codegen-units=1", str(source), "-o", str(binary)], check=True)
    return binary


def measure(command):
    p = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if hasattr(os, "wait4"):
        stdout = p.stdout.read()
        stderr = p.stderr.read()
        _, status, usage = os.wait4(p.pid, 0)
        p.returncode = os.waitstatus_to_exitcode(status)
        rss = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    else:
        stdout, stderr = p.communicate()
        rss = None
    if p.returncode:
        raise RuntimeError(f"{command}\n{stderr}")
    result = json.loads(stdout)
    result["peak_rss_bytes"] = rss
    return result


def report(data):
    rows = data["runs"]
    budgets = data.get("budgets", DEFAULT_BUDGETS)
    if data.get("time_only"):
        lines = ["# Fixed-time memory comparison", "",
                 f"{data['seconds']} seconds per search, {data['repetitions']} cold repetitions, identical thread limits. Medians shown; depth ranges retain run variation. Budgets and RAM usage are decimal MB. Peak RSS excludes the browser.", "",
                 "| Position | Budget MB | Depth | Depth range | Nodes | Cache hits | Cache MB | Peak RSS MB |",
                 "|---|---:|---:|---:|---:|---:|---:|---:|"]
        med = statistics.median
        for name, _, _ in data["cases"]:
            for budget in budgets:
                group = [r for r in rows if r["case"] == name and r["budget_mb"] == budget]
                peak = med(r["peak_rss_bytes"] / 1e6 for r in group if r["peak_rss_bytes"] is not None) if group[0]["peak_rss_bytes"] else float("nan")
                lo, hi = min(r["depth"] for r in group), max(r["depth"] for r in group)
                lines.append(f"| {name} | {budget} | {med(r['depth'] for r in group):g} | {lo}–{hi} | {med(r['nodes'] for r in group):g} | {med(r['hits'] for r in group):g} | {med(r['cache_bytes'] / 1e6 for r in group):.1f} | {peak:.1f} |")
        return "\n".join(lines) + "\n"
    if data.get("depth_only"):
        lines = ["# Fixed-depth memory comparison", "", f"{data['repetitions']} cold repetitions; medians shown. Times include cache initialization.", "",
                 "| Position | Budget MB | Target depth | Median time ms | Time range ms | Nodes | Cache MB | Peak RSS MB |",
                 "|---|---:|---:|---:|---:|---:|---:|---:|"]
        med = statistics.median
        for name, _, target in data["cases"]:
            for budget in budgets:
                group = [r for r in rows if r["case"] == name and r["budget_mb"] == budget]
                peak = med(r["peak_rss_bytes"] / 1e6 for r in group if r["peak_rss_bytes"] is not None) if group[0]["peak_rss_bytes"] else float("nan")
                lo, hi = min(r['elapsed_ms'] for r in group), max(r['elapsed_ms'] for r in group)
                lines.append(f"| {name} | {budget} | {target} | {med(r['elapsed_ms'] for r in group):.1f} | {lo:.0f}–{hi:.0f} | {med(r['nodes'] for r in group):g} | {med(r['cache_bytes'] / 1e6 for r in group):.1f} | {peak:.1f} |")
        return "\n".join(lines) + "\n"
    if data.get("ponder"):
        lines = ["# Pondering memory comparison", "", f"Human-turn search: {data['seconds']} seconds; computer move: 2 seconds after the recorded human move. {data['repetitions']} cold repetitions; medians shown.", "",
                 "| Position | Budget MB | Ponder depth | Computer depth | Computer nodes | Cache hits | Cache MB | Peak RSS MB |",
                 "|---|---:|---:|---:|---:|---:|---:|---:|"]
        med = statistics.median
        for name, _, _ in data["cases"]:
            for budget in budgets:
                group = [r for r in rows if r["case"] == name and r["budget_mb"] == budget]
                peak = med(r["peak_rss_bytes"] / 1e6 for r in group if r["peak_rss_bytes"] is not None) if group[0]["peak_rss_bytes"] else float("nan")
                lines.append(f"| {name} | {budget} | {med(r['ponder_depth'] for r in group):g} | {med(r['depth'] for r in group):g} | {med(r['nodes'] for r in group):g} | {med(r['hits'] for r in group):g} | {med(r['cache_bytes'] / 1e6 for r in group):.1f} | {peak:.1f} |")
        return "\n".join(lines) + "\n"
    lines = ["# Search memory comparison", "", f"Baseline: `{BASELINE}`; machine: {data['machine']}.", "",
             "Budgets are decimal MB of total engine/server RAM, with cache headroom reserved for other allocations. Each run is a fresh process and cold cache. Peak RSS includes process overhead and is not the configured ceiling. Timing includes initialization. All tiers use the same CPU headroom policy.", "",
             "| Position | Budget | Depth in timed search | Fixed-depth time (ms) | Cache MB, timed | Peak RSS MB, timed |",
             "|---|---:|---:|---:|---:|---:|"]
    for name, _, _ in data["cases"]:
        for budget in budgets:
            group = [r for r in rows if r["case"] == name and r["budget_mb"] == budget]
            timed = [r for r in group if r["mode"] == "time"]
            fixed = [r for r in group if r["mode"] == "depth"]
            med = statistics.median
            peak = med(r["peak_rss_bytes"] / 1e6 for r in timed if r["peak_rss_bytes"] is not None) if timed[0]["peak_rss_bytes"] else float("nan")
            lines.append(f"| {name} | {'original ~30' if budget == 30 else budget} | {med(r['depth'] for r in timed):g} | {med(r['elapsed_ms'] for r in fixed):.1f} | {med(r['cache_bytes'] / 1e6 for r in timed):.1f} | {peak:.1f} |")
    lines += ["", f"Timed searches: {data['seconds']} seconds; fixed targets: " + ", ".join(f"{n} {d}" for n, _, d in data["cases"]) + ".",
              f"{data['repetitions']} repetitions per cell; medians shown. Results depend on the position and machine. Cache growth is demand-driven, so a larger ceiling may use the same amount of RAM and produce the same depth.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--quick", action="store_true")
    parser.add_argument("--seconds", type=int, default=10)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "benchmarks/memory/results.json")
    parser.add_argument("--stress-only", action="store_true")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--ponder", action="store_true")
    mode.add_argument("--depth-only", action="store_true")
    mode.add_argument("--time-only", action="store_true")
    parser.add_argument("--budgets", nargs="+", type=int, default=DEFAULT_BUDGETS,
                        help="total RAM budgets in decimal MB; 30 selects the historical baseline")
    parser.add_argument("--depth", type=int)
    parser.add_argument("--case", choices=["all", "opening", "middlegame", "endgame"], default="all")
    args = parser.parse_args()
    repetitions, seconds = (1, 1) if args.quick else (args.repetitions, args.seconds)
    if repetitions < 1 or seconds < 1 or args.depth is not None and not 1 <= args.depth <= 255:
        parser.error("positive repetitions/seconds and depth 1–255 are required")
    if any(not 1 <= b <= 1000 for b in args.budgets) or len(set(args.budgets)) != len(args.budgets):
        parser.error("budgets must be unique values between 1 and 1000 MB")
    cases = [(n, f, args.depth if args.depth is not None else d - int(args.quick)) for n, f, d in CASES]
    if args.case != "all": cases = [c for c in cases if c[0] == args.case]
    if args.ponder and args.output == ROOT / "benchmarks/memory/results.json":
        args.output = args.output.with_name("ponder-results.json")
    data = {"baseline": BASELINE, "machine": platform.platform(), "cpus": os.cpu_count(),
            "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "source_sha256": hashlib.sha256((ROOT / "src/engine.rs").read_bytes() + (ROOT / "src/cache.rs").read_bytes()).hexdigest(),
            "seconds": seconds, "repetitions": repetitions, "cases": cases, "budgets": args.budgets,
            "ponder": args.ponder, "depth_only": args.depth_only, "time_only": args.time_only, "runs": []}
    rng = random.Random(0xC4C4E)
    with tempfile.TemporaryDirectory(prefix="chess-memory-") as tmp:
        binary = compile_benchmark(pathlib.Path(tmp))
        if args.stress_only:
            result = measure([str(binary), "optimized", CASES[0][1], "stress", str(repetitions), "0", "1000000000"])
            result.update(memory_limit_bytes=1_000_000_000, source_sha256=data["source_sha256"], machine=data["machine"])
            args.output.write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps(result), flush=True)
            assert result["peak_rss_bytes"] is None or result["peak_rss_bytes"] < 1_000_000_000, result
            return
        for name, fen, depth in cases:
            for repetition in range(repetitions):
                budgets = args.budgets.copy()
                rng.shuffle(budgets)
                for budget in budgets:
                    for mode in (["ponder"] if args.ponder else ["depth"] if args.depth_only else ["time"] if args.time_only else ["time", "depth"]):
                        row = measure([str(binary), "baseline" if budget == 30 else "optimized", fen, mode, str(seconds), str(depth), str(budget * 1_000_000)])
                        row.update(case=name, budget_mb=budget, mode=mode, repetition=repetition)
                        data["runs"].append(row)
                        print(json.dumps(row), flush=True)
                        args.output.parent.mkdir(parents=True, exist_ok=True)
                        args.output.write_text(json.dumps(data, indent=2) + "\n")
    # Equal fixed depths must produce equal scores across all memory tiers.
    for name, _, _ in cases:
        scores = {r["score"] for r in data["runs"] if r["case"] == name and r["mode"] == "depth"}
        assert args.ponder or args.time_only or len(scores) == 1, (name, scores)
    args.output.with_suffix(".md").write_text(report(data))
    print(f"Results: {args.output.with_suffix('.md')}", flush=True)


if __name__ == "__main__":
    main()
