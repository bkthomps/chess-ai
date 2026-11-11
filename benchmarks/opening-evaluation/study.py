"""Offline opening evaluation study; Python stdlib and rustc, no app dependencies."""
import argparse
import hashlib
import functools
import json
import pathlib
import platform
import re
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BASELINE = 'bdb77b1c4af864b24b9dd81a4906110ef970b129'
FLAGS = ['--edition=2021', '-C', 'opt-level=3', '-C', 'codegen-units=1', '-C', 'lto=fat']
VARIANTS = {
    'baseline': None,
    'application': None,
    'knight6': ('2 => center * 10,', '2 => center * 6,'),
    'knight8': ('2 => center * 10,', '2 => center * 8,'),
    'bishop7': ('3 => center * 5,', '3 => center * 7,'),
    'pawn4': ('1 => y * 8 + center * 2,', '1 => y * 4 + center * 2,'),
    'queen0': ('5 => center * 2,', '5 => 0,'),
    'space2': ('const SPACE_UNIT: i32 = 4;', 'const SPACE_UNIT: i32 = 2;'),
    'bishop-mobility3': ('(5, 5), (2, 4)', '(3, 3), (2, 4)'),
    'safety-half': ('danger.min(300) * attack_material / 12', 'danger.min(300) * attack_material / 24'),
}
for _tempo in [16, 24, 32]:
    VARIANTS[f'tempo{_tempo}'] = (
        '    if p.white {\n        score\n    } else {\n        -score\n    }\n}\n\n#[cfg(test)]',
        f'    let tempo = {_tempo} * phase.min(FULL_PHASE) / FULL_PHASE;\n'
        '    if p.white {\n        score + tempo\n    } else {\n        -score + tempo\n    }\n}\n\n#[cfg(test)]')

for _penalty in [8, 12, 16]:
    VARIANTS[f'early-queen{_penalty}'] = (
        '    let (threat_mg, threat_eg) = threats(p, occupied, kings, pawn_attacks, &pressure);',
        '''    let mut early_queen = 0;
    for color in 0..2 {
        let home = if color == 1 { 0x66 } else { 0x6600_0000_0000_0000 };
        let minors = knights[color] | (diagonal[color] & !straight[color]);
        let undeveloped = (minors & home).count_ones() as i32;
        let advanced = if color == 1 { !0xffff } else { !0xffff_0000_0000_0000 };
        let queen = diagonal[color] & straight[color] & advanced;
        let sign = if color == 1 { -1 } else { 1 };
        early_queen += sign * queen.count_ones() as i32 * undeveloped * ''' + str(_penalty) + ''';
    }
    score += early_queen;
    endgame_delta -= early_queen;
    let (threat_mg, threat_eg) = threats(p, occupied, kings, pawn_attacks, &pressure);''')

# Second-stage trials differ from tempo24 by one further criterion.
for _name in ['knight6', 'knight8', 'bishop-mobility3']:
    VARIANTS[f'tempo24-{_name}'] = [VARIANTS['tempo24'], VARIANTS[_name]]

VARIANTS['tempo24-knight6-tapered'] = [
    VARIANTS['tempo24'],
    ('2 => center * 10,', '2 => center * if endgame { 10 } else { 6 },'),
    ('const fn king_deltas()', 'const fn phase_deltas(kind: i8)'),
    ('let piece = if color == 1 { 6 } else { -6 };', 'let piece = if color == 1 { kind } else { -kind };'),
    ('static KING_ENDGAME_DELTAS: [[i16; 64]; 2] = king_deltas();',
     'static KING_ENDGAME_DELTAS: [[i16; 64]; 2] = phase_deltas(6);\n'
     'static KNIGHT_ENDGAME_DELTAS: [[i16; 64]; 2] = phase_deltas(2);'),
    ('2 => knights[color] |= 1u64 << square,',
     '2 => { knights[color] |= 1u64 << square; endgame_delta += i32::from(KNIGHT_ENDGAME_DELTAS[color][square]); },'),
]


def references():
    rows = {}
    for path in sorted((HERE / 'reference-pages').glob('*.json')):
        page = json.loads(path.read_text())
        for row in page['rows']:
            if row['games'] < 10000 or row['score_cp'] is None:
                continue
            sequence = page['path'] + [row['san']]
            if len(sequence) > 6:
                continue
            key = ' '.join(sequence)
            rows[key] = dict(sequence=sequence, games=row['games'],
                             reference_cp=row['score_cp'], reference_depth=row['depth'],
                             source_url=page['url'], snapshot=path.name,
                             family=sequence[0], parent=' '.join(page['path']))
    # Prefer the position's own stored analysis when it is deeper than the
    # score shown in its parent's continuation table. Preserve the alternative
    # and provenance; independent cached pages need not be mutually consistent.
    for path in sorted((HERE / 'reference-pages').glob('*.json')):
        page = json.loads(path.read_text())
        key = ' '.join(page['path'])
        match = re.search(r'(?:Stored Analysis:|Current eval:)\s*([+−-]\d+\.\d+)\s*Depth:?\s*(\d+)', page['raw'])
        if key not in rows or match is None:
            continue
        score = round(float(match[1].replace('−', '-')) * 100)
        depth = int(match[2])
        row = rows[key]
        alternative = dict(score_cp=score, depth=depth, source_url=page['url'], snapshot=path.name)
        if depth > row['reference_depth']:
            alternative = dict(score_cp=row['reference_cp'], depth=row['reference_depth'],
                               source_url=row['source_url'], snapshot=row['snapshot'])
            row.update(reference_cp=score, reference_depth=depth,
                       source_url=page['url'], snapshot=path.name)
        row['alternative'] = alternative
    return [dict(id=str(i), **row) for i, row in enumerate(rows.values())]


@functools.lru_cache(maxsize=None)
def shared_source(name):
    return subprocess.check_output(['git', 'show', f'{BASELINE}:src/{name}.rs'], cwd=ROOT)


def compile_variant(folder, variant, engine_source):
    dest = folder / variant
    dest.mkdir()
    source = engine_source
    replacements = VARIANTS[variant]
    if replacements and isinstance(replacements, tuple):
        replacements = [replacements]
    for old, new in replacements or []:
        assert old in source, (variant, 'baseline source changed')
        source = source.replace(old, new, 1)
    digest = hashlib.sha256(source.encode()).hexdigest()
    (dest / 'engine.rs').write_text(source + '\n' + (HERE / 'driver.rs').read_text())
    for name in ['chess', 'tables', 'cache', 'ordering']:
        (dest / f'{name}.rs').write_bytes(shared_source(name))
    (dest / 'main.rs').write_text('''#![allow(dead_code)]
mod chess; mod tables; mod engine;
fn main() {
 let a: Vec<_> = std::env::args().collect();
 engine::opening_study(&a[1], a[2].parse().unwrap());
}
''')
    binary = dest / 'study'
    subprocess.run(['rustc', *FLAGS, str(dest / 'main.rs'), '-o', str(binary)], check=True)
    return binary, digest


def metrics(rows, gold_only=True):
    selected = [r for r in rows if not gold_only or r['reference_depth'] == 50]
    errors = [abs(r['score_cp'] - r['reference_cp']) for r in selected]
    groups = {}
    for r in selected:
        groups.setdefault(r['parent'], []).append(r)
    concordant = total = 0
    # Ignore near-ties: these sources have different engines and search depths.
    for group in groups.values():
        for i, a in enumerate(group):
            for b in group[i + 1:]:
                delta = a['reference_cp'] - b['reference_cp']
                if abs(delta) < 20:
                    continue
                total += 1
                concordant += (a['score_cp'] - b['score_cp']) * delta > 0
    return dict(positions=len(selected), mae_cp=sum(errors) / len(errors) if errors else None,
                ordering_agreement=concordant / total if total else None,
                ordered_pairs=total)


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--variants', nargs='+', choices=list(VARIANTS), default=['baseline'])
    parser.add_argument('--depths', nargs='+', type=int, default=[4, 5])
    parser.add_argument('--scope', choices=['gold', 'all'], default='all')
    parser.add_argument('--cohort', type=pathlib.Path,
                        help='Reuse the exact references embedded in an earlier result JSON')
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    source = subprocess.check_output(['git', 'show', f'{BASELINE}:src/engine.rs'], cwd=ROOT, text=True)
    refs = json.loads(args.cohort.read_text())['references'] if args.cohort else references()
    if args.scope == 'gold':
        refs = [r for r in refs if r['reference_depth'] == 50]
    data = dict(baseline_revision=BASELINE, compiler=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                flags=FLAGS, platform=platform.platform(), total_memory_budget=100000000,
                threads=1, scope=args.scope, references=refs,
                shared_sha256={name: hashlib.sha256(shared_source(name)).hexdigest()
                               for name in ['chess', 'tables', 'cache', 'ordering']},
                engine_sha256={}, runs=[], summaries=[])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='chess-opening-study-') as tmp:
        folder = pathlib.Path(tmp)
        positions = folder / 'positions.tsv'
        positions.write_text(''.join(r['id'] + '\t' + ' '.join(r['sequence']) + '\n' for r in refs))
        binaries = {}
        for variant in args.variants:
            variant_source = (ROOT / 'src/engine.rs').read_text() if variant == 'application' else source
            binaries[variant], data['engine_sha256'][variant] = compile_variant(folder, variant, variant_source)
        by_id = {r['id']: r for r in refs}
        # This is a deterministic score study, not a timing benchmark. Every
        # position starts with a fresh cache and complete played history.
        for variant, binary in binaries.items():
            for depth in args.depths:
                result = subprocess.run([str(binary), str(positions), str(depth)], check=True,
                                        text=True, capture_output=True)
                rows = [dict(by_id[r['id']], **{k: v for k, v in r.items() if k != 'id'},
                             variant=variant) for r in map(json.loads, result.stdout.splitlines())]
                assert len(rows) == len(refs)
                data['runs'].extend(rows)
                summary = dict(variant=variant, depth=depth, gold=metrics(rows),
                               all_depths_diagnostic=metrics(rows, False),
                               families={family: metrics([r for r in rows if r['family'] == family])
                                         for family in sorted({r['family'] for r in rows})})
                data['summaries'].append(summary)
                args.output.write_text(json.dumps(data, indent=2) + '\n')
                print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
