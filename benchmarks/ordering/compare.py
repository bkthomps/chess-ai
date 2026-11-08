"""Before/after quiet-move ordering benchmark; Python stdlib and rustc only."""
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
BASELINE = "5f76829"
CASES = [
    ("opening", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 9),
    ("middlegame", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 7),
    ("endgame", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 12),
    ("tactics", "rnbqk2r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQkq - 1 8", 7),
    ("pawns", "8/5pk1/2p3p1/1p1pP2p/1P1P1P1P/2P3K1/8/8 w - - 0 1", 12),
]
COMMON = runpy.run_path(str(ROOT / "benchmarks/memory/compare.py"))


def compile_benchmark(directory, variants, verification=False):
    old = subprocess.check_output(["git", "show", f"{BASELINE}:src/engine.rs"], cwd=ROOT, text=True)
    new = (ROOT / "src/engine.rs").read_text()
    ordering = (ROOT / "src/ordering.rs").read_text()
    cache = (ROOT / "src/cache.rs").read_text()
    driver = COMMON["DRIVER"].replace("memory: usize)", "memory: usize, thread_limit: usize)")
    driver = driver.replace(
        "let threads = search_threads(thread::available_parallelism().map_or(1, usize::from)).min(64);",
        "let available = SearchCache::default().threads; let threads = if thread_limit == 0 { available } else { thread_limit.min(available).max(1) };")
    driver = driver.replace("__CONSTRUCTOR__", "SearchCache::with_threads_and_memory(threads, memory)")
    driver = driver.replace("__INITIALIZE__", "cache.table.initialize();").replace("__BYTES__", "cache.table.bytes()").replace("__TOUCH__", "")
    if verification:
        # Terminal/drawn fixtures can legitimately return no best move.
        driver = driver.replace("best.unwrap()", 'best.map(|m| m.to_string()).unwrap_or_else(|| "-".into())')
    modules = [("tables", ROOT / "src/tables.rs"), ("chess", ROOT / "src/chess.rs")]
    hashes = {}
    for variant in variants:
        folder = directory / variant
        folder.mkdir()
        source = old if variant == "baseline" else new
        variant_ordering = ordering
        if variant == "killers": variant_ordering = variant_ordering.replace("const HISTORY_ENABLED: bool = true;", "const HISTORY_ENABLED: bool = false;")
        if variant == "history": variant_ordering = variant_ordering.replace("const KILLERS_ENABLED: bool = true;", "const KILLERS_ENABLED: bool = false;")
        if variant == "soft":
            variant_ordering = variant_ordering.replace("return 19_000;", "return 2048 + i32::from(history[m.from][m.to]);")
            variant_ordering = variant_ordering.replace("return 18_000;", "return 1024 + i32::from(history[m.from][m.to]);")
        (folder / "engine.rs").write_text(source + driver)
        (folder / "cache.rs").write_text(cache)
        (folder / "ordering.rs").write_text(variant_ordering)
        hashes[variant] = hashlib.sha256((source + cache + (variant_ordering if variant != "baseline" else "")).encode()).hexdigest()
        modules.append((variant, folder / "engine.rs"))
    rust = "#![allow(dead_code, unused_variables)]\n" + "\n".join(
        f"#[path={json.dumps(str(path), ensure_ascii=False)}] mod {name};" for name, path in modules)
    arms = ",".join(f'"{v}" => {v}::memory_benchmark' for v in variants)
    rust += '''
fn main() {
 let a: Vec<_> = std::env::args().collect();
 let run = match a[1].as_str() { __ARMS__, _ => panic!("unknown variant") };
 run(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), 100_000_000, a[6].parse().unwrap());
}
'''.replace("__ARMS__", arms)
    source = directory / "main.rs"
    source.write_text(rust)
    binary = directory / "ordering-bench"
    subprocess.run(["rustc", "--edition=2021", "-C", "opt-level=3", "-C", "codegen-units=1", str(source), "-o", str(binary)], check=True)
    return binary, hashes


def report(data):
    med = statistics.median
    lines = [f"# {data.get('title', 'Quiet-move ordering measurements')}", "",
             f"Baseline `{data['baseline']}`. {len(data['cases'])} positions, {data['repetitions']} fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.", "",
             "| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |",
             "|---|---|---|---:|---:|---:|---:|---:|---:|"]
    for name, _, _ in data["cases"]:
        for mode in data["modes"]:
            for variant in data["variants"]:
                rows = [r for r in data["runs"] if r["case"] == name and r["mode"] == mode and r["variant"] == variant]
                times = [r["elapsed_ms"] for r in rows]
                peak = med(r["peak_rss_bytes"] / 1e6 for r in rows) if rows[0]["peak_rss_bytes"] is not None else float("nan")
                lines.append(f"| {name} | {mode} | {variant} | {med(r['depth'] for r in rows):g} | {med(times):.2f} | {min(times):.1f}–{max(times):.1f} | {med(r['nodes'] for r in rows):,.0f} | {med(r['cache_bytes'] for r in rows)/1e6:.1f} | {peak:.1f} |")
    lines += ["", f"Time-limited searches use {data['seconds']} seconds. Fixed-depth targets: " + ", ".join(f"{n} {d}" for n, _, d in data["cases"]) + ".",
              "Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.", ""]
    if "ponder" in data["modes"]:
        lines += ["Pondering is bounded to the time allowance above, followed by a two-second computer search after the recorded human move.", "",
                  "| Position | Variant | Human move | Ponder depth, median | Computer depth, median |", "|---|---|---|---:|---:|"]
        for name, _, _ in data["cases"]:
            for variant in data["variants"]:
                rows = [r for r in data["runs"] if r["case"] == name and r["mode"] == "ponder" and r["variant"] == variant]
                lines.append(f"| {name} | {variant} | {rows[0]['human_move']} | {med(r['ponder_depth'] for r in rows):g} | {med(r['depth'] for r in rows):g} |")
        lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--variants", nargs="+", choices=["baseline", "killers", "history", "both", "soft"], default=["baseline", "both"])
    parser.add_argument("--modes", nargs="+", choices=["depth", "time", "ponder"], default=["depth", "time"])
    parser.add_argument("--threads", type=int, default=0, help="0 uses the application's automatic CPU headroom policy")
    parser.add_argument("--seconds", type=int, default=3)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--depth-offset", type=int, default=0)
    parser.add_argument("--case", choices=["all"] + [c[0] for c in CASES], default="all")
    parser.add_argument("--quick", action="store_true")
    parser.add_argument("--fixtures", action="store_true", help="compare shallow scores across the rule-regression FEN corpus")
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "benchmarks/ordering/results.json")
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
                if line and not line.startswith("#"): fens.add(line.split("\t")[-1])
        cases = [(f"fixture_{index:03d}", fen, 3) for index, fen in enumerate(sorted(fens))]
    if repetitions < 1 or seconds < 1 or args.threads < 0 or any(not 1 <= d <= 255 for _, _, d in cases):
        parser.error("positive durations/repetitions and depths 1–255 are required")
    data = {"baseline": BASELINE, "machine": platform.platform(), "threads_requested": args.threads,
            "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "memory_budget": 100_000_000, "seconds": seconds, "repetitions": repetitions,
            "cases": cases, "variants": args.variants, "modes": args.modes, "runs": []}
    rng = random.Random(0x4D4F5645)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="chess-ordering-") as tmp:
        binary, hashes = compile_benchmark(pathlib.Path(tmp), args.variants, args.fixtures)
        data["source_sha256"] = hashes
        for name, fen, depth in cases:
            for repetition in range(repetitions):
                variants = args.variants.copy()
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
            for variant in args.variants:
                results = {(r["nodes"], r["hits"], r["score"], r["best"]) for r in data["runs"] if r["case"] == name and r["variant"] == variant and r["mode"] == "depth"}
                assert len(results) <= 1, (name, variant, results)
    args.output.with_suffix(".md").write_text(report(data))
    print(f"Results: {args.output.with_suffix('.md')}", flush=True)


if __name__ == "__main__": main()
