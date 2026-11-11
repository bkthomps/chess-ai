# Why the updated engine prefers d5 to e5

This is a diagnostic follow-up to the opening calibration. It makes no further
application or evaluation-constant changes. The exact engine source hash is
recorded in each result file.

## Search controls

At six additional plies after each Black reply, the production search returns
−1 cp for White after `1. e4 d5`, and +38 after `1. e4 e5`.
The same scores and principal lines were reproduced with the transposition
table disabled, and with ordinary full-window alpha-beta replacing PVS.
These controls support an evaluation/horizon explanation for these two
positions, rather than a cache or PVS discrepancy. They do not prove that every
search path is correct.

The traced scores agree at every step, including the quiescence continuation,
and the final static evaluation equals the reported root score. History keys
and incremental history context are restored after tracing.

Raw controls: [d5-e5-search-check.json](d5-e5-search-check.json).

## Cutoff positions and the quiet move

The Scandinavian principal line is:

`1. e4 d5 2. exd5 Qxd5 3. Nc3 Qe5+ 4. Qe2 Nc6 5. Qxe5 Nxe5`

The last two captures are searched in quiescence. At the final board, Black's
knight on e5 receives 24 cp of middlegame centralization advantage over White's
knight on c3, plus 5 cp of phase-blended correction toward the larger retained
endgame weight. White receives 12 cp for mobility and 16 cp for initiative,
leaving −1 cp for White.

White can legally play the **quiet** move `6. d4`, attacking the e5 knight.
The engine's tactical generator does not include that move in quiescence.
Immediately after that push, the same evaluator reports **+38 cp for White**.
This is a static score before Black's response, not a proven minimax value of
the resulting position. It demonstrates that the leaf score is vulnerable to
a quiet pawn move which the cutoff does not explore.

Raw quiet-move check: [d5-e5-quiet-push-check.json](d5-e5-quiet-push-check.json).

For `...e5`, the depth-six principal line is:

`1. e4 e5 2. Nf3 Nc6 3. d4 exd4 4. Nxd4 d5`

That cutoff instead rewards White's centralized knight. The complete
white-relative decomposition is:

| Term | After the d5 line | After the e5 line | e5 minus d5 |
|---|---:|---:|---:|
| Piece-square placement | −24 | +24 | +48 |
| Mobility | +12 | −4 | −16 |
| Threats | 0 | −6 | −6 |
| Material-phase correction | −5 | 0 | +5 |
| Initiative | +16 | +24 | +8 |
| Other terms combined | 0 | 0 | 0 |
| **Total** | **−1** | **+38** | **+39** |

This attributes the reported score difference to its actual leaves. It does
not imply that simply subtracting 48 cp from a constant would fix minimax:
changing a weight can change both principal lines.

## Depth sensitivity

Independent fresh-cache searches, with iterative deepening within each run:

| Additional search plies after the reply | d5, White cp | e5, White cp | Gap favoring d5 |
|---:|---:|---:|---:|
| 5 | +12 | +23 | 11 |
| 6 | −1 | +38 | 39 |
| 7 | +10 | +29 | 19 |
| 8 | +15 | +29 | 14 |

The ranking remains too favorable to d5 compared with the stored depth-50
reference, but the 39 cp difference is not stable as depth increases. Lower
average error over the calibration set did not resolve this individual
preference. Centralized knights vulnerable to upcoming pawn pushes remain an
evaluation weakness; quiescence's omission of quiet continuations makes that
weakness particularly visible at this cutoff.

Raw depth comparison: [d5-e5-depth-check.json](d5-e5-depth-check.json).

## Reproduction

```sh
python3 benchmarks/opening-evaluation/diagnose_preferences.py --depths 6 --output /tmp/d5-e5-search-controls.json
python3 benchmarks/opening-evaluation/diagnose_preferences.py --modes cached --depths 5 6 7 8 --output /tmp/d5-e5-depth-follow-up.json
```

The script appends [diagnostic-driver.rs](diagnostic-driver.rs) and an isolated
evaluation-breakdown function to temporary source copies. Production has no
extra per-node tracing or instrumentation. All binaries are compiled before
running controls, using optimized builds with LTO and one search thread.
These are correctness and score diagnostics, not primary runtime benchmarks.
