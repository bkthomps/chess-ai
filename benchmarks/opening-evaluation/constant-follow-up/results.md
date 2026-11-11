# Constant follow-up results

Knight centralization remains useful. The experiments identify a promising
contextual correction, not grounds to remove that term or a universally
better set of constants. This historical stage left the application unchanged;
the later [queen refinement](../queen-refinement/README.md) records the adopted feature.

All scores below are white-relative centipawns. Positive `d5 minus e5`
means Black prefers e5, matching the stored reference ordering.

## Selected pair comparisons

| Variant | Depth after reply | e5 | d5 | d5 minus e5 |
|---|---:|---:|---:|---:|
| Stored depth-50 reference | 50 | 22 | 55 | 33 |
| current | 6 | 38 | -1 | -39 |
| current | 7 | 29 | 10 | -19 |
| current | 8 | 29 | 15 | -14 |
| early-queen12 | 6 | 24 | 29 | 5 |
| early-queen12 | 8 | 29 | 39 | 10 |
| early-queen16 | 6 | 24 | 41 | 17 |
| early-queen16 | 7 | 29 | 63 | 34 |
| early-queen16 | 8 | 29 | 47 | 18 |
| early-queen24 | 6 | 24 | 58 | 34 |
| early-queen24 | 7 | 24 | 69 | 45 |
| early-queen24 | 8 | 21 | 62 | 41 |
| pawn-push-threat-16 | 6 | 31 | 4 | -27 |
| pawn-push-threat-16 | 8 | 39 | 40 | 1 |
| pawn-push-threat-24 | 6 | 31 | 11 | -20 |
| pawn-push-threat-24 | 8 | 46 | 41 | -5 |

The custom early-queen penalty preserves the knight weights at 6/10.
At 24 cp per undeveloped home minor, it reverses the erroneous ordering
at every tested depth from four through eight. It overestimates the
reference gap at some depths. This is a candidate feature with a tunable
weight, not just a change to an existing application constant.

## All 46 depth-50 references

| Variant | Depth | MAE | Sibling ordering correct | Unique-board MAE |
|---|---:|---:|---:|---:|
| current | 4 | 15.09 | 6/9 | 15.66 |
| current | 5 | 17.65 | 5/9 | 18.09 |
| current | 6 | 15.83 | 5/9 | 16.18 |
| current | 7 | 14.87 | 6/9 | 15.16 |
| early-queen12 | 4 | 14.33 | 8/9 | 14.86 |
| early-queen12 | 5 | 17.48 | 8/9 | 18.02 |
| early-queen12 | 6 | 16.46 | 7/9 | 16.98 |
| early-queen16 | 4 | 14.65 | 8/9 | 15.18 |
| early-queen16 | 5 | 17.46 | 8/9 | 17.93 |
| early-queen16 | 6 | 15.76 | 7/9 | 16.25 |
| early-queen16 | 7 | 16.52 | 8/9 | 16.95 |
| early-queen24 | 4 | 15.00 | 8/9 | 15.55 |
| early-queen24 | 5 | 18.39 | 8/9 | 18.77 |
| early-queen24 | 6 | 15.35 | 9/9 | 15.80 |
| early-queen24 | 7 | 16.20 | 9/9 | 16.52 |
| pawn-push-threat-16 | 4 | 16.07 | 6/9 | 16.66 |
| pawn-push-threat-16 | 5 | 18.67 | 5/9 | 19.07 |
| pawn-push-threat-16 | 6 | 17.04 | 5/9 | 17.59 |
| pawn-push-threat-24 | 4 | 16.85 | 7/9 | 17.36 |
| pawn-push-threat-24 | 5 | 19.15 | 5/9 | 19.64 |
| pawn-push-threat-24 | 6 | 18.15 | 5/9 | 18.70 |

The eligible sibling comparisons are separated by at least 20 reference
cp. There are only nine such comparisons; transposed boards and these
few rankings are not independent proof of playing strength.

The 24 cp queen candidate improves MAE at depth six but regresses at
depth seven. The pawn-push trials worsen overall depth-50 MAE at
depths four through six, despite helping this particular pair at depth
eight. Neither result establishes a general improvement.

## Broader 299-position diagnostic at depth six

| Variant | All-reference MAE | Depth-50 MAE | All eligible ordering |
|---|---:|---:|---:|
| current | 20.55 | 15.83 | 58/92 |
| early-queen16 | 21.44 | 15.76 | 61/92 |
| early-queen24 | 21.64 | 15.35 | 65/92 |

This broader cohort includes varying lower-depth references. It is a
regression diagnostic, not an independent validation set. Both queen
candidates worsen its average error while improving its ordering.

## Largest depth-50 regressions, queen24 at depth six

| Sequence | Reference | Current | Queen24 | Increase in absolute error |
|---|---:|---:|---:|---:|
| e4 d5 exd5 Qxd5 Nc3 Qa5 | 54 | 64 | 107 | +43 |
| d4 g6 | 56 | 66 | 92 | +26 |
| e4 g6 d4 | 53 | 66 | 88 | +22 |
| d4 g6 e4 | 53 | 66 | 88 | +22 |
| Nf3 d5 g3 Nf6 Bg2 | 8 | -26 | -39 | +13 |
| Nf3 Nf6 c4 | 25 | 23 | 15 | +8 |
| Nf3 c5 c4 Nc6 | 26 | 24 | 19 | +5 |
| c4 c5 Nf3 Nc6 | 26 | 24 | 19 | +5 |
| e4 c5 Nf3 | 34 | 30 | 42 | +4 |
| e4 c6 | 39 | 49 | 52 | +3 |

## Ordinary root search checks

These are complete depth-seven searches from the named decision,
rather than separately scored candidate moves. Ties can select a
different equally valued move.

| Variant | Position | Best move (UCI) | White score |
|---|---|---|---:|
| current | start | d2d4 | 13 |
| current | e4 | d7d5 | -1 |
| current | d4 | d7d5 | 24 |
| early-queen16 | start | e2e4 | 23 |
| early-queen16 | e4 | e7e5 | 24 |
| early-queen16 | d4 | d7d5 | 24 |
| early-queen24 | start | e2e4 | 23 |
| early-queen24 | e4 | e7e5 | 24 |
| early-queen24 | d4 | d7d5 | 24 |

## Complete initial sensitivity screen

Each row changes only one criterion relative to the current engine.
A zero knight-center weight is a diagnostic ablation, not a proposed
application setting. This screen includes every measured variant,
including regressions.

| Variant | e5 at d6 | d5 at d6 | e5 at d8 | d5 at d8 |
|---|---:|---:|---:|---:|
| current | 38 | -1 | 29 | 15 |
| knight_center_middlegame-0 | 24 | 22 | 16 | 22 |
| knight_center_middlegame-2 | 27 | 13 | 25 | 22 |
| knight_center_middlegame-4 | 25 | 8 | 37 | 17 |
| knight_center_middlegame-8 | 43 | 8 | 24 | 15 |
| tempo_middlegame-16 | 30 | 5 | 21 | 7 |
| tempo_middlegame-32 | 43 | 4 | 37 | 23 |
| space_unit-2 | 38 | -1 | 24 | 7 |
| space_unit-8 | 24 | 8 | 47 | 31 |
| queen-mobility-0 | 38 | 3 | 26 | 22 |
| queen-mobility-2 | 35 | 5 | 32 | 8 |
| knight-mobility-2 | 40 | -4 | 37 | 27 |
| knight-mobility-6 | 36 | 2 | 24 | 9 |
| bishop-mobility-3 | 38 | -1 | 19 | 15 |
| bishop-mobility-7 | 37 | 2 | 37 | 21 |
| pawn-advance-4 | 38 | 0 | 24 | 8 |
| pawn-advance-12 | 25 | 0 | 41 | 23 |
| connected-pawn-0 | 34 | 2 | 35 | 21 |
| connected-pawn-12 | 41 | -1 | 25 | 9 |
| minor-behind-pawn-0 | 38 | 4 | 23 | 15 |
| minor-behind-pawn-16 | 32 | 2 | 37 | 15 |
| king-safety-half | 35 | 5 | 34 | 25 |
| king-safety-double | 37 | -1 | 24 | 9 |
| early-queen12 | 24 | 29 | 29 | 39 |

The existing constant changes alone did not correct this pair at both
depths. The development-sensitive queen term is the strongest lead
from these trials, with explicit regressions still requiring further
evaluation before adoption. Search depth, pruning, ordering, memory,
and thread policy were not altered.
