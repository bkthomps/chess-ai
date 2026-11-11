# Opening evaluation calibration

The [constant follow-up](constant-follow-up/README.md) investigates the remaining
Scandinavian preference while retaining knight centralization, including
classical Stockfish context, isolated trials, and broader regressions.
The subsequent [queen refinement](queen-refinement/README.md) records the
narrower development term now used by the application. The earlier tables
below describe the calibration before that refinement; `application` runs
always use the current production engine.

This study calibrates general evaluation constants using the stored engine scores
in the [365Chess Opening Explorer](https://www.365chess.com/opening.php).
The application baseline is `bdb77b1c4af864b24b9dd81a4906110ef970b129`.
Random opening bonuses had already been removed by the user and are not part of
the baseline, study, or resulting application.

## Reference collection and cutoff

Collection on 2026-11-09 starts at the initial board and follows every displayed
continuation with **at least 10,000 games at every ply**, stopping at ply six.
Six plies means three moves by each side. A branch ends when it has no qualifying
continuation; moves below the cutoff do not enter the calibration dataset.

The qualifying first moves are e4, d4, Nf3, c4, g3, f4, and b3. In the readable
snapshots g3 and b3 have no Black reply meeting the cutoff, and f4 stops after
d5. These are intentional exclusions, not illegal chess moves.

There are 208 readable pages and 299 qualifying move sequences. **46 references,
covering 44 distinct boards, report depth 50.** They are the calibration targets.
The other references are retained as lower-depth diagnostics. There is no
qualifying depth-50 reference for g3, f4, or b3 in this collection. Depth 45 or
48 is never relabeled depth 50.

Twenty-two requested pages remained unavailable after a retry. They are listed
in [collection-state.json](collection-state.json), separately from branches
with no qualifying continuation. Their descendants cannot be claimed as
covered. Direct HTTP access returned 403; collection therefore used readable
web results and each page's actual continuation links. Browser connection was
also unavailable. The source temporarily rate-limited requests; collection
paused and continued, without treating rate-limit failures as missing moves.

[reference-pages/](reference-pages/) retains each readable source snapshot,
URL, SAN sequence, continuation game counts, scores, and reported depths.
[references.json](references.json) is the normalized dataset. When a position's
own stored analysis is deeper than the score in its parent's table, the deeper
observation is used and the alternative is retained. Cached pages can come from
different crawl dates and engine analyses, so close score differences should
not be interpreted as an exact total ordering.

## Method

All SAN sequences are replayed with the application's legal move generator.
The driver records the resulting FEN and supplies the complete played history
to ordinary search, preserving halfmove and repetition context. Evaluations are
white-relative centipawns. No random preferences or opening-specific overrides
are present.

The initial screen changes one criterion at a time: knight or bishop
centralization, pawn advancement, queen centralization, space, bishop mobility,
or king safety. Follow-up screens test phase-scaled side-to-move initiative and
an early-queen penalty. The queen penalty was not selected: it changes some
individual lines without a consistent broad benefit. After the tempo change,
second-stage trials change one further criterion relative to that version.

Searches complete depths 4, 5, and 6 **from each reference position**, rather
than from the initial board. A depth-seven follow-up checks the selected
constants on all 46 depth-50 references. These are much shallower searches than
the website's depth 50; the target is useful score calibration and sensible
preferences, not identical search results.

Every position receives a fresh cache and thread buffers, one search thread,
and the unchanged 100,000,000-byte total memory budget. Raw rows record actual
cache allocation, limits, nodes, and hits. Instrumentation exists only in
temporary source copies: [driver.rs](driver.rs) is appended by
[study.py](study.py), never compiled into the application. Builds use
`rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`; LTO is
enabled, matching the release profile. Compiler/platform, source hashes,
reference data, and each scored FEN are preserved in the result JSON files.

This is a deterministic score study, not a timing benchmark. Node counts
describe changed search trees; no speed, extra-depth, or Elo claim is made.

## Selected constants

- **Middlegame knight centralization: 10 → 6 centipawns per center unit.**
  The old endgame weight of 10 is preserved, using the existing material-phase
  blend. The small delta table is compiled into the executable and adds no
  heap allocation or per-node geometry calculation.
- **Side-to-move initiative: 24 centipawns at full material**, fading to zero
  with the same phase. This addresses a substantial odd/even horizon swing in
  the original evaluator. It is an evaluation term used throughout the game,
  not a move-selection bonus.

The other evaluation constants retain their baseline values. The existing
static evaluation cache now explicitly checks the turn as well as the full
hash and piece squares: initiative makes a forced same-board/full-hash
collision with the opposite turn distinguishable. The additional tag fits
existing padding; entries remain 80 bytes and the memory allowance is unchanged.
Search, pruning, ordering, transposition-table bounds, time allocation, and
thread limits retain their previous logic.

On the 46 depth-50 references, mean absolute error changes as follows:

| Search depth | Baseline | Selected candidate |
|---:|---:|---:|
| 4 | 23.57 cp | 15.09 cp |
| 5 | 30.02 cp | 17.65 cp |
| 6 | 24.02 cp | 15.83 cp |
| 7 follow-up | 24.50 cp | 14.87 cp |

Ordering is mixed: agreement on nine sibling comparisons separated by at
least 20 cp improves at depths 4–6, but changes from 7/9 to 6/9 at depth 7.
The Scandinavian remains too attractive at some depths; at six more plies
after `1. e4 d5`, the candidate reports −1 cp for White versus the site's +55.
The calibration does not establish that the opening preference is fixed or
that every criterion is properly tuned. Individual regressions, family
breakdowns, and transposition-adjusted errors appear in
[results.md](results.md). The complete 299-position comparison is in
[comparison.csv](comparison.csv).

A [follow-up diagnosis of the d5/e5 gap](d5-e5-diagnosis.md) reproduces the
scores without the transposition table and without PVS, traces the scored
leaves, and identifies a quiet pawn push outside the quiescence search. The
39 cp gap at the original cutoff shrinks at greater depth; it is not a stable
measure of the opening difference.

The source data selected these constants, so this is an in-sample calibration.
Family breakdowns and lower-depth diagnostics are regression checks, not a
fully independent validation set. Opening positions do not adequately identify
endgame material values, passer weights, pawn races, or general king-safety
parameters. Those remain outside the evidence of this study.

## Reproduction

Only Git, Rust, and Python's standard library are required. The application
does not need these reference files or network access. Use the saved snapshots
to reproduce scores offline:

```sh
python3 benchmarks/opening-evaluation/study.py --variants baseline tempo24-knight6-tapered --scope all --depths 4 5 6 --output /tmp/opening-follow-up.json
python3 benchmarks/opening-evaluation/study.py --variants baseline tempo24-knight6-tapered --scope gold --depths 7 --output /tmp/opening-depth7-follow-up.json
python3 benchmarks/opening-evaluation/study.py --variants application --scope gold --depths 4 5 6 --output /tmp/opening-application-follow-up.json
python3 benchmarks/opening-evaluation/report.py
```

The baseline and shared engine modules are loaded from the pinned revision;
`application` uses the current `src/engine.rs` with the pinned shared modules.
Use new output paths for follow-up studies to retain these measurements.
`report.py` only rebuilds readable tables from the committed snapshots and
measurement files. To refresh source data, follow the recorded source URLs and
continuation links, retain actual game counts and reported depths, and record
unavailable pages explicitly.

[screens.md](screens.md) summarizes each isolated stage. Early stages have
smaller cohorts because collection was still progressing; comparisons between
candidates are meaningful within the same stage, not between stages with
different reference counts. Raw stages are `screen-gold.json`,
`constant-confirmation.json`, `tempo-screen.json`, `queen-screen.json`,
`broad-validation.json`, `second-stage.json`, `tapered-validation.json`, and
`deeper-check.json`.

To reproduce an earlier cohort exactly, pass its result file with
`--cohort benchmarks/opening-evaluation/screen-gold.json` (or the relevant
stage), together with the variants and depths recorded in that file and a new
output path.

## Verification

The application tests cover the initiative's side-to-move behavior, turn-tagged
cache collisions, phase endpoints, promotion clamping, retained endgame knight
weight, independent arithmetic evaluation, serial/parallel agreement, proven
best moves, mate normalization, terminal/draw handling, cancellation, history
restoration, and the existing memory/thread limits. The application verification
study checks that production scores, static scores, moves, and search counters
match the selected temporary candidate on the depth-50 reference set.
