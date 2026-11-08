"""Leading-branch parallel load balancing versus root-only PVS; stdlib/rustc only."""
import argparse
import hashlib
import json
import pathlib
import platform
import random
import runpy
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
BASELINE = "ac1d2b3"
ORDERING = runpy.run_path(str(ROOT / "benchmarks/ordering/compare.py"))
COMMON = ORDERING["COMMON"]
CASES = ORDERING["CASES"]
VARIANTS = ["baseline", "balanced"]


def report(data):
    text = ORDERING["report"](data)
    text += f"\nCompiler LTO: {'fat' if data.get('lto', False) else 'disabled'}.\n"
    if data.get("verification_ply"):
        text += "\nVerification compares parallel subtree entry with serial PVS at ply one; the baseline reference does not use helpers at these shallow targets.\n"
    return text


def compile_benchmark(directory, verification=False, transform=None, lto=False):
    old = subprocess.check_output(["git", "show", f"{BASELINE}:src/engine.rs"], cwd=ROOT, text=True)
    new = (ROOT / "src/engine.rs").read_text()
    shared = {}
    for name in ["chess", "tables", "cache", "ordering"]:
        source = (ROOT / f"src/{name}.rs").read_text()
        before = subprocess.check_output(["git", "show", f"{BASELINE}:src/{name}.rs"], cwd=ROOT, text=True)
        if source != before:
            raise RuntimeError(f"src/{name}.rs changed since {BASELINE}; this study must isolate parallel load balancing")
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
    for variant, source in [("baseline", old), ("balanced", new)]:
        folder = directory / variant
        folder.mkdir()
        variant_driver = driver
        if verification:
            # Exercise the new subtree entry at a nonzero ply, including
            # terminals/draws, against serial PVS at the same ply.
            variant_driver = variant_driver.replace("s.negamax(&p, d, 0, -INF, INF)", "s.negamax(&p, d, 1, -INF, INF)")
            if variant == "balanced":
                original_loop = """result = if threads > 1 && d >= 4 {
                s.parallel_root(&p, d, &legal, &mut helpers)
            } else { s.negamax(&p, d, 1, -INF, INF) };"""
                assert original_loop in variant_driver
                variant_driver = variant_driver.replace(original_loop,
                    "result = s.parallel_first(&p, d, 1, &mut helpers);")
        hashes[variant] = hashlib.sha256((source + "".join(shared.values())).encode()).hexdigest()
        if transform is not None:
            source, variant_driver = transform(source, variant_driver, variant)
        (folder / "engine.rs").write_text(source + variant_driver)
        for name in ["cache", "ordering"]:
            (folder / f"{name}.rs").write_text(shared[name])
        modules.append((variant, folder / "engine.rs"))
    rust = "#![allow(dead_code, unused_variables)]\n" + "\n".join(
        f"#[path={json.dumps(str(path), ensure_ascii=False)}] mod {name};" for name, path in modules)
    rust += '''
fn main() {
 let a: Vec<_> = std::env::args().collect();
 let run = match a[1].as_str() { "baseline" => baseline::memory_benchmark, "balanced" => balanced::memory_benchmark, _ => panic!("unknown variant") };
 run(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), 100_000_000, a[6].parse().unwrap());
}
'''
    source = directory / "main.rs"
    source.write_text(rust)
    binary = directory / "parallel-bench"
    flags = ["rustc", "--edition=2021", "-C", "opt-level=3", "-C", "codegen-units=1"]
    if lto:
        flags += ["-C", "lto=fat"]
    subprocess.run(flags + [str(source), "-o", str(binary)], check=True)
    return binary, hashes


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--modes", nargs="+", choices=["depth", "time", "ponder"], default=["depth", "time"])
    parser.add_argument("--threads", type=int, default=0, help="0 uses the application's automatic CPU headroom policy")
    parser.add_argument("--seconds", type=int, default=3)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--depth-offset", type=int, default=0)
    parser.add_argument("--case", choices=["all"] + [c[0] for c in CASES], default="all")
    parser.add_argument("--quick", action="store_true")
    parser.add_argument("--lto", action="store_true", help="enable fat LTO, matching the application's release optimization setting")
    parser.add_argument("--fixtures", action="store_true", help="verify depth-three scores across the rule-regression FEN corpus")
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "benchmarks/parallel/results.json")
    args = parser.parse_args()
    repetitions, seconds = (1, 1) if args.quick else (args.repetitions, args.seconds)
    offset = args.depth_offset - (2 if args.quick else 0)
    cases = [(n, f, d + offset) for n, f, d in CASES if args.case in ["all", n]]
    if args.fixtures:
        if args.modes != ["depth"]:
            parser.error("fixture verification requires --modes depth")
        fens = set()
        for name in ["regression_perft.tsv", "regression_moves.tsv", "regression_castling.tsv", "regression_terminal.tsv"]:
            for line in (ROOT / "tests/fixtures" / name).read_text().splitlines():
                if line and not line.startswith("#"):
                    fens.add(line.split("\t")[-1])
        cases = [(f"fixture_{index:03d}", fen, 3) for index, fen in enumerate(sorted(fens))]
    if repetitions < 1 or seconds < 1 or args.threads < 0 or any(not 1 <= d <= 255 for _, _, d in cases):
        parser.error("positive durations/repetitions and depths 1–255 are required")
    data = {"title": "Parallel load balancing measurements", "baseline": BASELINE,
            "machine": platform.platform(), "threads_requested": args.threads,
            "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "memory_budget": 100_000_000, "seconds": seconds, "repetitions": repetitions,
            "cases": cases, "variants": VARIANTS, "modes": args.modes, "runs": [],
            "lto": args.lto, "verification_ply": 1 if args.fixtures else 0}
    rng = random.Random(0x504152)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="chess-parallel-") as tmp:
        binary, hashes = compile_benchmark(pathlib.Path(tmp), args.fixtures, lto=args.lto)
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
            if not args.fixtures and "depth" in args.modes:
                serial = {(r["nodes"], r["hits"], r["score"], r["best"]) for r in data["runs"] if r["case"] == name and r["mode"] == "depth"}
                assert len(serial) == 1, ("single-thread algorithm changed", name, serial)
    args.output.with_suffix(".md").write_text(report(data))
    print(f"Results: {args.output.with_suffix('.md')}", flush=True)


if __name__ == "__main__":
    main()
