# chess-ai

A dependency-free Rust chess program with a local browser interface. Choose one-player mode against the computer or two-player mode on the same device. The compact menu contains mode, color, and time controls, with cream and sage colors and continuously falling chess pieces in the background. The animation pauses during gameplay and is hidden when reduced motion is enabled. The game screen contains the board, clocks for timed games, turn status, and basic controls. A right-hand sidebar shows move history and an expandable Inside the engine panel with live thinking and pondering status, the time allowance, completed and current search depths, nodes searched, cache hits, and elapsed search time. On small screens, the sidebar moves below the board. SVG pieces, legal-move hints, and check indicators keep the board easy to read.

## Run

Install Rust 1.70 or newer, then from this directory:

```sh
cargo run --release
```

Open **http://127.0.0.1:7878** in your browser. To choose another port:

```sh
cargo run --release -- --port 8080
```

There are no crate dependencies, external assets, accounts, or network services. The server binds only to the local machine. Keep the terminal running while playing; Ctrl+C stops it. Tabs share one game. Refreshing preserves the game while the server runs; restarting the server resets it.

## Play

- Select one or two players, and choose your color in one-player mode.
- Choose 1 min, 2 min + 1 sec increment, 5 min (default), or 10 min + 5 sec increment. Custom accepts 0.1–180 base minutes and a 0–600 second increment. Both players receive the same initial time and increment.
- Unlimited turns off both clocks. In one-player mode, choose a thinking time of 1–5 seconds per move (3 seconds by default), or Custom for any positive number of seconds, including fractions. Two-player Unlimited has no thinking-time setting.
- Click a piece, then a highlighted destination. White moves first.
- Use arrow keys to navigate the board, Enter or Space to select, and Escape to clear a selection.
- Castle by moving the king two squares. Choose queen, rook, bishop, or knight when promoting.
- Undo removes one move in local play or returns to your previous decision in solo play, restoring both clocks to that position. Flip board changes the view. New game opens the menu, where you can resume or start another game.
- Checkmate, stalemate, insufficient material, threefold repetition, and the fifty-move rule end the game. Repetition and fifty-move draws are applied automatically, without a separate claim step.
- Resign game opens a confirmation before conceding.
- Moves make a short click, check plays a two-note alert, and a finished game plays a three-note chime. The Sound button mutes or enables audio and remembers the setting in your browser. Sounds are synthesized locally; no audio files or downloads are needed. Restoring a game, undoing, and flipping the board are silent.

In timed games, White's clock starts when Start game is pressed. Only the player to move loses time; increment is added after an accepted move. Clocks keep running while the menu is open or the browser is inactive. Rust uses a monotonic server clock, and checks flags independently of browser polling. A late move cannot gain an increment to escape a timeout. Clocks stop when the game ends. Unlimited games never run a clock or award a timeout, including after undo.

Running out of time loses the game unless the opponent lacks mating material. The check is asymmetric and includes cooperative mating possibilities: a bare king cannot win, while two knights can. Ordinary material cases are covered; positional fortresses and forced capture sequences are not exhaustively solved.

## Engine

Rust handles every rule and move decision. The computer uses iterative-deepening negamax, a form of minimax, with alpha-beta pruning, Principal Variation Search, cached-move and capture ordering, killer/history ordering for quiet moves, quiescence search, and material/positional evaluation. Incremental 64-bit Zobrist hashing accounts for pieces, side to move, castling rights, and en-passant targets. The bounded transposition table stores depth, bound type, score, and best move; its identity also includes the reversible-history context and halfmove clock. The full key is checked before reuse. Repetition keys ignore en-passant targets that cannot legally be captured.

The shared transposition table grows on demand within a **100,000,000-byte (100 MB) default server RAM budget**. The cache ceiling subtracts 16 MiB for server state, 2.5 MiB per configured search thread for stacks and retained move buffers, and 2% for allocator/page overhead. With six threads, that leaves about 65.5 MB for cache allocations and their bookkeeping. The browser is a separate process and is outside this budget. `SearchCache::with_threads_and_memory` supports explicit budgets for comparisons, capped at 1 GB; larger experimental budgets reserve additional room for allocator-retained freed pages.

The cache starts with 4 MiB of bucket storage. Each 64-byte aligned bucket holds four full eight-byte position keys, four six-byte packed payloads, and an eight-byte shared epoch. A payload contains a signed 16-bit score, a 16-bit packed move, an eight-bit depth, and a byte for bound, occupancy, and compact age. This uses 16 bytes per slot including the shared epoch, rather than 14 bytes without it. Keeping a bucket together avoids separate key/payload memory accesses. Search calculations still use 32-bit scores; values outside the signed 16-bit range are left uncached rather than truncated or clamped.

The table uses 64 independently locked shards and 64 KiB pages. Busy shards are skipped rather than making a search wait for another thread. Sustained bucket collisions trigger growth, subject to one shared allocation ceiling. Linear hashing splits only one page per growth step and preserves the rest of the table, so it never reallocates or rehashes a giant contiguous array. Allocation failure leaves the existing cache usable. All threads share the same cache, including between pondering and computer moves.

Material checks scan the board without collecting pieces, and stable move ordering uses no heap scratch. Its small stack buffer has a fallback for larger move lists, so it does not impose a move-count limit. Equal-priority moves retain their original order.

Attack geometry, Zobrist features, and positional evaluation scores are computed at compile time in small immutable tables. Legal move generation computes check and absolute pins once per position, accepting unpinned non-king moves directly when safe. King moves, pinned moves, check evasions, and en passant still simulate the resulting board and check for attacks. These temporary legality checks skip hash updates. Quiescence generates captures and promotions directly outside check, while verifying that a legal move exists to distinguish quiet positions from stalemate; in check, it generates every legal evasion.

Each search thread reuses move-list allocations by ply. Move storage and quiet-ordering tables share a 512 KiB retained limit, plus a small vector of buffer headers. Larger move lists and deeper searches remain supported without increasing the retained pool limit. Reversible-history context is updated incrementally as search enters and leaves positions. A usable cached score returns before move generation, with the same full history-aware key checks. These optimizations use the same Rust implementation on every platform.

Quiet moves are non-captures without promotion. After the cached move, captures, and promotions, search tries two recent quiet cutoff moves (killers) for that ply, then orders remaining quiet moves by a bounded history score indexed by color, origin, and destination. Quiet beta cutoffs reward the successful move and penalize earlier quiet alternatives, with larger rewards for deeper cutoffs. Each thread owns its tables, adding no synchronization. Killers reset for each new search; history is halved and retained, including between pondering and thinking. The tables use 17,448 bytes per thread within the existing scratch allowance. This changes move order without adding pruning or reducing search depth. See [the measurements and tradeoffs](benchmarks/ordering/README.md).

Principal Variation Search searches the first ordered move with the full alpha-beta window. Later moves use a one-point window to check whether they improve the current best score. An improvement inside the original window receives a full-window re-search before it can establish an exact result; a proven beta cutoff needs no re-search. Nodes already using a one-point window skip the extra probe. This applies recursively in both thinking and pondering, building on the root workers' existing probes without changing quiescence or reducing any move's target depth. It adds no retained tables or allocations. See [the PVS comparison](benchmarks/pvs/README.md).

An aspiration-window experiment was removed after measurements showed little additional benefit over PVS. Its [benchmark results and tactical analysis](benchmarks/aspiration/README.md) are retained, with the experimental engine archived for reproduction.

Colliding positions can coexist within a bucket. Empty slots are used first. For the same position, deeper results are retained; at equal depth, exact scores take priority, and bounds are updated when tighter. Full buckets rank entries by depth plus a two-point exact-score bonus, minus four points per unused search generation. An incoming result replaces the weakest resident only if its rank is at least as high. Successful lookups refresh age. Each thinking or pondering search advances a 64-bit epoch in constant time. A bucket ages its four entries only when accessed, using its full epoch to retire stale entries safely even across many wraps of the compact six-bit age. There is no full-table aging sweep, so maintenance cost does not grow with the cache limit.

A persistent coordinator worker controls search and keeps the last completed depth under a time budget. At deeper iterations, it searches the leading root move first to establish a useful bound, then shares the remaining moves with helper threads through a dynamic work queue. Null-window probes identify moves that can improve the best result; those moves receive a full-window verification. An iteration is complete only after every root move is resolved, and all helpers finish before search returns. Shallow iterations run serially to avoid thread overhead. In one-player mode, the same search group also ponders during the human's turn, exploring possible human moves and computer replies without making a move or spending the computer's clock. Once the human moves, pondering is cancelled and the worker searches the actual position using the retained cache. Previously explored branches supply cached moves, bounds, and exact scores; an exact cached root resumes at the next depth. An unexpected move can benefit less because its branch may have received less search. Pondering uses the same thread limit while the human considers a move, including in Unlimited games and while the menu is open.

The automatic thread limit uses Rust's portable available-parallelism query and reserves roughly a quarter of logical CPUs, rounding the reservation up and always keeping at least one search thread. For example, two logical CPUs use one search thread, four use three, and eight use six. Search is capped at 64 threads. The default 100 MB budget further limits search to at most 18 threads, preserving at least 32 MiB for cache storage on machines with many CPUs. This leaves CPU capacity for the browser and other applications; it does not pin or exclusively reserve particular cores. On a machine reporting only one logical CPU, search runs on one thread and takes a short pause after roughly every nine milliseconds of computation, aiming to leave about 10% CPU time available. The same policy applies to thinking and pondering on macOS, Windows, and Linux.

The Inside the engine panel shows the current search, including during pondering. Depth reached is the last completed depth; Searching depth is the iteration in progress. Nodes, cache hits, and elapsed time update during an unfinished iteration and reset for each new position. Search threads shows the configured search-thread limit. Cache allocated / limit shows the storage currently allocated and its ceiling in decimal MB; unused budget is not preallocated. Node and hit counts combine the coordinator and helpers, with helper counters batched to avoid synchronization at every node. Pondering displays “Your turn” instead of a time allowance. The worker checks publication time every 1024 nodes and publishes a small snapshot about every 250ms, plus initial and final snapshots, without waiting on readers or locking the game state. The browser polls every 250ms during thinking and pondering, including Unlimited games; statistics-only updates preserve the board and selection. Finished searches retain their final snapshot until another search or a reset.

Timed games allocate approximately remaining time / 36 in the opening, rising toward remaining time / 18 as material is exchanged, plus 75% of the increment. The allocation reserves up to 250ms for scheduling and move submission and caps each search at 15% of usable remaining time. This is a time-management heuristic, not a claim of optimal play. Unlimited games use the selected thinking time for each computer move instead; pondering time is additional. Both modes keep deepening without a per-move depth target. A forced computer move, proven mate, or finished game can end the search early. New games, undo, resignation, and flags cancel pending searches and discard stale results. New games and successful undos also clear the search cache. The worker releases the cache allocation when the game ends or switches to two-player mode, and recreates it when solo search resumes. Opening the menu during an ongoing solo game keeps pondering and its cache. Two-player mode never searches or ponders.

Cache pages use ordinary Rust vectors on every platform, including macOS, Windows, and Linux. Cache cleanup runs on the worker, outside the game-state lock and outside active search. Dropping the cache returns its allocations to the system allocator; how quickly the process's physical memory usage falls depends on the OS and allocator.

## Verify

```sh
cargo test --offline
cargo clippy --all-targets --offline -- -D warnings
cargo build --release --offline
python3 tests/http_smoke.py
node tests/ui_smoke.cjs
```

To measure search throughput independently of a move's time allowance:

```sh
cargo test --release --offline engine::tests::benchmark_fixed_depth -- --ignored --nocapture
```

This serial benchmark searches five fixed positions with a fresh cache for each run, excludes cache initialization from timing, and reports the median of seven runs after a warmup. It prints depth, nodes, cache hits, score, best move, elapsed milliseconds, and retained move-buffer bytes. Nodes, scores, and moves must remain consistent between repetitions. Run it without competing CPU-intensive work for useful timing comparisons.

To compare serial and parallel iterative deepening at the same completed depths:

```sh
cargo test --release --offline engine::tests::benchmark_parallel_depth -- --ignored --nocapture
```

It compares one thread with the automatic thread limit, checks equal scores, and reports the median of three timed runs after a warmup. Parallel scheduling and null-window probes can change node counts and tie-breaking between equally valued moves. More threads do not guarantee a fixed depth increase: gains depend on the position, available CPUs, and the time allowance.

To compare the original roughly 30 MB engine with 100 MB, 250 MB, 500 MB, and 1 GB budgets:

```sh
python3 benchmarks/memory/compare.py
```

Custom budget sweeps use the same driver, for example:

```sh
python3 benchmarks/memory/compare.py --time-only --budgets 50 75 100 125 150 200 225 250 --output benchmarks/memory/fine-results.json
python3 benchmarks/memory/compare.py --depth-only --case endgame --depth 13 --budgets 50 75 100 125 150 200 225 250 --output benchmarks/memory/fine-depth13-results.json
```

This dependency-free benchmark compiles the original engine from commit `9288315` alongside the current engine with identical optimization flags and thread limits. It uses fresh processes and cold caches, randomizes tier order, checks fixed-depth score agreement, and records three repetitions of ten-second searches and time to fixed depths. Results include actual allocated cache bytes and peak resident memory where the platform exposes it to Python. JSON measurements and a Markdown comparison are written to `benchmarks/memory/results.*`. Use `--quick` for a smoke run, or `--seconds` and `--repetitions` to change duration. `--ponder --case opening` tests a human-turn search followed by a two-second computer move. `--depth-only --case endgame --depth 13` tests the deeper endgame target. Use `--output` to keep separate results. `--stress-only --repetitions 20` fills and dirties every cache page, then releases and recreates the cache repeatedly to check peak memory, including allocator retention; it does not measure normal search throughput. See [the measured comparison](benchmarks/memory/README.md).

The Rust tests include 152 regression fixture cases: recursive move counts with promotion, castling, and en passant counts; smoke positions; material draws; and terminal positions. They also compare fast and tactical generation with full legality checks over playouts, compare lookup-based evaluation and incremental history with recomputation, and check buffer reuse, concurrent cache coherence and growth, incremental page splits, shared memory ceilings, lazy age wraparound, serial/parallel score agreement and best-move verification, interrupted iterations, CPU headroom policy, incremental hashes, pins, repetition, undo, forced-mate search, and the HTTP handler for both modes. Clock tests cover increments, exact-zero flags, late moves, timeout draws, stopped clocks, undo, search allocation, and cancellation. The handler tests use an in-memory connection and require no socket permissions.

The UI harness checks timed and Unlimited presets, custom controls and thinking times, live clock rendering and resynchronization, move highlighting and requests, board flipping, promotion choice, solo play as Black, and engine diagnostics. Audio checks cover move, check, game-end, timeout, mute, duplicate suppression, silent undo/resume, and unavailable audio support. It uses a simulated DOM and audio context and does not replace visual or audible browser testing. The HTTP test needs permission to bind a local socket; restricted sandboxes may block it.

Source: `src/chess.rs` (rules/hashing), `src/tables.rs` (board geometry), `src/clock.rs` (clocks), `src/engine.rs` (AI/time allocation), `src/cache.rs` (bounded growing cache), `src/ordering.rs` (quiet-move ordering), `src/game.rs` (history/results), `src/main.rs` (local HTTP server), and `web/` (bundled UI).
