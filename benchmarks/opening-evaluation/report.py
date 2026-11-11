"""Write the readable comparison and complete position table from saved runs."""
import csv
import json
import pathlib
import statistics
import study

HERE = pathlib.Path(__file__).resolve().parent


def unique_metrics(rows):
    boards = {}
    for row in rows:
        if row['reference_depth'] == 50:
            boards.setdefault(' '.join(row['fen'].split()[:4]), []).append(
                abs(row['score_cp'] - row['reference_cp']))
    return len(boards), statistics.mean(map(statistics.mean, boards.values())) if boards else None


def main():
    baseline = json.loads((HERE / 'broad-validation.json').read_text())
    candidate = json.loads((HERE / 'tapered-validation.json').read_text())
    state = json.loads((HERE / 'collection-state.json').read_text())
    refs = study.references()
    gold = [r for r in refs if r['reference_depth'] == 50]
    exported = dict(source='365Chess stored opening analysis', captured_date='2026-11-09',
                    minimum_games_each_ply=10000, max_plies=6,
                    readable_pages=len(list((HERE / 'reference-pages').glob('*.json'))),
                    unavailable_pages=state['unavailable'], references=refs)
    (HERE / 'references.json').write_text(json.dumps(exported, indent=2) + '\n')
    before = {(tuple(r['sequence']), r['depth']): r for r in baseline['runs'] if r['variant'] == 'baseline'}
    after = {(tuple(r['sequence']), r['depth']): r for r in candidate['runs']}
    with (HERE / 'comparison.csv').open('w', newline='') as stream:
        fields = ['sequence', 'games', 'reference_cp', 'reference_depth', 'source_url', 'fen']
        fields += [f'{v}_depth{d}_cp' for d in [4, 5, 6] for v in ['baseline', 'candidate']]
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for ref in refs:
            key = tuple(ref['sequence'])
            row = {f: ref[f] for f in fields[:5] if f != 'sequence'}
            row.update(sequence=' '.join(key), fen=before[(key, 4)]['fen'])
            for depth in [4, 5, 6]:
                row[f'baseline_depth{depth}_cp'] = before[(key, depth)]['score_cp']
                row[f'candidate_depth{depth}_cp'] = after[(key, depth)]['score_cp']
            writer.writerow(row)
    lines = ['# Opening evaluation measurements', '',
             'Scores are white-relative centipawns, without randomness. Only reported depth-50 references drive calibration. Lower-depth references are separate diagnostics.', '',
             f'{len(refs)} qualifying move sequences; {len(gold)} depth-50 references. {exported["readable_pages"]} readable source pages; {len(state["unavailable"])} requested pages remained unavailable after retry. An unavailable page is a coverage gap, not evidence that it has no qualifying continuations.', '',
             '| Ply | Qualifying positions | Depth-50 references |', '|---:|---:|---:|']
    for ply in range(1, 7):
        lines.append(f'| {ply} | {sum(len(r["sequence"]) == ply for r in refs)} | {sum(len(r["sequence"]) == ply for r in gold)} |')
    lines += ['', '## Depth-50 comparison', '',
              'Candidate: phase-scaled tempo 24; knight center weight 6 in the middlegame, preserving 10 in the endgame. Search depths below start at each recorded position, not at the initial board.', '',
              '| Search depth | Baseline MAE | Candidate MAE | Baseline ordering | Candidate ordering | Unique boards | Baseline unique-board MAE | Candidate unique-board MAE |',
              '|---:|---:|---:|---:|---:|---:|---:|---:|']
    for depth in [4, 5, 6]:
        b = [r for (seq, d), r in before.items() if d == depth]
        a = [r for (seq, d), r in after.items() if d == depth]
        bm, am = study.metrics(b), study.metrics(a)
        bn, be = unique_metrics(b)
        an, ae = unique_metrics(a)
        assert bn == an
        lines.append(f'| {depth} | {bm["mae_cp"]:.2f} | {am["mae_cp"]:.2f} | {bm["ordering_agreement"]:.1%} | {am["ordering_agreement"]:.1%} | {bn} | {be:.2f} | {ae:.2f} |')
    deeper = json.loads((HERE / 'deeper-check.json').read_text())
    b = [r for r in deeper['runs'] if r['variant'] == 'baseline']
    a = [r for r in deeper['runs'] if r['variant'] != 'baseline']
    bm, am = study.metrics(b), study.metrics(a)
    bn, be = unique_metrics(b)
    an, ae = unique_metrics(a)
    assert bn == an
    lines.append(f'| 7 follow-up | {bm["mae_cp"]:.2f} | {am["mae_cp"]:.2f} | {bm["ordering_agreement"]:.1%} | {am["ordering_agreement"]:.1%} | {bn} | {be:.2f} | {ae:.2f} |')
    lines += ['', 'Ordering considers sibling references separated by at least 20 cp; there are only nine eligible comparisons. These counts are too small to establish general opening strength. Transposed positions are not independent evidence; the unique-board column weights their errors once.', '',
              '## Families at search depth six', '',
              '| First move | Positions | Depth-50 references | Baseline gold MAE | Candidate gold MAE | Baseline all-depth diagnostic MAE | Candidate all-depth diagnostic MAE |',
              '|---|---:|---:|---:|---:|---:|---:|']
    fmt = lambda x: '—' if x is None else f'{x:.2f}'
    for family in sorted({r['family'] for r in refs}):
        b = [r for (seq, d), r in before.items() if d == 6 and r['family'] == family]
        a = [r for (seq, d), r in after.items() if d == 6 and r['family'] == family]
        bg, ag = study.metrics(b), study.metrics(a)
        lines.append(f'| {family} | {len(b)} | {bg["positions"]} | {fmt(bg["mae_cp"])} | {fmt(ag["mae_cp"])} | {fmt(study.metrics(b, False)["mae_cp"])} | {fmt(study.metrics(a, False)["mae_cp"])} |')
    lines += ['', '## Qualifying replies to 1. e4', '',
              'Six more search plies from the position after each reply. Some reference rows are below depth 50 and are not calibration targets.', '',
              '| Reply | Reference | Reference depth | Baseline | Candidate |', '|---|---:|---:|---:|---:|']
    for ref in refs:
        if ref['parent'] == 'e4':
            key = (tuple(ref['sequence']), 6)
            lines.append(f'| {ref["sequence"][-1]} | {ref["reference_cp"]:+} | {ref["reference_depth"]} | {before[key]["score_cp"]:+} | {after[key]["score_cp"]:+} |')
    lines += ['', '## Largest depth-50 regressions at search depth six', '',
              '| Sequence | Reference | Baseline | Candidate | Increase in absolute error |', '|---|---:|---:|---:|---:|']
    differences = []
    for ref in gold:
        key = (tuple(ref['sequence']), 6)
        b, a, target = before[key]['score_cp'], after[key]['score_cp'], ref['reference_cp']
        differences.append((abs(a - target) - abs(b - target), ' '.join(ref['sequence']), target, b, a))
    for delta, seq, target, b, a in sorted(differences, reverse=True)[:10]:
        lines.append(f'| {seq} | {target:+} | {b:+} | {a:+} | {delta:+} |')
    lines += ['', '## Limits', '',
              '- The first-move cutoff includes e4, d4, Nf3, c4, g3, f4, and b3. The same cutoff applies at every later ply. g3 and b3 have no qualifying reply in these snapshots; f4 stops after d5. These short branches are intentional exclusions, separate from source failures.',
              '- Depth-50 coverage is concentrated in e4 and d4. There is no qualifying depth-50 reference for g3, f4, or b3. The data cannot establish that every evaluation criterion is correctly tuned.',
              '- The constants were selected using these references. This is calibration, not an independent test set or an Elo measurement. Family breakdowns and broader lower-depth diagnostics check regressions but do not remove selection bias.',
              '- Cached website pages were captured at different times and can disagree. Each observation retains its source, reported depth, alternative stored score, and raw readable snapshot. Near-tie rankings are deliberately not treated as targets.',
              '- Opening positions do not identify endgame material values, passed-pawn races, or general king-safety weights. Those constants are retained. The knight change tapers back to the original endgame weight.',
              '- Node counts are diagnostics. This is a score study with optimized builds, not a runtime benchmark; no speed or strength claim follows.', '',
              'See [comparison.csv](comparison.csv) for every position, [references.json](references.json) for source provenance, and [README.md](README.md) for reproduction.']
    (HERE / 'results.md').write_text('\n'.join(lines) + '\n')
    screens = ['# Isolated parameter screens', '', 'Different stages have different reference counts as collection progressed. Compare candidates only within the same stage.', '']
    for path in sorted(HERE.glob('*.json')):
        data = json.loads(path.read_text())
        if 'summaries' not in data:
            continue
        screens += [f'## {path.name}', '', '| Variant | Depth | Gold positions | Gold MAE | Ordering agreement |', '|---|---:|---:|---:|---:|']
        for row in data['summaries']:
            m = row['gold']
            agreement = '—' if m['ordering_agreement'] is None else f'{m["ordering_agreement"]:.1%}'
            screens.append(f'| {row["variant"]} | {row["depth"]} | {m["positions"]} | {m["mae_cp"]:.2f} | {agreement} |')
        screens.append('')
    (HERE / 'screens.md').write_text('\n'.join(screens) + '\n')


if __name__ == '__main__':
    main()
