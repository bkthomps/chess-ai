"""Investigate the tactical sample without modifying production engine sources."""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import random
import runpy
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
STUDY = runpy.run_path(str(ROOT / "benchmarks/aspiration/compare.py"))
FEN = next(f for n, f, _ in STUDY["CASES"] if n == "tactics")
CONTROLS = ["baseline", "current_full", "aspiration", "margin32", "margin128"]


def save(path, data):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(data, indent=2) + "\n")
    temporary.replace(path)


def measure(command):
    started = time.perf_counter()
    p = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    usage = None
    if hasattr(os, "wait4"):
        stdout, stderr = p.stdout.read(), p.stderr.read()
        _, status, usage = os.wait4(p.pid, 0)
        p.returncode = os.waitstatus_to_exitcode(status)
    else:
        stdout, stderr = p.communicate()
    if p.returncode:
        raise RuntimeError(f"{command}\n{stderr}")
    row = json.loads(stdout)
    row["process_wall_ms"] = (time.perf_counter() - started) * 1000
    row["cpu_ms"] = (usage.ru_utime + usage.ru_stime) * 1000 if usage else None
    row["involuntary_switches"] = usage.ru_nivcsw if usage else None
    row["voluntary_switches"] = usage.ru_nvcsw if usage else None
    row["peak_rss_bytes"] = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024) if usage else None
    return row


def summary(rows, repetitions, rng):
    baseline = {r["repetition"]: r for r in rows if r["variant"] == "baseline"}
    output = {}
    for variant in sorted({r["variant"] for r in rows}):
        group = [r for r in rows if r["variant"] == variant]
        costs = [100 * (r["elapsed_ms"] / baseline[r["repetition"]]["elapsed_ms"] - 1) for r in group]
        samples = sorted(statistics.median(rng.choices(costs, k=repetitions)) for _ in range(10_000))
        output[variant] = {
            "median_ms": statistics.median(r["elapsed_ms"] for r in group),
            "range_ms": [min(r["elapsed_ms"] for r in group), max(r["elapsed_ms"] for r in group)],
            "median_paired_cost_pct": statistics.median(costs),
            "paired_cost_bootstrap_95_pct": [samples[249], samples[9749]],
            "median_nodes": statistics.median(r["nodes"] for r in group),
            "depth_counts": {str(d): sum(r["depth"] == d for r in group) for d in sorted({r["depth"] for r in group})},
            "failures": sorted({r["fail_low"] + r["fail_high"] for r in group}),
            "median_cpu_ms": statistics.median(r["cpu_ms"] for r in group) if group[0]["cpu_ms"] is not None else None,
        }
    return output


def report(data):
    warmup = "Timing stages discard one warm-up group; diagnostic counters need no warm-up."
    lines = [f"# Tactical aspiration follow-up: {data['kind']}", "",
             f"Baseline is PVS at `98d2f0d`; archived experimental aspiration uses ±64 centipawns. The application uses PVS without aspiration. Fresh processes, cold caches, 100 MB total budget. {warmup} Variant names refer to the archived experiment. No production source changes.", ""]
    if data["kind"] == "profile":
        lines += ["Diagnostic counters only; timings from these instrumented builds are not performance evidence.", "",
                  "| Stage | Variant | Nodes | Negamax calls | Quiescence calls | PVS probes | PVS re-searches | Depth-zero calls | Null-window negamax | Positive-depth null-window | Null-window quiescence |",
                  "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
        for row in data["runs"]:
            counts = row["profile_counts"]
            values = " | ".join(f"{n:,}" for n in counts)
            lines.append(f"| {row['stage']} | {row['variant']} | {row['nodes']:,} | {values} |")
    else:
        lines += ["Positive paired cost means slower than the baseline in the same repetition. Intervals are descriptive percentile bootstrap intervals over pairs (10,000 resamples); adjacent measurements may be correlated and results are specific to this machine/build.", "",
                  "| Stage | Variant | Repetitions | Median ms | Range ms | Paired cost % | Bootstrap 95% range % | Nodes, median | Completed depths |",
                  "|---|---|---:|---:|---:|---:|---:|---:|---|"]
        for stage in data["stages"]:
            for variant, result in stage.get("summary", {}).items():
                lo, hi = result["range_ms"]
                ci_lo, ci_hi = result["paired_cost_bootstrap_95_pct"]
                depths = ", ".join(f"{d}: {count}" for d, count in result["depth_counts"].items())
                lines.append(f"| {stage['name']} | {variant} | {stage['repetitions']} | {result['median_ms']:.2f} | {lo:.1f}–{hi:.1f} | {result['median_paired_cost_pct']:+.2f} | {ci_lo:+.2f} to {ci_hi:+.2f} | {result['median_nodes']:,.0f} | {depths} |")
    return "\n".join(lines) + "\n"


def profiling(source, driver):
    # Diagnostic builds only. Per-node counters deliberately do not contribute
    # performance evidence; compare their node counts with the timing builds.
    for function, index in [("negamax", 0), ("quiescence_moves", 1)]:
        start = source.index(f"    fn {function}(\n")
        body = source.index(" {\n", start) + 3
        increment = f"        bench_profile({index});\n"
        if function == "negamax":
            increment += "        if depth == 0 { bench_profile(4); }\n        if beta - alpha == 1 { bench_profile(5); if depth > 0 { bench_profile(6); } }\n"
        else:
            increment += "        if window.1 - window.0 == 1 { bench_profile(7); }\n"
        source = source[:body] + increment + source[body:]
    needle = "                let probe = -self\n"
    assert source.count(needle) == 1
    source = source.replace(needle, "                bench_profile(2);\n" + needle)
    needle = "                if !self.aborted && probe > alpha && probe < beta {\n"
    assert source.count(needle) == 1
    source = source.replace(needle, needle + "                    bench_profile(3);\n")
    source += '''
std::thread_local! { static BENCH_PROFILE: std::cell::Cell<[u64; 8]> = std::cell::Cell::new([0; 8]); }
fn bench_profile(index: usize) { BENCH_PROFILE.with(|cell| { let mut counts = cell.get(); counts[index] += 1; cell.set(counts); }); }
fn bench_profile_counts() -> [u64; 8] { BENCH_PROFILE.with(std::cell::Cell::get) }
'''
    driver = driver.replace("    let p = Position::from_fen", "    let mut bench_trace: Vec<String> = Vec::new();\n    let p = Position::from_fen")
    needle = '            assert!(!s.aborted, "fixed-depth benchmark exceeded 60 seconds");'
    assert driver.count(needle) == 1
    driver = driver.replace(needle, needle + r'''
            let totals = s.totals();
            bench_trace.push(format!(r#"{{"depth":{},"score":{},"nodes":{},"hits":{},"profile":{:?}}}"#, d, result.0, totals.0, totals.1, bench_profile_counts()));''')
    prefix = r'''let profile_counts = format!("{:?}", bench_profile_counts()); let trace = format!("[{}]", bench_trace.join(",")); println!("{{\"profile_counts\":{profile_counts},\"trace\":{trace},'''
    driver = driver.replace('println!("{{', prefix)
    return source, driver


def run_stages(folder, kind, stages, variants, output, transform=None):
    data = {"baseline": STUDY["BASELINE"], "fen": FEN, "kind": kind,
            "machine": platform.platform(), "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "memory_budget": 100_000_000, "initial_margin_cp": 64,
            "source_sha256": {}, "stages": [], "warmups": [], "runs": []}
    rng = random.Random(0x54414354)
    binary, data["source_sha256"] = STUDY["compile_benchmark"](folder, variants=variants, transform=transform)
    for name, mode, depth, threads, repetitions in stages:
        print(f"Starting {kind}: {name} ({repetitions} pairs/groups)", flush=True)
        stage = dict(name=name, mode=mode, depth=depth, threads_requested=threads, repetitions=repetitions)
        data["stages"].append(stage)
        orders = [variants.copy() if i % 2 == 0 else list(reversed(variants)) for i in range(repetitions)]
        if len(variants) > 2:
            for order in orders:
                rng.shuffle(order)
        rng.shuffle(orders)
        groups = ([(-1, variants)] if transform is None else []) + list(enumerate(orders))
        for repetition, order in groups:
            for ordinal, variant in enumerate(order):
                row = measure([str(binary), variant, FEN, mode, "3", str(depth), str(threads)])
                row.update(stage=name, mode=mode, variant=variant, repetition=repetition, ordinal=ordinal)
                data["warmups" if repetition == -1 else "runs"].append(row)
                save(output, data)
        rows = [r for r in data["runs"] if r["stage"] == name]
        if mode == "depth":
            assert len({r["score"] for r in rows}) == 1, name
            if threads == 1:
                for v in variants:
                    assert len({(r["nodes"], r["hits"], r["score"], r["best"]) for r in rows if r["variant"] == v}) == 1, (name, v)
        if transform is None:
            stage["summary"] = summary(rows, repetitions, rng)
        else:
            stage["profile_counter_names"] = ["negamax_calls", "quiescence_calls", "pvs_probes", "pvs_full_researches", "depth_zero_negamax_calls", "null_window_negamax_calls", "positive_depth_null_window_calls", "null_window_quiescence_calls"]
        save(output, data)
        print(f"Completed {kind}: {name}", flush=True)
    output.with_suffix(".md").write_text(report(data))
    return data


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--study", choices=["all", "timing", "controls", "profile"], default="all")
    parser.add_argument("--quick", action="store_true", help="one measured group per stage, for checking the harness")
    args = parser.parse_args()
    stages = [
        ("serial-depth7", "depth", 7, 1, 31),
        ("parallel-depth7", "depth", 7, 0, 31),
        ("serial-depth8", "depth", 8, 1, 9),
        ("parallel-depth8", "depth", 8, 0, 9),
        ("serial-time3", "time", 8, 1, 7),
        ("parallel-time3", "time", 8, 0, 11),
    ]
    controls = [("serial-depth7", "depth", 7, 1, 9), ("parallel-depth7", "depth", 7, 0, 9)]
    profiles = [("serial-depth8", "depth", 8, 1, 1)]
    if args.quick:
        stages = [(n, m, d, t, 1) for n, m, d, t, _ in stages]
        controls = [(n, m, d, t, 1) for n, m, d, t, _ in controls]
    original_hash = hashlib.sha256((ROOT / "src/engine.rs").read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="chess-tactical-") as tmp:
        for kind, selected, variants, transform in [
            ("timing", stages, STUDY["VARIANTS"], None),
            ("controls", controls, CONTROLS, None),
            ("profile", profiles, ["baseline", "aspiration", "margin32"], profiling),
        ]:
            if args.study in ["all", kind]:
                folder = pathlib.Path(tmp) / kind
                folder.mkdir()
                output = ROOT / f"benchmarks/aspiration/tactical-{kind}.json"
                run_stages(folder, kind, selected, variants, output, transform)
    assert hashlib.sha256((ROOT / "src/engine.rs").read_bytes()).hexdigest() == original_hash


if __name__ == "__main__":
    main()
