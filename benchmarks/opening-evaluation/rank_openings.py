"""Score every legal opening move at equal depth with the current engine."""
import argparse
import hashlib
import json
import pathlib
import platform
import subprocess
import tempfile

import study

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MAIN = '''#![allow(dead_code)]
mod chess; mod tables; mod engine;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args[1] == "--generate" {
        let mut lines = String::new();
        for prefix in ["", "e4", "d4"] {
            let mut p = chess::Position::default();
            if !prefix.is_empty() {
                let legal = p.legal_moves();
                let m = *legal.iter().find(|&&m| p.san(m, &legal) == prefix).unwrap();
                p = p.play(m);
            }
            let legal = p.legal_moves();
            let group = if prefix.is_empty() { "start" } else { prefix };
            for &m in &legal {
                let san = p.san(m, &legal);
                lines.push_str(&format!("{group}:{san}\\t{prefix} {san}\\n"));
            }
        }
        std::fs::write(&args[2], lines).unwrap();
    } else {
        engine::opening_study(&args[1], args[2].parse().unwrap());
    }
}
'''


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--depth', type=int, default=7, help='Includes the candidate move')
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.depth < 2:
        parser.error('--depth must be at least 2')
    source = {name: (ROOT / f'src/{name}.rs').read_bytes()
              for name in ['engine', 'chess', 'tables', 'cache', 'ordering']}
    driver = (HERE / 'driver.rs').read_bytes()
    data = dict(root_depth=args.depth, child_depth=args.depth - 1, threads=1,
                total_memory_budget=100000000, flags=study.FLAGS,
                compiler=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                platform=platform.platform(),
                source_sha256={name: hashlib.sha256(blob).hexdigest() for name, blob in source.items()},
                driver_sha256=hashlib.sha256(driver).hexdigest(), runs=[], rankings={})
    with tempfile.TemporaryDirectory(prefix='chess-opening-rankings-') as tmp:
        folder = pathlib.Path(tmp)
        for name, blob in source.items():
            (folder / f'{name}.rs').write_bytes(blob + (b'\n' + driver if name == 'engine' else b''))
        (folder / 'main.rs').write_text(MAIN)
        binary = folder / 'rank'
        subprocess.run(['rustc', *study.FLAGS, str(folder / 'main.rs'), '-o', str(binary)], check=True)
        positions = folder / 'positions.tsv'
        subprocess.run([str(binary), '--generate', str(positions)], check=True)
        for line in positions.read_text().splitlines():
            single = folder / 'single.tsv'
            single.write_text(line + '\n')
            result = subprocess.run([str(binary), str(single), str(args.depth - 1)],
                                    check=True, capture_output=True, text=True)
            row = json.loads(result.stdout)
            row['group'], row['move'] = row['id'].split(':')
            data['runs'].append(row)
    for group in ['e4', 'd4', 'start']:
        rows = [r for r in data['runs'] if r['group'] == group]
        assert len(rows) == 20
        rows.sort(key=lambda r: r['score_cp'], reverse=group == 'start')
        data['rankings'][group] = [dict(move=r['move'], score_cp=r['score_cp']) for r in rows]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(data, indent=2) + '\n')
    for group, rows in data['rankings'].items():
        print(group, json.dumps(rows[:10]), flush=True)


if __name__ == '__main__':
    main()
