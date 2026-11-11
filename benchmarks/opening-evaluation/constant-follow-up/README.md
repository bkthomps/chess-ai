# Follow-up: the Scandinavian preference

This historical stage investigates the calibrated application's preference for
`1. e4 d5` over `1. e4 e5`. It left the application's evaluation unchanged.
All proposed changes in this stage were compiled only into temporary copies.
The later [queen refinement](../queen-refinement/README.md) records the
narrower feature subsequently adopted in the application.

## Baseline and method

`baseline-engine.rs` preserves the exact uncommitted engine at the start of
this investigation (SHA-256
`bf4a7ba8abf7fe1504c78eae4734e328775043b18838e607cef690b202638952`).
Shared modules come from revision `bdb77b1c4af864b24b9dd81a4906110ef970b129`.
The reference dataset is the existing 299-position collection, including 46
depth-50 references representing 44 distinct boards; no new references were
collected. See the parent README for collection gaps and selection bias.

The baseline retains knight center weights of 6/10 and full-material tempo 24.
Every candidate changes one criterion relative to that baseline. Searches use
one thread, fresh processes/caches, complete played history, the unchanged
100,000,000-byte total budget, and iterative deepening to the requested depth.
Each recorded result must finish its target depth; an interrupted iteration is
an error. Depths here are **additional plies after the recorded sequence**:
depth eight after `e4 d5` corresponds to nine plies from Black's decision
after `e4`, including `d5`. Root checks are measured from their own positions.

Build flags are `--edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`.
Raw JSON includes compiler, platform, source hashes, exact replacements, FENs,
scores, moves, nodes, hits, actual cache allocation, and cache limit. This is a
deterministic score study: there are no timing, peak-RSS, throughput, or Elo
claims. Compilations and studies run sequentially, without competing tests.

## What classical Stockfish suggests

Stockfish 11's [piece-square tables](https://github.com/official-stockfish/Stockfish/blob/sf_11/src/psqt.cpp)
retain strong knight placement preferences. Its
[classical evaluator](https://github.com/official-stockfish/Stockfish/blob/sf_11/src/evaluate.cpp)
combines them with mobility tables, pawn-supported outposts that enemy pawns
cannot challenge, and threats produced by relatively safe pawn pushes. The
latter considers both single and double advances. These ideas support adding
context around centralization rather than eliminating it.

Its [internal piece values](https://github.com/official-stockfish/Stockfish/blob/sf_11/src/types.h)
use a different scale from this engine; Stockfish's raw constants cannot be
copied as this engine's centipawns. Sources were inspected on 2026-11-11.
No Stockfish code or tables were imported.

The experimental pawn-push term is an original, simpler positional estimate:
count distinct enemy non-pawn, non-king pieces attacked after an empty-path
pawn advance to a square not attacked by enemy pawns and either defended by
us or unattacked by the enemy. It does not simulate full legality, pins,
exchanges, or check evasions. Trials add 8, 16, 24, or 32 middlegame cp per
target, with no endgame term; other weights remain unchanged.

The early-queen trial is a separate heuristic, **not a Stockfish constant**.
It penalizes a queen beyond its own first two ranks by a configurable amount
per friendly knight/bishop still on b1/c1/f1/g1 (or the mirrored Black
squares). The penalty uses the existing material-phase blend and fades to
zero in the endgame. It scores board features, without identifying openings,
changing search, or retaining new state. Its simplifications can penalize a
usefully active queen, including positions where delaying a minor's
development is appropriate.

## Files and reproduction

The driver is [followup_constants.py](../followup_constants.py). The initial
screen is `pair-screen.json` (24 variants including the baseline); subsequent
files retain their own exact candidate lists and cohorts. Later additions to
the script's available candidates do not change earlier recorded cohorts.
Use separate output paths to retain historical data:

```sh
python3 benchmarks/opening-evaluation/followup_constants.py --variants current knight_center_middlegame-0 queen-mobility-0 king-safety-half early-queen12 --scope pair --depths 6 8 --output /tmp/follow-up-pair.json
python3 benchmarks/opening-evaluation/followup_constants.py --variants pawn-push-threat-8 pawn-push-threat-16 pawn-push-threat-24 pawn-push-threat-32 --scope pair --depths 6 8 --output /tmp/follow-up-pawn-push.json
python3 benchmarks/opening-evaluation/followup_constants.py --variants current early-queen12 pawn-push-threat-16 pawn-push-threat-24 --scope gold --depths 4 5 6 --output /tmp/follow-up-gold.json
python3 benchmarks/opening-evaluation/followup_constants.py --variants early-queen4 early-queen8 early-queen16 early-queen24 --scope gold --depths 4 5 6 --output /tmp/follow-up-queen.json
python3 benchmarks/opening-evaluation/followup_constants.py --variants early-queen4 early-queen8 early-queen16 early-queen24 --scope pair --depths 8 --output /tmp/follow-up-queen-depth8.json
python3 benchmarks/opening-evaluation/followup_constants.py --variants current early-queen16 early-queen24 --scope all --depths 6 --output /tmp/follow-up-broad.json
python3 benchmarks/opening-evaluation/followup_constants.py --variants current early-queen16 early-queen24 --scope gold --depths 7 --output /tmp/follow-up-depth7.json
python3 benchmarks/opening-evaluation/followup_constants.py --variants current early-queen16 early-queen24 --scope roots --depths 7 --output /tmp/follow-up-roots.json
```

Detailed tables and regressions are in [results.md](results.md).
Rebuild that report with `python3 benchmarks/opening-evaluation/summarize_followup.py`.

## Verification performed

The rerun baseline matches all 138 previous application-verification rows at
depths four through six, including static scores, searched scores, best moves,
nodes, and cache hits. The application engine is byte-identical to the saved
baseline snapshot at completion of this stage. Python syntax, raw JSON, report generation,
and whitespace checks passed. The full application test suite was not rerun:
this follow-up changes only benchmark scripts, measurements, and documentation.
