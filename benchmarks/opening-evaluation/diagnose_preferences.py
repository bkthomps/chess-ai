"""Diagnose d5/e5 preference without changing application search or weights."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FLAGS = ['--edition=2021', '-C', 'opt-level=3', '-C', 'codegen-units=1', '-C', 'lto=fat']
TERMS = ['material', 'piece_square', 'pawn_structure', 'piece_pawn', 'space',
         'mobility', 'threats', 'king_safety', 'phase_blend', 'bishop_pair', 'tempo', 'queen_development']


def detail_function(source):
    detail = source[source.index('fn evaluate(p: &Position) -> i32 {'):source.index('\n#[cfg(test)]\nmod tests')]
    detail = detail.replace('fn evaluate(p: &Position) -> i32 {',
                            'fn preference_eval_detail(p: &Position) -> (i32, [i32; 12]) {\n let mut terms = [0; 12];')
    detail = detail.replace('    let pawn_attacks = mobility::pawn_attacks(pawns);',
                            '    terms[0] = p.squares.iter().map(|&pc| i32::from(pc.signum()) * value(pc)).sum();\n'
                            '    terms[1] = score - terms[0];\n'
                            '    let pawn_attacks = mobility::pawn_attacks(pawns);')
    for needle, change in [
        ('score += pawn_mg;', 'terms[2] = pawn_mg;'),
        ('score += piece_mg;', 'terms[3] = piece_mg;'),
        ('score += space;', 'terms[4] = space;'),
        ('score += mg;', 'terms[5] += mg;'),
        ('score += threat_mg + safety;', 'terms[6] = threat_mg; terms[7] = safety;'),
        ('score += endgame_delta * (FULL_PHASE - phase.min(FULL_PHASE)) / FULL_PHASE;',
         'terms[8] = endgame_delta * (FULL_PHASE - phase.min(FULL_PHASE)) / FULL_PHASE;'),
        ('score += 30;', 'terms[9] += 30;'),
        ('score -= 30;', 'terms[9] -= 30;'),
    ]:
        assert needle in detail, needle
        detail = detail.replace(needle, needle + '\n' + change, 1)
    old = '''    if p.white {
        score + tempo
    } else {
        -score + tempo
    }'''
    if 'score += development;' in detail:
        detail = detail.replace('score += development;',
                                'score += development;\n    terms[11] = development;', 1)
    assert old in detail
    return detail.replace(old, '''    terms[10] = if p.white { tempo } else { -tempo };
    assert_eq!(terms.iter().sum::<i32>(), score + terms[10]);
    (score + terms[10], terms)''')


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--modes', nargs='+', choices=['cached', 'no-table', 'full-window'],
                        default=['cached', 'no-table', 'full-window'])
    parser.add_argument('--depths', nargs='+', type=int, default=[6])
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    source = (ROOT / 'src/engine.rs').read_text()
    data = dict(flags=FLAGS, compiler=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                engine_sha256=hashlib.sha256(source.encode()).hexdigest(),
                term_names=TERMS, runs=[])
    with tempfile.TemporaryDirectory(prefix='chess-preference-diagnostic-') as tmp:
        folder = pathlib.Path(tmp)
        for name in ['chess', 'tables', 'cache', 'ordering']:
            (folder / f'{name}.rs').write_bytes((ROOT / f'src/{name}.rs').read_bytes())
        binaries = {}
        for mode in args.modes:
            modified = source
            if mode == 'full-window':
                a = modified.index('            let score = if index == 0 || beta - alpha == 1 {')
                b = modified.index('            self.pop_position(previous);', a)
                modified = modified[:a] + '            let score = -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha).0;\n' + modified[b:]
            (folder / 'engine.rs').write_text(modified + '\n' + detail_function(source) + '\n' +
                                            (HERE / 'diagnostic-driver.rs').read_text())
            (folder / 'main.rs').write_text('''#![allow(dead_code, unused_variables)]
mod chess; mod tables; mod engine;
fn main() { let a: Vec<_> = std::env::args().collect();
engine::preference_diagnostic(&a[1], a[2].parse().unwrap(), a[3] == "no-table"); }
''')
            binary = folder / mode
            subprocess.run(['rustc', *FLAGS, str(folder / 'main.rs'), '-o', str(binary)], check=True)
            binaries[mode] = binary
        for mode, binary in binaries.items():
            for depth in args.depths:
                for reply in ['d5', 'e5']:
                    result = subprocess.run([str(binary), reply, str(depth), mode], check=True,
                                            capture_output=True, text=True)
                    row = dict(json.loads(result.stdout), mode=mode)
                    row['terms'] = dict(zip(TERMS, row.pop('leaf_terms')))
                    data['runs'].append(row)
                    args.output.parent.mkdir(parents=True, exist_ok=True)
                    args.output.write_text(json.dumps(data, indent=2) + '\n')
                    print(json.dumps({k: row[k] for k in ['mode', 'reply', 'depth_after_reply', 'score_cp', 'pv', 'pv_white_scores', 'leaf_white_score', 'terms']}), flush=True)


if __name__ == '__main__':
    main()
