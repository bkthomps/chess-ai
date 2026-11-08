# Project guidance

## Constraints

- Keep the application dependency-free: Rust standard library, bundled assets, and plain browser JavaScript/CSS. Do not add crates, frontend packages, external assets, or required network services.
- Support Rust 1.70 or newer, as declared in `Cargo.toml`.
- Use the same engine logic on macOS, Windows, and Linux. Cache storage uses ordinary Rust vectors; do not reintroduce platform-specific allocation or cleanup paths.
- Preserve the default **100,000,000-byte total server RAM budget**. This includes reservations for server state, stacks, scratch storage, and allocator overhead; it is not a 100 MB cache allowance. Browser memory is separate. Experimental benchmark budgets do not change the application default.
- Preserve CPU headroom. The portable thread policy reserves roughly a quarter of logical CPUs and always permits at least one search thread. The single-logical-CPU path yields periodically. Parallel phases must not multiply the configured concurrency limit.
- Treat these as defaults to preserve unless the user explicitly requests a change.

## Source map

- `src/chess.rs`: board rules, legal/tactical move generation, position identity, and incremental hashing.
- `src/tables.rs`: compiled board geometry and lookup tables.
- `src/engine.rs`: evaluation, search, parallel scheduling, pondering, progress, and time allocation.
- `src/ordering.rs`: per-thread history and killer move ordering.
- `src/cache.rs`: bounded shared transposition table, allocation, growth, and eviction.
- `src/clock.rs` and `src/game.rs`: clocks, move history, results, and undo.
- `src/main.rs`: local HTTP server, API validation, game state, and worker coordination.
- `web/`: bundled browser interface.
- `tests/`: Rust integration tests, rule fixtures, and HTTP/UI smoke harnesses.
- `benchmarks/`: reproducible performance experiments, raw measurements, and analysis.

Read the relevant implementation and `README.md` before changing behavior. Check the working tree and preserve unrelated user changes.

## Engine correctness

- The current engine uses iterative deepening, alpha-beta negamax, PVS, quiescence, cached/capture ordering, quiet history/killers, and parallel leading-branch splitting. Aspiration windows were removed after measurement; their archived experiment is not part of the application build. Do not silently restore it.
- Keep search off the game-state/UI path. Thinking and pondering share cache work, obey cancellation, and discard stale results when the game changes.
- Return the last fully completed iteration when interrupted. Bounds from unfinished work or failed probes must not become exact scores or unproven best moves.
- Preserve full cache-key checks, reversible-history and halfmove-clock context, and mate-score normalization at the actual ply. Restore history context after every explored move, including cancellation paths.
- Verify terminal and draw states in split subtrees. Every active helper must finish before its phase returns or its buffers/cache can be released.
- Maintain legal moves, scores, mate distances, repetition, clocks, and undo behavior when optimizing. Changes to evaluation, pruning, or target depth must be explicit rather than hidden in a performance change.
- Live diagnostics must work during thinking and pondering without adding blocking synchronization or expensive instrumentation at every node.

## Verification

Use checks appropriate to the change. For engine or server changes, the standard checks are:

```sh
cargo fmt -- --check
cargo test --offline
cargo clippy --all-targets --offline -- -D warnings
cargo build --release --offline
python3 tests/http_smoke.py
node tests/ui_smoke.cjs
```

Build the release executable before the HTTP smoke test. That test binds a local socket and may need sandbox permission. The UI harness uses a simulated DOM/audio environment; it does not replace visual browser testing for layout changes. Documentation-only changes generally need content/link and whitespace checks, not a full engine test run.

For search/scheduling changes, cover serial/parallel score agreement, proof of the chosen best move, mate normalization, terminal/draw states, cancellation, and history restoration as applicable. Keep tests meaningful; do not add tests that merely repeat the implementation.

## Performance work

- Isolate one proposed optimization at a time and compare it with the immediately preceding engine. Keep other search rules, evaluation, memory budgets, and thread limits equal.
- Benchmark optimized builds without competing builds, tests, or other agent-run performance studies. Record compiler flags, including whether LTO is enabled; the application release profile enables LTO with one codegen unit.
- Use repeated fresh-process trials, cold caches, and randomized or balanced variant order. Increase repetitions when results are noisy rather than drawing conclusions from a small timing difference.
- Measure both time to an equal completed depth and completed depth under equal thinking-time budgets. Include a serial control for parallel changes and warm-cache pondering when relevant.
- Save raw measurements and a readable analysis under `benchmarks/<experiment>/`. Record the baseline revision/source hashes, FENs, repetitions, thread configuration, RAM budget, actual cache allocation, and peak RSS where available.
- Keep instrumentation in temporary benchmark source copies. Diagnostic builds explain behavior; use uninstrumented builds for primary timing claims.
- Check fixed-depth score agreement. Equal-valued moves may differ; parallel node counts may vary with scheduling. Distinguish completed iterations from unfinished work.
- Report regressions, ranges, and position dependence. Fewer nodes or higher throughput does not guarantee an extra ply or stronger play; do not infer Elo from these benchmarks.
- Preserve historical data and clearly label archived/removed implementations. Keep their reproduction scripts functional without enabling experimental code in the application. Use a separate output path for smoke runs or follow-up studies.

See each experiment's README for its exact commands and limitations. Existing memory, ordering, PVS, aspiration, and parallel studies are evidence for their recorded configurations, not universal conclusions about those techniques.
