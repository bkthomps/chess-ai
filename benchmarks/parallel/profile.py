"""Root-phase diagnostics for load balancing; instrumentation stays in temp copies."""
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

ROOT = pathlib.Path(__file__).resolve().parents[2]
STUDY = runpy.run_path(str(ROOT / "benchmarks/parallel/compare.py"))


def transform(source, driver, variant):
    needle = "        let key = self.table_key(position);"
    assert source.count(needle) == 1
    source = source.replace(needle, "        let bench_phase_start = Instant::now();\n" + needle)
    needle = "        self.pop_position(previous);\n        if self.aborted {"
    assert source.count(needle) == 1
    nested = "ply < 2 && depth >= 6 && !helpers.is_empty()" if variant == "balanced" else "false"
    replacement = "        self.pop_position(previous);\n        let bench_first_ns = bench_phase_start.elapsed().as_nanos() as u64;\n"
    replacement += f"        if !({nested}) {{ BENCH_FIRST_SERIAL.fetch_add(bench_first_ns, Ordering::Relaxed); }}\n        if self.aborted {{"
    source = source.replace(needle, replacement)
    needle = "                        worker.root_jobs(work);"
    assert source.count(needle) == 1
    source = source.replace(needle, "                        let _bench_worker = BenchWorker::new();\n" + needle)
    needle = "        (score, Some(best))"
    assert source.count(needle) == 1
    ply = "ply" if variant == "balanced" else "0"
    source = source.replace(needle, f"        bench_phase({ply}, depth, legal.len(), bench_first_ns, bench_phase_start.elapsed().as_nanos() as u64);\n" + needle)
    source += '''
static BENCH_ACTIVE: AtomicU64 = AtomicU64::new(0);
static BENCH_MAX: AtomicU64 = AtomicU64::new(0);
static BENCH_HELPER_NS: AtomicU64 = AtomicU64::new(0);
static BENCH_FIRST_SERIAL: AtomicU64 = AtomicU64::new(0);
static BENCH_PHASES: Mutex<Vec<(i32, u8, usize, u64, u64)>> = Mutex::new(Vec::new());
struct BenchWorker(Instant);
impl BenchWorker {
    fn new() -> Self {
        let count = BENCH_ACTIVE.fetch_add(1, Ordering::Relaxed) + 1;
        BENCH_MAX.fetch_max(count, Ordering::Relaxed);
        Self(Instant::now())
    }
}
impl Drop for BenchWorker {
    fn drop(&mut self) {
        BENCH_HELPER_NS.fetch_add(self.0.elapsed().as_nanos() as u64, Ordering::Relaxed);
        BENCH_ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}
fn bench_phase(ply: i32, depth: u8, moves: usize, first_ns: u64, total_ns: u64) {
    BENCH_PHASES.lock().unwrap().push((ply, depth, moves, first_ns, total_ns));
}
fn bench_diagnostics() -> String {
    assert_eq!(BENCH_ACTIVE.load(Ordering::Relaxed), 0);
    let rows = BENCH_PHASES.lock().unwrap();
    let mut phases = Vec::new();
    for &(ply, depth, moves, first, total) in rows.iter() {
        phases.push(format!(r#"{{"ply":{ply},"depth":{depth},"moves":{moves},"first_ns":{first},"total_ns":{total}}}"#));
    }
    format!(r#"{{"max_helpers":{},"helper_lifetime_ns":{},"first_serial_ns":{},"phases":[{}]}}"#,
        BENCH_MAX.load(Ordering::Relaxed), BENCH_HELPER_NS.load(Ordering::Relaxed),
        BENCH_FIRST_SERIAL.load(Ordering::Relaxed), phases.join(","))
}
'''
    driver = driver.replace('println!("{{', r'''let diagnostics = bench_diagnostics(); println!("{{\"diagnostics\":{diagnostics},''')
    return source, driver


def measure(command):
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
    row["cpu_ms"] = (usage.ru_utime + usage.ru_stime) * 1000 if usage else None
    row["peak_rss_bytes"] = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024) if usage else None
    return row


def report(data):
    med = statistics.median
    lines = ["# Parallel root-phase diagnostics", "",
             "Temporary per-phase instrumentation, three fresh-process repetitions. Timings are diagnostic; use the uninstrumented comparison for speed claims. CPU/elapsed is the average CPU-core use over the process search, not a claim of exclusive CPU reservation.", "",
             "| Position | Variant | CPU ms | Elapsed ms | CPU / elapsed | Root-phase ms | First-move serial ms | Serial-first fraction | Maximum helpers |",
             "|---|---|---:|---:|---:|---:|---:|---:|---:|"]
    for name, _, _ in data["cases"]:
        for variant in STUDY["VARIANTS"]:
            rows = [r for r in data["runs"] if r["case"] == name and r["variant"] == variant]
            root = [sum(p["total_ns"] for p in r["diagnostics"]["phases"] if p["ply"] == 0) / 1e6 for r in rows]
            first = [r["diagnostics"]["first_serial_ns"] / 1e6 for r in rows]
            cpu = med(r["cpu_ms"] for r in rows) if rows[0]["cpu_ms"] is not None else float("nan")
            used = med(r["cpu_ms"] / r["elapsed_ms"] for r in rows) if rows[0]["cpu_ms"] is not None else float("nan")
            fraction = med(100 * f / t for f, t in zip(first, root))
            lines.append(f"| {name} | {variant} | {cpu:.2f} | {med(r['elapsed_ms'] for r in rows):.2f} | {used:.2f} | {med(root):.2f} | {med(first):.2f} | {fraction:.1f}% | {max(r['diagnostics']['max_helpers'] for r in rows)} |")
    return "\n".join(lines) + "\n"


def main():
    data = {"baseline": STUDY["BASELINE"], "machine": platform.platform(),
            "memory_budget": 100_000_000, "repetitions": 3, "cases": STUDY["CASES"],
            "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(), "runs": []}
    output = ROOT / "benchmarks/parallel/profile-results.json"
    rng = random.Random(0x50484153)
    with tempfile.TemporaryDirectory(prefix="chess-phases-") as tmp:
        binary, data["source_sha256"] = STUDY["compile_benchmark"](pathlib.Path(tmp), transform=transform)
        for name, fen, depth in STUDY["CASES"]:
            print("Profiling", name, flush=True)
            for repetition in range(3):
                variants = STUDY["VARIANTS"].copy()
                rng.shuffle(variants)
                for variant in variants:
                    row = measure([str(binary), variant, fen, "depth", "3", str(depth), "0"])
                    row.update(case=name, variant=variant, repetition=repetition)
                    assert row["diagnostics"]["max_helpers"] < row["threads"]
                    data["runs"].append(row)
                    output.write_text(json.dumps(data, indent=2) + "\n")
        for name, _, _ in STUDY["CASES"]:
            assert len({r["score"] for r in data["runs"] if r["case"] == name}) == 1
    output.with_suffix(".md").write_text(report(data))


if __name__ == "__main__":
    main()
