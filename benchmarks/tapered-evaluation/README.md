# Gradual middlegame/endgame blending

This experiment isolates replacing the king evaluation's abrupt endgame switch with a material-weighted blend. The baseline is commit `5c531c93ca74740b91a918f597bc482aef7ca65f`, the application immediately before this change. Earlier uncommitted opening-book preferences discussed in the conversation were no longer present in that baseline; this comparison uses the current application's ordinary search, without random opening bonuses.

## Change

The old evaluator used its endgame king formula whenever at most two rooks and queens remained, regardless of the remaining minor pieces. Otherwise it used its middlegame formula. The new evaluator assigns phase weights of one to each knight/bishop, two to each rook, and four to each queen. Starting material totals 24. It blends the old king endpoints using endgame weight `(24 - min(phase, 24)) / 24`, rounding the combined signed king correction toward zero. Promotions cannot extrapolate past the middlegame endpoint. Pawns/kings carry no phase.

Material values, pawn/knight/bishop/rook/queen positional formulas, and the bishop-pair bonus retain their existing values in both phases. Thus only king evaluation differs. This is deliberately an isolated change, not a port of Stockfish's evaluation tables. Search rules, target depths, memory budget, cache policy, time allocation, and concurrency policy remain equal.

At phase 24 the old middlegame endpoint is preserved. At phase zero the old endgame endpoint is preserved. Between them, king activity acquires weight gradually. The removal of a particular rook or queen no longer triggers a wholesale switch affecting both kings, although the ordinary material/capture and positional changes still apply.

## Method and reproduction

Run with no competing builds, tests, or performance studies:

```sh
python3 benchmarks/tapered-evaluation/compare.py
```

Only Python's standard library, Git, and Rust are required. The script loads both engine sources from pinned revisions (baseline above, blended `80abdf6954393e0a4caffe85b602db40deedd317`) and checks that shared chess, geometry, cache, and ordering sources match that revision. It compiles separate temporary executables with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`. LTO is enabled, as in the application release profile. Both compilations finish before measurement. Production code has no added instrumentation.

Each measurement uses a fresh process and cold cache. Five trials use a seeded shuffle of variant order, a 100,000,000-byte total RAM budget, and both one search thread and the application automatic thread limit. Timed searches receive 100 ms or 500 ms. Fixed-depth searches complete all iterations to the same specified depth, including initialization, with a 60-second failure guard. Time-limited results report only the last completed iteration. Source hashes, compiler/platform, FENs, thread counts, raw scores, best moves, nodes, hits, allocation, elapsed time, and peak RSS are recorded in [results.json](results.json). The complete medians/ranges are in [results.md](results.md).

The script checks fixed-depth serial/parallel score agreement within each evaluator and repeatable serial fixed-depth scores, moves, and nodes. Scores between evaluators are allowed to differ because evaluation is the intentional change. Equal scores can have different best moves, and helper scheduling can vary nodes. Search unit tests independently verify legal/proven moves, cancellation, terminal/draw handling, history restoration, and mate normalization.

Use `--output benchmarks/tapered-evaluation/follow-up.json` for follow-ups and `--repetitions N` to adjust repetition count without overwriting historical measurements.

## Results

Five trials across seven FENs, two thread settings (one and six), two time allowances, and fixed-depth searches produced **420 measurements**. Every sampled best move was the same before and after blending, including all timed trials. Fixed-depth scores agreed between serial and parallel within each evaluator; each serial fixed-depth trial repeated the same score, move, and node count.

The initial-position searches chose **Nc3** in all 20 timed trials per evaluator (five repetitions × two budgets × two thread settings). After e4 and d4, both versions chose **Nf6** throughout. Starting-position completed depths were seven at 100 ms and eight at 500 ms for both variants and thread settings. Thus **this change did not improve the reported opening knight preference**. Full-material static positions use the same middlegame scores, and blending adds no reward for freeing bishops or gaining space. Captures or promotions in explored branches can still cause small differences in the search tree even when the root phase is unchanged.

At fixed depth, six of the seven FENs retained the same scores and all seven retained the same best move:

| Position | Target depth | Baseline score | Blended score | Best move in both |
|---|---:|---:|---:|---|
| Initial position | 8 | 0 | 0 | b1c3 |
| After e4 | 7 | 0 | 0 | g8f6 |
| After d4 | 7 | 0 | 0 | g8f6 |
| Middlegame | 6 | -10 | -10 | d5e6 |
| Rook ending | 10 | +40 | +27 | b4f4 |
| Tactical promotion | 6 | +479 | +479 | d7c8q |
| Pawn ending | 10 | +28 | +28 | g3f3 |

Scores are raw centipawns from the side to move. In the rook ending, two rooks leave phase 4: the new root evaluation uses 83.3% endgame king weight rather than the old 100%. Different values are expected; the lower score is not evidence of better accuracy. At 500 ms with six threads, the tactical case also changed score (+539 to +537), at completed depth seven, while choosing the same promotion.

Fixed-depth timing changes were small and position dependent. Serial medians changed between -1.4% and +0.5%; six-thread medians between -3.1% and +5.1%. All corresponding before/after timing ranges overlapped. This sample does not support a general speedup or slowdown. The biggest six-thread median increase was after d4 (80.18 → 84.23 ms); the rook ending decreased (230.83 → 223.68 ms). Different trees and helper scheduling contribute to these figures.

Timed depth was generally unchanged. Exceptions underline the limits of short benchmarks: after e4, one of five blended six-thread 100 ms trials completed depth six instead of seven (baseline five of five reached seven). At 500 ms in the middlegame, baseline reached depth six in three trials and seven in two, while blending reached seven in all five. In the serial pawn ending at 500 ms, baseline reached twelve in one trial and thirteen in four; blending reached thirteen in all five. These isolated timing-boundary changes do not establish a strength improvement.

Maximum observed process RSS was 26.1 MB, excluding the browser and server. Actual cache allocation and limits are preserved in each raw row; this shallow/short study does not fill the cache or validate a worst-case memory ceiling.

The demonstrated benefit is removal of the discontinuous king-formula switch. Playing-strength improvement remains unproven. Mobility and space remain separate candidates for the opening issue.

## Interpretation limits

This is a small search-behavior and timing study, not a playing-strength tournament. Different evaluations and moves do not establish better chess. Timed move/depth changes can come from timing boundaries, changed search trees, and scheduling. Short allowances and process scheduling make small timing differences particularly noisy. No Elo inference is justified. The suite includes initial play, a developed middlegame, a tactical promotion position, a rook ending, and a pawn ending; it does not comprehensively cover king safety, transitions, or endgame technique.

## Verification

Formatting, offline Rust tests (including the new blend endpoint/midpoint/promotion checks and the independent arithmetic evaluation reference), Clippy with warnings denied, the offline release build, and HTTP smoke tests pass. The UI smoke harness fails at `tests/ui_smoke.cjs:414` with an extra move request during drag cancellation (`13 !== 12`). The identical failure reproduces in an isolated copy of the unchanged baseline web files and harness; no web or UI-test code changed in this experiment.
