# Piece–pawn evaluation optimization

This experiment starts from the exact uncommitted relationship evaluator preserved in [baseline-engine.rs](baseline-engine.rs). Its parent revision is `65810da150f1b7335a1eec98aeebec30412d4ceb`; that revision alone does not include the new relationships. The baseline source matches the earlier relationship study candidate. No prior measurements or source snapshots are overwritten.

Three proposals are screened separately: compute both pawn-file masks once, share geometry between pawn structure and relationships, and skip work for absent piece types. File-mask hoisting is screened first because geometry sharing also supplies those masks. Each retained change becomes the next proposal's baseline. All terms, phase blending, search rules, memory budgets, and thread policies must remain identical.

## Reproduction and method

The existing [comparison script](../piece-pawn-relationships/compare.py) accepts explicit frozen engine sources. For example:

```sh
python3 benchmarks/piece-pawn-relationships/compare.py --baseline benchmarks/piece-pawn-optimization/baseline-engine.rs --candidate benchmarks/piece-pawn-optimization/files-engine.rs --require-equivalent --threads 1 --modes depth --repetitions 7 --output benchmarks/piece-pawn-optimization/files-follow-up.json
```

Keep each follow-up output distinct from historical data. Screens use seven repetitions, all eight original FENs/depths, fresh processes/cold caches, serial search, seeded shuffled variant order, and no competing builds/tests/performance studies. Both variants compile before timing with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`, matching application release LTO. Shared chess, geometry tables, cache, and ordering sources are checked against the parent revision. Raw JSON records source hashes, compiler/platform/flags, total 100,000,000-byte budget, actual cache allocation, RSS, threads, FENs/depths, scores, moves, nodes, hits, and elapsed times. Markdown files record medians/ranges.

`--require-equivalent` checks all root-child static scores and fixed-depth scores across both variants and serial/parallel configurations, and exact serial moves/nodes/hits. Candidate correctness also uses the existing independent coordinate oracle and playout tests. Instrumentation/drivers are appended only to temporary benchmark copies. Frozen engine sources here are reproduction artifacts, not application modules.

A retained-candidate follow-up includes serial/automatic threading, equal-depth/equal-time trials, and warm-cache pondering where relevant. Pure pawn endings control for timing noise because the relationship term is skipped there. Small overlapping timing differences are inconclusive; no Elo claims follow from timing or completed depth.

## File-mask screen

[Raw JSON](files-screen.json) / [ranges](files-screen.md): seven trials per position, 112 runs, full equivalence checks passed. Candidate [files-engine.rs](files-engine.rs) computes both full-file masks outside the color loop. Median differences were mixed: initial -1.18%, after e4 +0.70%, after d4 +0.35%, middlegame +0.08%, closed center +0.49%, rook ending +2.82%, promotion +1.42%, and pawn-ending control -0.04%. This does not establish an improvement; the standalone change was rejected. Code-level duplicate arithmetic need not survive compiler optimization. No fixed-time or parallel follow-up was warranted for this rejected variant. Geometry sharing is tested next against the unchanged starting evaluator.

## Shared-geometry screen

[Raw JSON](shared-screen.json) / [ranges](shared-screen.md): seven trials per position, 112 runs, full equivalence checks passed. Candidate [shared-engine.rs](shared-engine.rs) computes rear/forward pawn spans, neighboring forward spans, and occupied files once and passes the local geometry to both evaluation terms. The neighboring forward spans exactly match outpost challenge masks; occupied-file masks also remove the separate rook file fills. This is ephemeral evaluation data, not a new retained cache.

Median differences ranged from -0.85% to +0.82% across all eight positions, with no clear improvement (initial +0.08%, after e4 +0.14%, after d4 -0.82%, middlegame -0.16%, closed center +0.12%, rook ending +0.82%, promotion -0.85%, control +0.11%). The refactor was rejected. These results do not prove the compiler's exact mechanism, but demonstrate that fewer expressions in source did not produce a useful measured speedup in this build. Skipping absent types is therefore screened against the unchanged starting evaluator as well.

## Absent-piece screen

[Raw JSON](skip-screen.json) / [ranges](skip-screen.md): seven trials per position, 112 runs, full equivalence checks passed. Candidate [skip-engine.rs](skip-engine.rs) wraps minor-piece/outpost, bishop-color, and rook-file calculations in checks for the corresponding piece types. Enemy pawn challenge spans are calculated only for a color with minor pieces. Pawnless rook bonuses remain enabled. Median differences were +0.28% to +0.77% including the control, so this screen showed no benefit. The small differences alone do not establish that branches are intrinsically slower.

## Longer confirmation

Because all three screens produced small differences, [confirm.py](confirm.py) increases the sample to 15 trials per variant/configuration on initial, middlegame, rook-ending, and pawn-ending positions. It interleaves all four variants in seeded shuffled order with fresh processes/cold caches and one thread. All builds finish before timing. It measures both equal completed depth and completed depth at 100 ms; strict static-score and serial score/move/node/hit equality is enforced. This is a confirmation screen, not a new search/scheduling experiment.

```sh
python3 benchmarks/piece-pawn-optimization/confirm.py --output benchmarks/piece-pawn-optimization/confirmation-follow-up.json
```

The main [confirmation JSON](confirmation.json) and [median/range table](confirmation.md) preserve the original confirmation run. The script uses the existing comparison compiler/driver; it compiles a baseline/candidate pair for each proposal before timing and retains one baseline executable. Reproduction requires Rust, Git, and Python's standard library only.

## Decision and confirmation results

**None of the three proposals was retained.** The longer confirmation still shows no consistent measurable improvement. The application evaluator remains byte-for-byte equal to the starting relationship evaluator; no weights, search behavior, memory reservations, or thread policy changed.

| Position | File masks once | Shared geometry | Skip absent types |
|---|---:|---:|---:|
| Initial | -0.12% | -0.29% | +0.27% |
| Middlegame | +0.10% | -0.28% | +0.11% |
| Rook ending | +0.66% | +0.14% | +0.07% |
| Pawn-ending control | -0.13% | -0.36% | -0.01% |

Values are percentage changes in median equal-depth elapsed time, positive slower. These small differences overlap observed timing variation. The shared-geometry change's tiny apparent gains are comparable to its pawn-ending control variation, where relationship evaluation is skipped. Source simplification alone did not justify retaining the refactor.

At 100 ms all variants completed initial-position depth 6 in every trial. The middlegame completed depth 4 except one baseline depth-3 trial. Rook-ending depth 8 was reached in 13/15 baseline trials, 12/15 file-mask trials, 13/15 shared trials, and 14/15 skip trials; the rest reached depth 7. The pawn-ending control reached depth 10 except one baseline depth-9 trial. These occasional boundaries do not demonstrate a reliable extra ply. Peak recorded process RSS was 14.39 MB in confirmation for these workloads; this is not a general memory-limit proof.

Across the screens and confirmation, 816 fresh-process measurements were saved. All static diagnostics and fixed-depth score/move/node/hit equivalence checks passed. The candidates did not reduce search work. Since no candidate cleared the serial screen/confirmation, no parallel or warm-cache promotion study was warranted. The original feature study's opening slowdown remains primarily additional search work; these source-level proposals did not solve it.

## Verification

The candidate archives and baseline match their recorded SHA-256 hashes; the baseline also matches the prior feature study and current `src/engine.rs`. Both benchmark scripts parse, local documentation links resolve, and whitespace checks pass. No application source changes were retained in this optimization turn, so the preceding feature's standard application checks remain applicable; this turn's verification specifically exercises the experimental comparisons above. Historical feature data and rejected variants remain intact.
