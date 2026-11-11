# Queen refinement results

The application adopts cubic file exposure with central weight 24.
Knight centralization stays at 6/10. The term corrects the e5/d5
ordering and reduces the blanket penalty's flank-queen regression.
It does not improve every score or position.

Scores and errors are white-relative centipawns. Depth is additional
search plies after each complete listed sequence, except in the root
search table. This is an in-sample calibration, not an Elo claim.

`current` in historical variant names means the archived engine before
this refinement; `application` means the production implementation.

## Focus positions

| Variant | Depth | After e4 e5 | After e4 d5 | After the Qa5 sequence |
|---|---:|---:|---:|---:|
| Stored reference | 50 | 22 | 55 | 54 |
| current | 6 | 38 | -1 | 64 |
| current | 8 | 29 | 15 | 62 |
| early-queen24 | 6 | 24 | 58 | 107 |
| early-queen24 | 8 | 21 | 62 | 108 |
| queen-file-power3-24 | 6 | 24 | 53 | 72 |
| queen-file-power3-24 | 8 | 21 | 48 | 73 |

At depth six, the Qa5 regression relative to the current engine
shrinks from +43 cp under the blanket penalty to +8 cp. At depth
eight, it shrinks from +46 cp to +11 cp. Qa5 is still overvalued
for White compared with both the baseline and the reference.

## All 46 depth-50 references

| Variant | Depth | MAE | Eligible sibling rankings | Unique-board MAE |
|---|---:|---:|---:|---:|
| current | 4 | 15.09 | 6/9 | 15.66 |
| current | 5 | 17.65 | 5/9 | 18.09 |
| current | 6 | 15.83 | 5/9 | 16.18 |
| current | 7 | 14.87 | 6/9 | 15.16 |
| early-queen24 | 4 | 15.00 | 8/9 | 15.55 |
| early-queen24 | 5 | 18.39 | 8/9 | 18.77 |
| early-queen24 | 6 | 15.35 | 9/9 | 15.80 |
| early-queen24 | 7 | 16.20 | 9/9 | 16.52 |
| queen-file-16 | 4 | 14.17 | 8/9 | 14.68 |
| queen-file-16 | 5 | 16.61 | 8/9 | 16.98 |
| queen-file-16 | 6 | 15.13 | 7/9 | 15.36 |
| queen-file-16 | 7 | 15.50 | 8/9 | 15.70 |
| queen-file-24 | 4 | 13.80 | 8/9 | 14.30 |
| queen-file-24 | 5 | 17.09 | 8/9 | 17.34 |
| queen-file-24 | 6 | 14.89 | 9/9 | 15.09 |
| queen-file-24 | 7 | 15.54 | 9/9 | 15.73 |
| queen-file-32 | 4 | 13.83 | 8/9 | 14.32 |
| queen-file-32 | 5 | 17.24 | 8/9 | 17.50 |
| queen-file-32 | 6 | 15.33 | 9/9 | 15.55 |
| queen-file-32 | 7 | 15.61 | 9/9 | 15.80 |
| queen-file-power2-24 | 4 | 13.76 | 8/9 | 14.25 |
| queen-file-power2-24 | 5 | 17.00 | 8/9 | 17.25 |
| queen-file-power2-24 | 6 | 14.70 | 9/9 | 14.89 |
| queen-file-power2-24 | 7 | 15.43 | 9/9 | 15.61 |
| queen-file-power3-24 | 4 | 13.80 | 8/9 | 14.30 |
| queen-file-power3-24 | 5 | 16.93 | 8/9 | 17.18 |
| queen-file-power3-24 | 6 | 14.70 | 9/9 | 14.89 |
| queen-file-power3-24 | 7 | 15.11 | 9/9 | 15.27 |

Mean MAE across these four completed depths changes from 15.86
to 15.14 cp. Improvements occur at depths 4–6; depth seven
regresses from 14.87 to 15.11 cp. All nine eligible rankings agree
with the references at depths six and seven, compared with five and
six respectively before the refinement. Nine comparisons and 44
distinct boards do not establish general playing strength.

## Broader 299-position diagnostic at depth six

| Variant | All-reference MAE | Depth-50 MAE | Eligible sibling rankings |
|---|---:|---:|---:|---:|
| current | 20.55 | 15.83 | 58/92 |
| early-queen16 | 21.44 | 15.76 | 61/92 |
| early-queen24 | 21.64 | 15.35 | 65/92 |
| queen-file-16 | 21.05 | 15.13 | 62/92 |
| queen-file-24 | 21.25 | 14.89 | 65/92 |
| queen-file-power2-24 | 21.15 | 14.70 | 66/92 |
| queen-file-power3-24 | 21.15 | 14.70 | 66/92 |

The selected broader-set error is lower than the blanket penalty's
but still higher than the original engine's. These references have
varying analysis depths and remain a regression diagnostic. They
must not be presented as uniform depth-50 or independent validation.

## Largest depth-50 regressions against the original engine

### Depth 6

| Sequence | Reference | Before | Refined | Increase in absolute error |
|---|---:|---:|---:|---:|
| d4 g6 | 56 | 66 | 84 | +18 |
| e4 g6 d4 | 53 | 66 | 84 | +18 |
| d4 g6 e4 | 53 | 66 | 84 | +18 |
| Nf3 d5 g3 Nf6 Bg2 | 8 | -26 | -39 | +13 |
| Nf3 Nf6 c4 | 25 | 23 | 15 | +8 |
| e4 d5 exd5 Qxd5 Nc3 Qa5 | 54 | 64 | 72 | +8 |
| Nf3 c5 c4 Nc6 | 26 | 24 | 19 | +5 |
| c4 c5 Nf3 Nc6 | 26 | 24 | 19 | +5 |

### Depth 7

| Sequence | Reference | Before | Refined | Increase in absolute error |
|---|---:|---:|---:|---:|
| e4 g6 d4 | 53 | 62 | 91 | +29 |
| d4 g6 e4 | 53 | 62 | 91 | +29 |
| d4 g6 | 56 | 66 | 84 | +18 |
| e4 e5 Nf3 Nc6 Bc4 Bc5 | 19 | 30 | 44 | +14 |
| d4 d5 c4 c6 Nc3 Nf6 | 26 | 29 | 13 | +10 |
| d4 Nf6 Nc3 | 0 | 19 | 26 | +7 |
| Nf3 c5 c4 Nc6 | 26 | 20 | 14 | +6 |
| c4 c5 Nf3 Nc6 | 26 | 20 | 14 | +6 |

## Ordinary production root searches

Here depth includes the move being selected. Complete depth nine
therefore corresponds to eight more plies after the chosen move.

| Root depth | Position | Best move (UCI) | White score |
|---|---|---|---:|
| 7 | start | e2e4 | 23 |
| 7 | e4 | e7e5 | 24 |
| 7 | d4 | d7d5 | 24 |
| 9 | start | e2e4 | 24 |
| 9 | e4 | e7e5 | 21 |
| 9 | d4 | d7d5 | 21 |

## Complete refinement sensitivity screen

| Variant | Depth | e5 | d5 | Qa5 sequence |
|---|---:|---:|---:|---:|
| current | 6 | 38 | -1 | 64 |
| current | 8 | 29 | 15 | 62 |
| early-queen24 | 6 | 24 | 58 | 107 |
| early-queen24 | 8 | 21 | 62 | 108 |
| queen-file-16 | 6 | 24 | 41 | 72 |
| queen-file-16 | 8 | 29 | 47 | 74 |
| queen-file-24 | 6 | 24 | 53 | 76 |
| queen-file-24 | 8 | 21 | 52 | 77 |
| queen-file-32 | 6 | 24 | 61 | 80 |
| queen-file-32 | 8 | 17 | 52 | 79 |
| knight-chase-16 | 6 | 38 | 9 | 71 |
| knight-chase-16 | 8 | 24 | 26 | 71 |
| knight-chase-24 | 6 | 42 | 15 | 76 |
| knight-chase-24 | 8 | 24 | 26 | 77 |
| knight-chase-32 | 6 | 45 | 15 | 78 |
| knight-chase-32 | 8 | 24 | 27 | 77 |
| minor-chase-16 | 6 | 38 | 9 | 66 |
| minor-chase-16 | 8 | 29 | 21 | 70 |
| minor-chase-24 | 6 | 38 | 9 | 58 |
| minor-chase-24 | 8 | 29 | 22 | 70 |
| queen-file-power2-24 | 6 | 24 | 53 | 72 |
| queen-file-power2-24 | 8 | 21 | 48 | 76 |
| queen-file-power2-32 | 6 | 24 | 61 | 72 |
| queen-file-power2-32 | 8 | 17 | 52 | 77 |
| queen-file-power3-24 | 6 | 24 | 53 | 72 |
| queen-file-power3-24 | 8 | 21 | 48 | 73 |
| queen-file-power3-32 | 6 | 24 | 57 | 72 |
| queen-file-power3-32 | 8 | 17 | 49 | 77 |

Direct chase terms were not selected: they did not reliably resolve
the pair at both depths. Cubic24 retains the geometric correction
with a smaller flank cost and slightly better four-depth calibration
than the measured linear and quadratic alternatives. Its remaining
regressions are preserved above; no out-of-sample or Elo claim follows.
