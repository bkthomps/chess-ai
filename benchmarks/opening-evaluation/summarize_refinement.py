"""Rebuild the queen-refinement report from retained measurements."""
import json
import pathlib
import statistics

HERE = pathlib.Path(__file__).resolve().parent
FOLDER = HERE / 'queen-refinement'
SELECTED = 'queen-file-power3-24'


def read(folder, name):
    return json.loads((folder / name).read_text())


def main():
    old = HERE / 'constant-follow-up'
    initial = read(FOLDER, 'focus-screen.json')
    convex = read(FOLDER, 'convex-focus.json')
    gold_files = [read(old, name) for name in
                  ['gold-validation.json', 'queen-gold.json', 'depth7-validation.json']]
    gold_files += [read(FOLDER, name) for name in ['gold-validation.json', 'convex-gold.json']]
    gold_rows = [r for file in gold_files for r in file['runs']]
    summaries = {(r['variant'], r['depth']): r for file in gold_files for r in file['summaries']}
    focus = [r for file in [initial, convex] for r in file['runs']]
    by_focus = {(r['variant'], r['depth'], tuple(r['sequence'])): r for r in focus}
    qa5 = ('e4', 'd5', 'exd5', 'Qxd5', 'Nc3', 'Qa5')
    lines = ['# Queen refinement results', '',
             'The application adopts cubic file exposure with central weight 24.',
             'Knight centralization stays at 6/10. The term corrects the e5/d5',
             'ordering and reduces the blanket penalty\'s flank-queen regression.',
             'It does not improve every score or position.', '',
             'Scores and errors are white-relative centipawns. Depth is additional',
             'search plies after each complete listed sequence, except in the root',
             'search table. This is an in-sample calibration, not an Elo claim.', '',
             '`current` in historical variant names means the archived engine before',
             'this refinement; `application` means the production implementation.', '',
             '## Focus positions', '',
             '| Variant | Depth | After e4 e5 | After e4 d5 | After the Qa5 sequence |',
             '|---|---:|---:|---:|---:|',
             '| Stored reference | 50 | 22 | 55 | 54 |']
    for variant in ['current', 'early-queen24', SELECTED]:
        for depth in [6, 8]:
            values = [by_focus[(variant, depth, seq)]['score_cp']
                      for seq in [('e4', 'e5'), ('e4', 'd5'), qa5]]
            lines.append(f'| {variant} | {depth} | ' + ' | '.join(map(str, values)) + ' |')
    lines += ['', 'At depth six, the Qa5 regression relative to the current engine',
              'shrinks from +43 cp under the blanket penalty to +8 cp. At depth',
              'eight, it shrinks from +46 cp to +11 cp. Qa5 is still overvalued',
              'for White compared with both the baseline and the reference.', '',
              '## All 46 depth-50 references', '',
              '| Variant | Depth | MAE | Eligible sibling rankings | Unique-board MAE |',
              '|---|---:|---:|---:|---:|']
    variants = ['current', 'early-queen24', 'queen-file-16', 'queen-file-24',
                'queen-file-32', 'queen-file-power2-24', SELECTED]
    for variant in variants:
        for depth in [4, 5, 6, 7]:
            summary = summaries[(variant, depth)]['gold']
            boards = {}
            for row in gold_rows:
                if row['variant'] == variant and row['depth'] == depth:
                    boards.setdefault(' '.join(row['fen'].split()[:4]), []).append(
                        abs(row['score_cp'] - row['reference_cp']))
            unique = statistics.mean(map(statistics.mean, boards.values()))
            right = round(summary['ordering_agreement'] * summary['ordered_pairs'])
            lines.append(f'| {variant} | {depth} | {summary["mae_cp"]:.2f} | '
                         f'{right}/{summary["ordered_pairs"]} | {unique:.2f} |')
    baseline_mean = statistics.mean(summaries['current', d]['gold']['mae_cp'] for d in [4, 5, 6, 7])
    selected_mean = statistics.mean(summaries[SELECTED, d]['gold']['mae_cp'] for d in [4, 5, 6, 7])
    lines += ['', f'Mean MAE across these four completed depths changes from {baseline_mean:.2f}',
              f'to {selected_mean:.2f} cp. Improvements occur at depths 4–6; depth seven',
              'regresses from 14.87 to 15.11 cp. All nine eligible rankings agree',
              'with the references at depths six and seven, compared with five and',
              'six respectively before the refinement. Nine comparisons and 44',
              'distinct boards do not establish general playing strength.', '',
              '## Broader 299-position diagnostic at depth six', '',
              '| Variant | All-reference MAE | Depth-50 MAE | Eligible sibling rankings |',
              '|---|---:|---:|---:|---:|']
    broad_files = [read(old, 'broad-validation.json'), read(FOLDER, 'broad-validation.json'),
                   read(FOLDER, 'convex-broad.json')]
    for file in broad_files:
        for s in file['summaries']:
            m = s['all']
            right = round(m['ordering_agreement'] * m['ordered_pairs'])
            lines.append(f'| {s["variant"]} | {m["mae_cp"]:.2f} | '
                         f'{s["gold"]["mae_cp"]:.2f} | {right}/{m["ordered_pairs"]} |')
    lines += ['', 'The selected broader-set error is lower than the blanket penalty\'s',
              'but still higher than the original engine\'s. These references have',
              'varying analysis depths and remain a regression diagnostic. They',
              'must not be presented as uniform depth-50 or independent validation.', '',
              '## Largest depth-50 regressions against the original engine', '']
    for depth in [6, 7]:
        baseline = {tuple(r['sequence']): r for r in gold_rows
                    if r['variant'] == 'current' and r['depth'] == depth}
        changed = [r for r in gold_rows if r['variant'] == SELECTED and r['depth'] == depth]
        regressions = []
        for row in changed:
            before = baseline[tuple(row['sequence'])]
            increase = abs(row['score_cp'] - row['reference_cp']) - abs(before['score_cp'] - row['reference_cp'])
            regressions.append((increase, row, before))
        lines += [f'### Depth {depth}', '',
                  '| Sequence | Reference | Before | Refined | Increase in absolute error |',
                  '|---|---:|---:|---:|---:|']
        for increase, r, b in sorted(regressions, key=lambda item: item[0], reverse=True)[:8]:
            lines.append(f'| {" ".join(r["sequence"])} | {r["reference_cp"]} | '
                         f'{b["score_cp"]} | {r["score_cp"]} | {increase:+d} |')
        lines.append('')
    lines += ['## Ordinary production root searches', '',
              'Here depth includes the move being selected. Complete depth nine',
              'therefore corresponds to eight more plies after the chosen move.', '',
              '| Root depth | Position | Best move (UCI) | White score |',
              '|---|---|---|---:|']
    roots = read(FOLDER, 'root-checks.json')
    for summary in roots['summaries']:
        for name, r in summary['roots'].items():
            lines.append(f'| {summary["depth"]} | {name} | {r["best"]} | {r["score_cp"]} |')
    lines += ['', '## Complete refinement sensitivity screen', '',
              '| Variant | Depth | e5 | d5 | Qa5 sequence |',
              '|---|---:|---:|---:|---:|']
    for file in [initial, convex]:
        for variant in file['changes']:
            for depth in [6, 8]:
                values = [by_focus[(variant, depth, seq)]['score_cp']
                          for seq in [('e4', 'e5'), ('e4', 'd5'), qa5]]
                lines.append(f'| {variant} | {depth} | ' + ' | '.join(map(str, values)) + ' |')
    lines += ['', 'Direct chase terms were not selected: they did not reliably resolve',
              'the pair at both depths. Cubic24 retains the geometric correction',
              'with a smaller flank cost and slightly better four-depth calibration',
              'than the measured linear and quadratic alternatives. Its remaining',
              'regressions are preserved above; no out-of-sample or Elo claim follows.', '']
    (FOLDER / 'results.md').write_text('\n'.join(lines))


if __name__ == '__main__':
    main()
