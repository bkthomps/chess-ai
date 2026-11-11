"""Isolated constant trials relative to the current calibrated engine snapshot.

All builds and instrumentation stay in temporary directories. No timing claims.
"""
import argparse
import hashlib
import json
import pathlib
import platform
import subprocess
import tempfile

import study

HERE = pathlib.Path(__file__).resolve().parent
BASE = HERE / 'constant-follow-up'
VARIANTS = {'current': []}
VARIANTS['application'] = []


def constant(name, old, values):
    for value in values:
        VARIANTS[f'{name.lower()}-{value}'] = [
            (f'const {name}: i32 = {old};', f'const {name}: i32 = {value};')]


constant('KNIGHT_CENTER_MIDDLEGAME', 6, [0, 2, 4, 8])
constant('TEMPO_MIDDLEGAME', 24, [16, 32])
constant('SPACE_UNIT', 4, [2, 8])
for value in [0, 2]:
    VARIANTS[f'queen-mobility-{value}'] = [
        ('(2, 4), (1, 2)', f'(2, 4), ({value}, 2)')]
for value in [2, 6]:
    VARIANTS[f'knight-mobility-{value}'] = [
        ('(4, 4), (5, 5)', f'({value}, 4), (5, 5)')]
for value in [3, 7]:
    VARIANTS[f'bishop-mobility-{value}'] = [
        ('(5, 5), (2, 4)', f'({value}, 5), (2, 4)')]
for value in [4, 12]:
    VARIANTS[f'pawn-advance-{value}'] = [
        ('1 => y * 8 + center * 2,',
         f'1 => y * if endgame {{ 8 }} else {{ {value} }} + center * 2,')]
    # Add the corresponding phase delta, keeping the endgame endpoint intact.
    VARIANTS[f'pawn-advance-{value}'] += [
        ('1 => pawns[color] |= 1u64 << square,',
         '1 => { pawns[color] |= 1u64 << square; '
         'let rank = if piece > 0 { square / 8 } else { 7 - square / 8 }; '
         f'endgame_delta += i32::from(piece.signum()) * rank as i32 * (8 - {value}); }}')]
for value in [0, 12]:
    VARIANTS[f'connected-pawn-{value}'] = [
        ('const CONNECTED_PAWN: (i32, i32) = (6, 10);',
         f'const CONNECTED_PAWN: (i32, i32) = ({value}, 10);')]
for value in [0, 16]:
    VARIANTS[f'minor-behind-pawn-{value}'] = [
        ('const MINOR_BEHIND_PAWN: (i32, i32) = (8, 4);',
         f'const MINOR_BEHIND_PAWN: (i32, i32) = ({value}, 4);')]
for name, divisor in [('king-safety-half', 24), ('king-safety-double', 6)]:
    VARIANTS[name] = [('danger.min(300) * attack_material / 12',
                       f'danger.min(300) * attack_material / {divisor}')]
VARIANTS['early-queen12'] = [study.VARIANTS['early-queen12']]
for weight in [4, 8, 16, 24]:
    old, replacement = study.VARIANTS['early-queen12']
    assert '* 12;' in replacement
    VARIANTS[f'early-queen{weight}'] = [(old, replacement.replace('* 12;', f'* {weight};'))]

# Original experimental implementation of the general safe-pawn-push idea.
# This is a positional estimate, not a legal-move or exchange proof. It keeps
# knight placement intact and adds only a middlegame term; no endgame tuning.
PUSH_HELPER = '''
fn experimental_push_threats(
    occupied: [u64; 2], pawns: [u64; 2], kings: [usize; 2],
    pawn_attacks: [u64; 2], pressure: &Pressure,
) -> i32 {
    let empty = !(occupied[0] | occupied[1]);
    let mut counts = [0i32; 2];
    for color in 0..2 {
        let first = if color == 1 { pawns[color] << 8 } else { pawns[color] >> 8 } & empty;
        let second = if color == 1 {
            (first & (0xffu64 << 16)) << 8
        } else {
            (first & (0xffu64 << 40)) >> 8
        } & empty;
        let safe = (first | second) & !pawn_attacks[1 - color]
            & (!pressure.all[1 - color] | pressure.all[color]);
        let mut candidates = [0; 2];
        candidates[color] = safe;
        let targets = mobility::pawn_attacks(candidates)[color]
            & occupied[1 - color] & !pawns[1 - color]
            & !(1u64 << kings[1 - color]);
        counts[color] = targets.count_ones() as i32;
    }
    (counts[1] - counts[0]) * PUSH_WEIGHT
}
'''
for weight in [8, 16, 24, 32]:
    VARIANTS[f'pawn-push-threat-{weight}'] = [
        ('fn threats(', PUSH_HELPER.replace('PUSH_WEIGHT', str(weight)) + '\nfn threats('),
        ('let (threat_mg, threat_eg) = threats(p, occupied, kings, pawn_attacks, &pressure);',
         'let (threat_base, threat_eg) = threats(p, occupied, kings, pawn_attacks, &pressure);\n'
         '    let threat_mg = threat_base + experimental_push_threats(occupied, pawns, kings, pawn_attacks, &pressure);')]


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--variants', nargs='+', choices=list(VARIANTS), default=list(VARIANTS))
    parser.add_argument('--scope', choices=['pair', 'focus', 'gold', 'all', 'roots'], default='pair')
    parser.add_argument('--depths', nargs='+', type=int, default=[6, 8])
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    source = (BASE / 'baseline-engine.rs').read_text()
    refs = json.loads((HERE / 'references.json').read_text())
    if isinstance(refs, dict):
        refs = refs['references']
    if args.scope == 'pair':
        refs = [r for r in refs if r['sequence'] in [['e4', 'd5'], ['e4', 'e5']]]
        assert len(refs) == 2
    elif args.scope == 'focus':
        wanted = [['e4', 'd5'], ['e4', 'e5'], ['e4', 'd5', 'exd5', 'Qxd5', 'Nc3', 'Qa5']]
        refs = [r for r in refs if r['sequence'] in wanted]
        assert len(refs) == len(wanted)
    elif args.scope == 'gold':
        refs = [r for r in refs if r['reference_depth'] == 50]
    elif args.scope == 'roots':
        refs = [dict(id=name, sequence=sequence) for name, sequence in
                [('start', []), ('e4', ['e4']), ('d4', ['d4'])]]
    data = dict(baseline_revision=study.BASELINE, baseline_engine_sha256=hashlib.sha256(source.encode()).hexdigest(),
                compiler=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                flags=study.FLAGS, platform=platform.platform(), threads=1,
                total_memory_budget=100000000, scope=args.scope, references=refs,
                shared_sha256={name: hashlib.sha256(study.shared_source(name)).hexdigest()
                               for name in ['chess', 'tables', 'cache', 'ordering']},
                changes={}, engine_sha256={}, runs=[], summaries=[])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='chess-constant-follow-up-') as tmp:
        folder = pathlib.Path(tmp)
        positions = folder / 'positions.tsv'
        positions.write_text(''.join(r['id'] + '\t' + ' '.join(r['sequence']) + '\n' for r in refs))
        binaries = {}
        for variant in args.variants:
            study.VARIANTS[variant] = VARIANTS[variant]
            variant_source = (study.ROOT / 'src/engine.rs').read_text() if variant == 'application' else source
            binary, digest = study.compile_variant(folder, variant, variant_source)
            binaries[variant] = binary
            data['changes'][variant] = VARIANTS[variant]
            data['engine_sha256'][variant] = digest
        by_id = {r['id']: r for r in refs}
        for variant, binary in binaries.items():
            for depth in args.depths:
                result = subprocess.run([str(binary), str(positions), str(depth)],
                                        check=True, text=True, capture_output=True)
                rows = [dict(by_id[r['id']], **{k: v for k, v in r.items() if k != 'id'},
                             variant=variant) for r in map(json.loads, result.stdout.splitlines())]
                assert len(rows) == len(refs)
                data['runs'].extend(rows)
                pair = {' '.join(r['sequence']): r['score_cp'] for r in rows
                        if r['sequence'] in [['e4', 'd5'], ['e4', 'e5']]}
                if args.scope == 'roots':
                    summary = dict(variant=variant, depth=depth,
                                   roots={r['id']: dict(score_cp=r['score_cp'], best=r['best'])
                                          for r in rows})
                else:
                    summary = dict(variant=variant, depth=depth, gold=study.metrics(rows),
                                   all=study.metrics(rows, False), pair=pair,
                                   d5_minus_e5=pair.get('e4 d5', 0) - pair.get('e4 e5', 0))
                data['summaries'].append(summary)
                args.output.write_text(json.dumps(data, indent=2) + '\n')
                print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
