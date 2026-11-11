# Opening evaluation measurements

Scores are white-relative centipawns, without randomness. Only reported depth-50 references drive calibration. Lower-depth references are separate diagnostics.

299 qualifying move sequences; 46 depth-50 references. 208 readable source pages; 22 requested pages remained unavailable after retry. An unavailable page is a coverage gap, not evidence that it has no qualifying continuations.

| Ply | Qualifying positions | Depth-50 references |
|---:|---:|---:|
| 1 | 7 | 2 |
| 2 | 28 | 8 |
| 3 | 46 | 9 |
| 4 | 64 | 11 |
| 5 | 84 | 9 |
| 6 | 70 | 7 |

## Depth-50 comparison

Candidate: phase-scaled tempo 24; knight center weight 6 in the middlegame, preserving 10 in the endgame. Search depths below start at each recorded position, not at the initial board.

| Search depth | Baseline MAE | Candidate MAE | Baseline ordering | Candidate ordering | Unique boards | Baseline unique-board MAE | Candidate unique-board MAE |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 4 | 23.57 | 15.09 | 44.4% | 66.7% | 44 | 23.91 | 15.66 |
| 5 | 30.02 | 17.65 | 33.3% | 55.6% | 44 | 30.50 | 18.09 |
| 6 | 24.02 | 15.83 | 44.4% | 55.6% | 44 | 24.09 | 16.18 |
| 7 follow-up | 24.50 | 14.87 | 77.8% | 66.7% | 44 | 25.00 | 15.16 |

Ordering considers sibling references separated by at least 20 cp; there are only nine eligible comparisons. These counts are too small to establish general opening strength. Transposed positions are not independent evidence; the unique-board column weights their errors once.

## Families at search depth six

| First move | Positions | Depth-50 references | Baseline gold MAE | Candidate gold MAE | Baseline all-depth diagnostic MAE | Candidate all-depth diagnostic MAE |
|---|---:|---:|---:|---:|---:|---:|
| Nf3 | 45 | 4 | 21.75 | 12.25 | 17.91 | 15.02 |
| b3 | 1 | 0 | — | — | 5.00 | 17.00 |
| c4 | 33 | 2 | 17.50 | 7.50 | 21.12 | 15.79 |
| d4 | 101 | 16 | 29.62 | 14.12 | 26.11 | 22.02 |
| e4 | 116 | 24 | 21.21 | 18.25 | 27.78 | 23.04 |
| f4 | 2 | 0 | — | — | 20.00 | 9.50 |
| g3 | 1 | 0 | — | — | 10.00 | 14.00 |

## Qualifying replies to 1. e4

Six more search plies from the position after each reply. Some reference rows are below depth 50 and are not calibration targets.

| Reply | Reference | Reference depth | Baseline | Candidate |
|---|---:|---:|---:|---:|
| c5 | +25 | 50 | +26 | +43 |
| e5 | +22 | 50 | +23 | +38 |
| e6 | +4 | 45 | +31 | +48 |
| c6 | +39 | 50 | +38 | +49 |
| d6 | +22 | 45 | +32 | +56 |
| d5 | +55 | 50 | -12 | -1 |
| g6 | +58 | 26 | +68 | +80 |
| Nf6 | +62 | 50 | +30 | +86 |
| Nc6 | +45 | 50 | +8 | +29 |

## Largest depth-50 regressions at search depth six

| Sequence | Reference | Baseline | Candidate | Increase in absolute error |
|---|---:|---:|---:|---:|
| e4 c5 c3 Nf6 | +18 | +9 | +53 | +26 |
| e4 Nf6 e5 Nd5 | +65 | +72 | +97 | +25 |
| e4 c5 | +25 | +26 | +43 | +17 |
| e4 e5 | +22 | +23 | +38 | +15 |
| e4 e5 Nc3 Nf6 | +11 | +15 | +29 | +14 |
| e4 c6 d4 d5 | +36 | +50 | +62 | +12 |
| e4 c6 | +39 | +38 | +49 | +9 |
| e4 d5 exd5 Qxd5 Nc3 Qa5 | +54 | +49 | +64 | +5 |
| d4 g6 e4 Bg7 Nc3 d6 | +51 | +120 | +124 | +4 |
| e4 e5 Nf3 Nc6 Bc4 Bc5 | +19 | +5 | +36 | +3 |

## Limits

- The first-move cutoff includes e4, d4, Nf3, c4, g3, f4, and b3. The same cutoff applies at every later ply. g3 and b3 have no qualifying reply in these snapshots; f4 stops after d5. These short branches are intentional exclusions, separate from source failures.
- Depth-50 coverage is concentrated in e4 and d4. There is no qualifying depth-50 reference for g3, f4, or b3. The data cannot establish that every evaluation criterion is correctly tuned.
- The constants were selected using these references. This is calibration, not an independent test set or an Elo measurement. Family breakdowns and broader lower-depth diagnostics check regressions but do not remove selection bias.
- Cached website pages were captured at different times and can disagree. Each observation retains its source, reported depth, alternative stored score, and raw readable snapshot. Near-tie rankings are deliberately not treated as targets.
- Opening positions do not identify endgame material values, passed-pawn races, or general king-safety weights. Those constants are retained. The knight change tapers back to the original endgame weight.
- Node counts are diagnostics. This is a score study with optimized builds, not a runtime benchmark; no speed or strength claim follows.

See [comparison.csv](comparison.csv) for every position, [references.json](references.json) for source provenance, and [README.md](README.md) for reproduction.
