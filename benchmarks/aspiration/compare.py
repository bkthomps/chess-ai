"""Aspiration windows versus full-window PVS; stdlib/rustc only."""
import argparse
import hashlib
import json
import pathlib
import platform
import random
import re
import runpy
import statistics
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
BASELINE = "98d2f0d"
ORDERING = runpy.run_path(str(ROOT / "benchmarks/ordering/compare.py"))
COMMON = ORDERING["COMMON"]
CASES = ORDERING["CASES"]
VARIANTS = ["baseline", "aspiration"]


def report(data):
    lines = [ORDERING["report"](data), f"Initial aspiration half-width: {data['initial_margin_cp']} centipawns; enabled from depth {data['minimum_depth']}.", "", "## Root-window attempts", "",
             "Counts are medians for the aspiration variant. Narrow attempts include retries; failed windows are retried at the same depth. Ponder counts cover both the warm search and computer search.", "",
             "| Position | Mode | Narrow attempts | Fail low | Fail high |",
             "|---|---|---:|---:|---:|"]
    for name, _, _ in data["cases"]:
        for mode in data["modes"]:
            rows = [r for r in data["runs"] if r["case"] == name and r["mode"] == mode and r["variant"] == "aspiration"]
            values = [statistics.median(r[k] for r in rows) for k in ["narrow_attempts", "fail_low", "fail_high"]]
            lines.append(f"| {name} | {mode} | {values[0]:g} | {values[1]:g} | {values[2]:g} |")
    return "\n".join(lines) + "\n"


def compile_benchmark(directory, verification=False, margin=64, variants=None, transform=None):
    variants = VARIANTS if variants is None else variants
    assert all(v in ["baseline", "aspiration", "current_full", "margin32", "margin128"] for v in variants)
    old = subprocess.check_output(["git", "show", f"{BASELINE}:src/engine.rs"], cwd=ROOT, text=True)
    new = (ROOT / "benchmarks/aspiration/experimental-engine.rs").read_text()
    shared = {}
    for name in ["chess", "tables", "cache", "ordering"]:
        source = (ROOT / f"src/{name}.rs").read_text()
        before = subprocess.check_output(["git", "show", f"{BASELINE}:src/{name}.rs"], cwd=ROOT, text=True)
        if source != before:
            raise RuntimeError(f"src/{name}.rs changed since {BASELINE}; this study must isolate aspiration windows")
        shared[name] = source
    driver = COMMON["DRIVER"].replace("memory: usize)", "memory: usize, thread_limit: usize)")
    driver = driver.replace(
        "let threads = search_threads(thread::available_parallelism().map_or(1, usize::from)).min(64);",
        "let available = SearchCache::default().threads; let threads = if thread_limit == 0 { available } else { thread_limit.min(available).max(1) };")
    driver = driver.replace("__CONSTRUCTOR__", "SearchCache::with_threads_and_memory(threads, memory)")
    driver = driver.replace("__INITIALIZE__", "cache.table.initialize();").replace("__BYTES__", "cache.table.bytes()").replace("__TOUCH__", "")
    if verification:
        driver = driver.replace("best.unwrap()", 'best.map(|m| m.to_string()).unwrap_or_else(|| "-".into())')
    modules = [(name, ROOT / f"src/{name}.rs") for name in ["tables", "chess"]]
    hashes = {}
    for variant in variants:
        source = old if variant == "baseline" else new
        folder = directory / variant
        folder.mkdir()
        if variant != "baseline":
            width = {"margin32": 32, "margin128": 128}.get(variant, margin)
            source, count = re.subn(r"let mut margin = \d+;", f"let mut margin = {width};", source)
            assert count == 1
            if variant == "current_full":
                guard = "Some(score) if depth >= 4 && score.abs() < MATE - 256"
                assert source.count(guard) == 1
                source = source.replace(guard, "Some(score) if false && depth >= 4 && score.abs() < MATE - 256")
        hashes[variant] = hashlib.sha256((source + "".join(shared.values())).encode()).hexdigest()
        variant_driver = driver
        if variant != "baseline":
            full_iteration = """result = if threads > 1 && d >= 4 {
                s.parallel_root(&p, d, &legal, &mut helpers)
            } else { s.negamax(&p, d, 0, -INF, INF) };"""
            assert full_iteration in driver
            variant_driver = driver.replace(full_iteration,
                "result = s.iteration(&p, d, &legal, &mut helpers, (d > 1).then_some(result.0));")
            attempt = "        loop {\n            let result = if self.threads"
            assert source.count(attempt) == 1
            source = source.replace(attempt,
                "        loop {\n            if alpha != -INF || beta != INF { BENCH_NARROW.fetch_add(1, Ordering::Relaxed); }\n            let result = if self.threads")
            failed = "            margin = (margin * 2).min(INF);"
            assert source.count(failed) == 1
            source = source.replace(failed,
                "            if result.0 <= alpha { BENCH_LOW.fetch_add(1, Ordering::Relaxed); } else { BENCH_HIGH.fetch_add(1, Ordering::Relaxed); }\n" + failed)
        # Counters run once per root attempt, never inside the node loop. They
        # exist only in temporary benchmark sources, including API searches.
        counters = """
static BENCH_NARROW: AtomicU64 = AtomicU64::new(0);
static BENCH_LOW: AtomicU64 = AtomicU64::new(0);
static BENCH_HIGH: AtomicU64 = AtomicU64::new(0);
"""
        prefix = r'''let narrow_attempts = BENCH_NARROW.load(Ordering::Relaxed); let fail_low = BENCH_LOW.load(Ordering::Relaxed); let fail_high = BENCH_HIGH.load(Ordering::Relaxed); println!("{{\"narrow_attempts\":{narrow_attempts},\"fail_low\":{fail_low},\"fail_high\":{fail_high},'''
        variant_driver = variant_driver.replace('println!("{{', prefix)
        if verification:
            variant_driver = variant_driver.replace('Duration::from_secs(60)', 'Duration::from_secs(180)').replace('exceeded 60 seconds', 'exceeded 180 seconds')
        if transform is not None:
            source, variant_driver = transform(source, variant_driver)
        (folder / "engine.rs").write_text(source + counters + variant_driver)
        for name in ["cache", "ordering"]:
            (folder / f"{name}.rs").write_text(shared[name])
        modules.append((variant, folder / "engine.rs"))
    rust = "#![allow(dead_code, unused_variables)]\n" + "\n".join(
        f"#[path={json.dumps(str(path), ensure_ascii=False)}] mod {name};" for name, path in modules)
    rust += '''
fn main() {
 let a: Vec<_> = std::env::args().collect();
 let run = match a[1].as_str() { "baseline" => baseline::memory_benchmark, "aspiration" => aspiration::memory_benchmark, _ => panic!("unknown variant") };
 run(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), 100_000_000, a[6].parse().unwrap());
}
'''
    if variants != VARIANTS:
        arms = ", ".join(f'"{v}" => {v}::memory_benchmark' for v in variants)
        rust = rust.replace('"baseline" => baseline::memory_benchmark, "aspiration" => aspiration::memory_benchmark', arms)
    source = directory / "main.rs"
    source.write_text(rust)
    binary = directory / "aspiration-bench"
    subprocess.run(["rustc", "--edition=2021", "-C", "opt-level=3", "-C", "codegen-units=1", str(source), "-o", str(binary)], check=True)
    return binary, hashes


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--modes", nargs="+", choices=["depth", "time", "ponder"], default=["depth", "time"])
    parser.add_argument("--threads", type=int, default=0, help="0 uses the application's automatic CPU headroom policy")
    parser.add_argument("--seconds", type=int, default=3)
    parser.add_argument("--margin", type=int, choices=[32, 64, 128, 256], default=64, help="initial aspiration half-width in centipawns; temporary source override")
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--depth-offset", type=int, default=0)
    parser.add_argument("--case", choices=["all"] + [c[0] for c in CASES], default="all")
    parser.add_argument("--quick", action="store_true")
    parser.add_argument("--fixtures", action="store_true", help="verify depth-four scores across the rule-regression FEN corpus")
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "benchmarks/aspiration/results.json")
    args = parser.parse_args()
    repetitions, seconds = (1, 1) if args.quick else (args.repetitions, args.seconds)
    offset = args.depth_offset - (2 if args.quick else 0)
    cases = [(n, f, d + offset) for n, f, d in CASES if args.case in ["all", n]]
    if args.fixtures:
        if args.modes != ["depth"] or args.threads != 1:
            parser.error("fixture verification requires --modes depth --threads 1")
        fens = set()
        for name in ["regression_perft.tsv", "regression_moves.tsv", "regression_castling.tsv", "regression_terminal.tsv"]:
            for line in (ROOT / "tests/fixtures" / name).read_text().splitlines():
                if line and not line.startswith("#"):
                    fens.add(line.split("\t")[-1])
        cases = [(f"fixture_{index:03d}", fen, 4) for index, fen in enumerate(sorted(fens))]
    if repetitions < 1 or seconds < 1 or args.threads < 0 or any(not 1 <= d <= 255 for _, _, d in cases):
        parser.error("positive durations/repetitions and depths 1–255 are required")
    data = {"title": "Aspiration window measurements", "baseline": BASELINE,
            "machine": platform.platform(), "threads_requested": args.threads,
            "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "memory_budget": 100_000_000, "seconds": seconds, "repetitions": repetitions,
            "cases": cases, "variants": VARIANTS, "modes": args.modes, "runs": [],
            "initial_margin_cp": args.margin, "minimum_depth": 4,
            "full_window_margin_cp": 1024, "benchmark_root_counters": True}
    rng = random.Random(0x415350)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="chess-aspiration-") as tmp:
        binary, hashes = compile_benchmark(pathlib.Path(tmp), args.fixtures, args.margin)
        data["source_sha256"] = hashes
        for name, fen, depth in cases:
            for repetition in range(repetitions):
                variants = VARIANTS.copy()
                rng.shuffle(variants)
                for variant in variants:
                    for mode in args.modes:
                        row = COMMON["measure"]([str(binary), variant, fen, mode, str(seconds), str(depth), str(args.threads)])
                        row.update(case=name, mode=mode, variant=variant, repetition=repetition)
                        data["runs"].append(row)
                        print(json.dumps(row), flush=True)
                        args.output.write_text(json.dumps(data, indent=2) + "\n")
    for name, _, _ in cases:
        scores = {r["score"] for r in data["runs"] if r["case"] == name and r["mode"] == "depth"}
        assert "depth" not in args.modes or len(scores) == 1, (name, scores)
        if args.threads == 1:
            for variant in VARIANTS:
                results = {(r["nodes"], r["hits"], r["score"], r["best"]) for r in data["runs"] if r["case"] == name and r["variant"] == variant and r["mode"] == "depth"}
                assert len(results) <= 1, (name, variant, results)
    args.output.with_suffix(".md").write_text(report(data))
    print(f"Results: {args.output.with_suffix('.md')}", flush=True)


if __name__ == "__main__":
    main()
