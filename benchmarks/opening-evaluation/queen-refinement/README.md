# Refining the queen-development term

This stage refines the previous blanket queen penalty while keeping knight
centralization at 6/10, initiative at 24, and all existing search/evaluation
constants unchanged. It adopts a narrower, middlegame-only positional term in
`src/engine.rs`. This is score calibration, not a timing or playing-strength
study.

## Selected definition

For each queen beyond its own first two ranks:

```
penalty = 24 * home_minors * min(file, 7 - file)^3 / 27
```

`home_minors` counts friendly knights and bishops on b1/c1/f1/g1, or their
Black equivalents. Files are numbered 0–7. Integer division truncates the
per-queen penalty before applying the existing material-phase blend. The
signed term is added to the middlegame score and subtracted from the endgame
delta, so its endgame endpoint is zero. It does not change the existing
knight endpoints or the other evaluation terms.

The exposure numerator across files a–h is `[0, 1, 8, 27, 27, 8, 1, 0]`.
An edge queen receives no direct cost; a central queen receives the full
24 cp per undeveloped minor. The penalty disappears when the home minors
develop. It is a geometric prior, not proof that a queen can be attacked:
there are no opening names, line recognition, square-specific exceptions,
random choices, legal-move probes, or retained state. Existing piece masks
are reused, with no added heap allocation or synchronization. A small
compiled file-factor array avoids runtime exponentiation.

## Experiment stages

The immutable baseline is the previous stage's
[`baseline-engine.rs`](../constant-follow-up/baseline-engine.rs), SHA-256
`bf4a7ba8abf7fe1504c78eae4734e328775043b18838e607cef690b202638952`.
Shared modules use revision `bdb77b1c4af864b24b9dd81a4906110ef970b129`.
The driver is [refine_queen.py](../refine_queen.py), which extends the existing
offline harness. Every variant's exact source replacements and source hash
are saved with its measurements. Instrumentation remains in temporary copies.

1. `focus-screen.json`: current engine, blanket24, linear file exposure at
   weights 16/24/32, and future knight/minor queen-chase bonuses. The focus set
   contains `e4 e5`, `e4 d5`, and `e4 d5 exd5 Qxd5 Nc3 Qa5`. Each is searched
   for six and eight additional plies after its own sequence. These are
   three diagnostic positions, not a broad validation cohort.
2. `gold-validation.json`: linear weights 16/24/32 on all 46 stored depth-50
   references, at additional search depths 4/5/6/7.
3. `broad-validation.json`: linear16/24 on all 299 references at depth six.
4. `convex-focus.json`: quadratic/cubic file falloff at central weights 24/32,
   on the same three focus positions at depths six/eight.
5. `convex-gold.json`: quadratic24/cubic24 on the 46 depth-50 references at
   depths 4/5/6/7.
6. `convex-broad.json`: both convex candidates on all 299 references at six.
7. `application-verification.json`: production implementation on the 46
   depth-50 references at depths 4/5/6/7. All **184 rows** exactly match the
   selected cubic24 candidate's FEN, static/search scores, moves, nodes,
   hits, and actual/maximum cache allocation.
8. `root-checks.json`: ordinary production root searches from the start,
   after e4, and after d4. Root depth includes the move being selected.
9. `diagnostic-check.json`: updated evaluation decomposition and principal-line
   tracing at depth six after d5/e5. Every traced score agrees with its root
   score, the decomposed leaf matches static evaluation, and history keys and
   context are restored. The diagnostic supports the new queen term while
   continuing to support historical sources without that term.
10. `move-rankings-depth7.json`: subsequent exhaustive ranking of all 20 legal
    moves from each of the three opening decisions. Each candidate is scored
    with six further plies, a fresh process/cache, and full played history.
    These scores can be compared with the original depth-seven top-ten tables.

The chase experiments estimate one-move threats on the current board using
relatively safe minor destinations and pin-restricted attack maps. They
exclude queens already under a minor/pawn attack, count at most once per
queen, and fade to zero in the endgame. Bishop versions use conservative
reverse rays under current occupancy. These are positional estimates, not
fully simulated legal moves or exchange proofs. They did not reliably
correct the focus ordering at both depths, and were not adopted.

Every measurement uses one thread, a fresh process/cache, full played history,
and the unchanged 100,000,000-byte total budget. Target iterations must
complete or the driver fails. Depth eight after a Black reply corresponds
to nine plies from Black's decision after e4, including the reply. Builds use
`--edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`, with LTO enabled.
Raw JSON records compiler/platform, hashes, FENs, nodes, hits, and actual cache
allocation. No peak-RSS or speed claim is made. Studies, builds, and tests
run sequentially. Original measurements were preserved in their original
paths.

The same 46 references selected the constants, so the results are in-sample.
They cover 44 distinct boards, and only nine eligible sibling comparisons
separated by at least 20 reference cp. The 299-position set includes varying
lower-depth references; it is a regression diagnostic, not an independent
test set. See [results.md](results.md) for per-depth accuracy, all measured
focus variants, remaining regressions, and root choices.

## Reproduction

Git, Rust, and Python's standard library suffice; no new dependencies or
network access are required. Use fresh output paths:

```sh
python3 benchmarks/opening-evaluation/refine_queen.py --variants current early-queen24 queen-file-16 queen-file-24 queen-file-32 knight-chase-16 knight-chase-24 knight-chase-32 minor-chase-16 minor-chase-24 --scope focus --depths 6 8 --output /tmp/queen-focus.json
python3 benchmarks/opening-evaluation/refine_queen.py --variants queen-file-16 queen-file-24 queen-file-32 --scope gold --depths 4 5 6 7 --output /tmp/queen-linear-gold.json
python3 benchmarks/opening-evaluation/refine_queen.py --variants queen-file-16 queen-file-24 --scope all --depths 6 --output /tmp/queen-linear-broad.json
python3 benchmarks/opening-evaluation/refine_queen.py --variants queen-file-power2-24 queen-file-power2-32 queen-file-power3-24 queen-file-power3-32 --scope focus --depths 6 8 --output /tmp/queen-convex-focus.json
python3 benchmarks/opening-evaluation/refine_queen.py --variants queen-file-power2-24 queen-file-power3-24 --scope gold --depths 4 5 6 7 --output /tmp/queen-convex-gold.json
python3 benchmarks/opening-evaluation/refine_queen.py --variants queen-file-power2-24 queen-file-power3-24 --scope all --depths 6 --output /tmp/queen-convex-broad.json
python3 benchmarks/opening-evaluation/refine_queen.py --variants application --scope gold --depths 4 5 6 7 --output /tmp/queen-application.json
python3 benchmarks/opening-evaluation/refine_queen.py --variants application --scope roots --depths 7 9 --output /tmp/queen-roots.json
python3 benchmarks/opening-evaluation/diagnose_preferences.py --modes cached --depths 6 --output /tmp/queen-diagnostic.json
python3 benchmarks/opening-evaluation/summarize_refinement.py
python3 benchmarks/opening-evaluation/rank_openings.py --depth 7 --output /tmp/queen-rankings.json
```

`application` uses the current `src/engine.rs`; other variants use the archived
baseline. Updating the application does not change the historical candidates.

## Verification

All standard application checks passed after adoption:

```sh
cargo fmt -- --check
cargo test --offline
cargo clippy --all-targets --offline -- -D warnings
cargo build --release --offline
python3 tests/http_smoke.py
node tests/ui_smoke.cjs
```

The Rust suites passed 110 tests; two explicit runtime benchmarks remain
ignored. The new behavior test checks home-rank exemptions, file exposure,
color symmetry, the home-minor dependency, and exclusion of opposing/moved
minors. Independent arithmetic evaluation and playout checks include the new
term; existing tests still cover serial/parallel proof, mate/draw handling,
cancellation, history restoration, and the memory/thread defaults. Production
candidate parity, complete depth-nine root choices, diagnostic decomposition,
Python syntax, JSON, documentation links, and whitespace were also checked.
