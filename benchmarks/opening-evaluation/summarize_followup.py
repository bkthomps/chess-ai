"""Rebuild the constant follow-up report from saved offline measurements."""
import json
import pathlib
import statistics

HERE = pathlib.Path(__file__).resolve().parent
FOLDER = HERE / 'constant-follow-up'


def read(name):
    return json.loads((FOLDER / name).read_text())


def main():
    pair = read('pair-screen.json')
    push = read('pawn-push-pair.json')
    queen = read('queen-pair8.json')
    gold = read('gold-validation.json')
    more_gold = read('queen-gold.json')
    broad = read('broad-validation.json')
    deeper = read('depth7-validation.json')
    roots = read('root-checks.json')
    summaries = {(r['variant'], r['depth']): r for file in [gold, more_gold, deeper]
                 for r in file['summaries']}
    pairs = {(r['variant'], r['depth']): r['pair'] for file in [pair, push, queen, gold, more_gold, deeper]
             for r in file['summaries']}
    lines = ['# Constant follow-up results', '',
             'Knight centralization remains useful. The experiments identify a promising',
             'contextual correction, not grounds to remove that term or a universally',
             'better set of constants. This historical stage left the application unchanged;',
             'the later [queen refinement](../queen-refinement/README.md) records the adopted feature.', '',
             'All scores below are white-relative centipawns. Positive `d5 minus e5`',
             'means Black prefers e5, matching the stored reference ordering.', '',
             '## Selected pair comparisons', '',
             '| Variant | Depth after reply | e5 | d5 | d5 minus e5 |',
             '|---|---:|---:|---:|---:|',
             '| Stored depth-50 reference | 50 | 22 | 55 | 33 |']
    for variant in ['current', 'early-queen12', 'early-queen16', 'early-queen24',
                    'pawn-push-threat-16', 'pawn-push-threat-24']:
        for depth in [6, 7, 8]:
            p = pairs.get((variant, depth))
            if p is None:
                continue
            lines.append(f'| {variant} | {depth} | {p["e4 e5"]} | {p["e4 d5"]} | '
                         f'{p["e4 d5"] - p["e4 e5"]} |')
    lines += ['', 'The custom early-queen penalty preserves the knight weights at 6/10.',
              'At 24 cp per undeveloped home minor, it reverses the erroneous ordering',
              'at every tested depth from four through eight. It overestimates the',
              'reference gap at some depths. This is a candidate feature with a tunable',
              'weight, not just a change to an existing application constant.', '',
              '## All 46 depth-50 references', '',
              '| Variant | Depth | MAE | Sibling ordering correct | Unique-board MAE |',
              '|---|---:|---:|---:|---:|']
    all_rows = gold['runs'] + more_gold['runs'] + deeper['runs']
    for variant in ['current', 'early-queen12', 'early-queen16', 'early-queen24',
                    'pawn-push-threat-16', 'pawn-push-threat-24']:
        for depth in [4, 5, 6, 7]:
            s = summaries.get((variant, depth))
            if s is None:
                continue
            boards = {}
            for row in all_rows:
                if row['variant'] == variant and row['depth'] == depth:
                    boards.setdefault(' '.join(row['fen'].split()[:4]), []).append(
                        abs(row['score_cp'] - row['reference_cp']))
            unique_mae = statistics.mean(map(statistics.mean, boards.values()))
            m = s['gold']
            right = round(m['ordering_agreement'] * m['ordered_pairs'])
            lines.append(f'| {variant} | {depth} | {m["mae_cp"]:.2f} | '
                         f'{right}/{m["ordered_pairs"]} | {unique_mae:.2f} |')
    lines += ['', 'The eligible sibling comparisons are separated by at least 20 reference',
              'cp. There are only nine such comparisons; transposed boards and these',
              'few rankings are not independent proof of playing strength.', '',
              'The 24 cp queen candidate improves MAE at depth six but regresses at',
              'depth seven. The pawn-push trials worsen overall depth-50 MAE at',
              'depths four through six, despite helping this particular pair at depth',
              'eight. Neither result establishes a general improvement.', '',
              '## Broader 299-position diagnostic at depth six', '',
              '| Variant | All-reference MAE | Depth-50 MAE | All eligible ordering |',
              '|---|---:|---:|---:|']
    for s in broad['summaries']:
        m = s['all']
        right = round(m['ordering_agreement'] * m['ordered_pairs'])
        lines.append(f'| {s["variant"]} | {m["mae_cp"]:.2f} | '
                     f'{s["gold"]["mae_cp"]:.2f} | {right}/{m["ordered_pairs"]} |')
    lines += ['', 'This broader cohort includes varying lower-depth references. It is a',
              'regression diagnostic, not an independent validation set. Both queen',
              'candidates worsen its average error while improving its ordering.', '',
              '## Largest depth-50 regressions, queen24 at depth six', '',
              '| Sequence | Reference | Current | Queen24 | Increase in absolute error |',
              '|---|---:|---:|---:|---:|']
    before = {tuple(r['sequence']): r for r in broad['runs'] if r['variant'] == 'current'}
    changes = []
    for r in broad['runs']:
        if r['variant'] != 'early-queen24' or r['reference_depth'] != 50:
            continue
        b = before[tuple(r['sequence'])]
        delta = abs(r['score_cp'] - r['reference_cp']) - abs(b['score_cp'] - r['reference_cp'])
        changes.append((delta, r, b))
    for delta, r, b in sorted(changes, key=lambda item: item[0], reverse=True)[:10]:
        lines.append(f'| {" ".join(r["sequence"])} | {r["reference_cp"]} | '
                     f'{b["score_cp"]} | {r["score_cp"]} | {delta:+d} |')
    lines += ['', '## Ordinary root search checks', '',
              'These are complete depth-seven searches from the named decision,',
              'rather than separately scored candidate moves. Ties can select a',
              'different equally valued move.', '',
              '| Variant | Position | Best move (UCI) | White score |',
              '|---|---|---|---:|']
    for s in roots['summaries']:
        for name, r in s['roots'].items():
            lines.append(f'| {s["variant"]} | {name} | {r["best"]} | {r["score_cp"]} |')
    lines += ['', '## Complete initial sensitivity screen', '',
              'Each row changes only one criterion relative to the current engine.',
              'A zero knight-center weight is a diagnostic ablation, not a proposed',
              'application setting. This screen includes every measured variant,',
              'including regressions.', '',
              '| Variant | e5 at d6 | d5 at d6 | e5 at d8 | d5 at d8 |',
              '|---|---:|---:|---:|---:|']
    for variant in pair['changes']:
        p6 = pairs[(variant, 6)]
        p8 = pairs[(variant, 8)]
        lines.append(f'| {variant} | {p6["e4 e5"]} | {p6["e4 d5"]} | '
                     f'{p8["e4 e5"]} | {p8["e4 d5"]} |')
    lines += ['', 'The existing constant changes alone did not correct this pair at both',
              'depths. The development-sensitive queen term is the strongest lead',
              'from these trials, with explicit regressions still requiring further',
              'evaluation before adoption. Search depth, pruning, ordering, memory,',
              'and thread policy were not altered.', '']
    (FOLDER / 'results.md').write_text('\n'.join(lines))


if __name__ == '__main__':
    main()
