# Depth 10: 20, 40, and 60 MB cache ceilings

5 fresh-process, cold-cache repetitions per size, position, and thread count. All trials complete depth 10. Cache ceilings are decimal MB. Speed is the reciprocal of elapsed time; 60 MB is 1.000×. Each aggregate is the geometric mean of per-position median speed ratios, weighting positions equally. Controls are from this batch; historical timings are not pooled.

| Cache MB | One-thread speed | Change | Six-thread speed | Change |
|---:|---:|---:|---:|---:|
| 20 | 0.9529× | -4.71% | 0.9388× | -6.12% |
| 40 | 0.9935× | -0.65% | 1.0003× | +0.03% |
| 60 | 1.0000× | +0.00% | 1.0000× | +0.00% |

## Per-position measurements

| Position | Threads | Cache MB | Speed vs 60 MB | Median ms (range) | Node range | Hit range | Max cache MB | Max RSS MB | Score | Best moves |
|---|---:|---:|---:|---|---|---|---:|---:|---:|---|
| opening | 1 | 20 | 0.9890× | 16102.87 (15987.57–16186.68) | 36567160–36567160 | 516847–516847 | 19.93 | 22.02 | 21 | e2e4 |
| opening | 1 | 40 | 0.9880× | 16118.60 (15890.03–17439.04) | 36125612–36125612 | 561314–561314 | 39.94 | 42.07 | 21 | e2e4 |
| opening | 1 | 60 | 1.0000× | 15924.97 (15779.46–17093.55) | 36040443–36040443 | 573763–573763 | 59.94 | 62.08 | 21 | e2e4 |
| opening | 6 | 20 | 0.9319× | 5454.58 (5102.31–5701.75) | 43847240–46202977 | 501273–547222 | 19.93 | 24.41 | 21 | e2e4 |
| opening | 6 | 40 | 0.9903× | 5133.24 (4916.28–5428.05) | 42271037–44733304 | 563360–604175 | 39.94 | 44.43 | 21 | e2e4 |
| opening | 6 | 60 | 1.0000× | 5083.36 (4921.49–5345.67) | 42220679–43422491 | 605958–644719 | 59.94 | 64.45 | 21 | e2e4 |
| middlegame | 1 | 20 | 0.8778× | 81442.76 (80672.33–81839.75) | 152873161–152873161 | 977987–977987 | 19.93 | 22.02 | -82 | d5e6 |
| middlegame | 1 | 40 | 0.9894× | 72254.86 (72077.97–72346.93) | 135476517–135476517 | 1284543–1284543 | 39.94 | 42.06 | -82 | d5e6 |
| middlegame | 1 | 60 | 1.0000× | 71492.24 (70395.67–75118.10) | 131397051–131397051 | 1372623–1372623 | 59.94 | 62.08 | -82 | d5e6 |
| middlegame | 6 | 20 | 0.9196× | 19971.83 (19578.49–22079.17) | 154628443–155227927 | 955218–966544 | 19.93 | 24.43 | -82 | d5e6 |
| middlegame | 6 | 40 | 1.0392× | 17673.21 (17460.26–18999.58) | 136360060–136740695 | 1283983–1288625 | 39.94 | 44.45 | -82 | d5e6 |
| middlegame | 6 | 60 | 1.0000× | 18366.54 (17232.69–18918.49) | 132070478–132195146 | 1387156–1392152 | 59.94 | 64.47 | -82 | d5e6 |
| endgame | 1 | 20 | 0.9968× | 957.10 (946.00–962.02) | 3180913–3180913 | 64768–64768 | 15.86 | 17.92 | 14 | b4f4 |
| endgame | 1 | 40 | 1.0032× | 950.98 (942.23–970.02) | 3180913–3180913 | 64768–64768 | 15.86 | 17.91 | 14 | b4f4 |
| endgame | 1 | 60 | 1.0000× | 954.06 (942.35–979.81) | 3180913–3180913 | 64768–64768 | 15.86 | 17.91 | 14 | b4f4 |
| endgame | 6 | 20 | 0.9653× | 426.72 (384.82–445.99) | 3789851–4184165 | 82544–93870 | 16.84 | 21.12 | 14 | b4f4 |
| endgame | 6 | 40 | 0.9724× | 423.60 (394.45–445.34) | 3721666–4153683 | 89381–92782 | 16.84 | 21.14 | 14 | b4f4 |
| endgame | 6 | 60 | 1.0000× | 411.92 (380.54–450.49) | 3699449–4178340 | 85606–94285 | 16.84 | 21.14 | 14 | b4f4 |

## Scope and validation

FENs, source hashes, base revision, compiler, flags, thread configuration, experimental RAM allowance, direct cache ceilings, allocations, RSS, and timestamps are recorded in the JSON. The source-copy driver uses full iterative deepening and release-equivalent optimization including LTO and one codegen unit. Size order is randomized within each repetition. No agent builds, tests, or other performance studies run during collection. The application default remains 60 MB under its 100 MB total server RAM budget.

Fixed-depth scores agree across sizes and thread counts. Serial scores, moves, node counts, hits, and allocations repeat for each size. Parallel scheduling can change node counts and tie-breaking. Full timing ranges are shown; small differences with overlapping ranges do not establish a reliable optimum. These depth-only measurements do not measure completed depth under equal time allowances, warm-cache pondering, or playing strength.
