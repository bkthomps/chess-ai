# chess-ai

A dependency-free Rust chess program with a local browser interface. Choose one-player mode against the computer or two-player mode for same-device play or AI vs. AI. The compact menu contains mode, color, and time controls, with cream and sage colors and continuously falling chess pieces in the background. The animation pauses during gameplay and is hidden when reduced motion is enabled. The game screen contains the board, clocks for timed games, turn status, and basic controls. A right-hand sidebar shows move history and an expandable Inside the engine panel with live thinking and pondering status, white-relative evaluation, elapsed search time and allowance, search threads, cache usage, completed depth, nodes searched, and cache hits. On small screens, the sidebar moves below the board. SVG pieces, legal-move hints, and check indicators keep the board easy to read.

## Run

Download the repository ZIP from GitHub and **extract it first**. Open the extracted folder:

- **macOS:** double-click **mac_launcher.command**. If macOS blocks it as an internet download, use the system's Open/Privacy & Security controls to allow the launcher you downloaded.
- **Windows:** double-click **windows_launcher.cmd**. Keep the entire extracted folder together.
- **Linux:** run `sh linux_launcher.sh` in the extracted folder. You can also run **linux_launcher.sh** as a program in file managers that support shell scripts; you may need to mark it executable first.

If Make is already available, simply run:

```sh
make
```

`make it` also works. Make is optional and is not required by the double-click launchers. The launchers work from folders containing spaces.

First launch installs missing Rust and native compiler tools, builds the optimized application, starts the local server, and opens your default browser. JavaScript runs in the browser; Node.js, npm, Python, and Git are not needed to play. Installation requires internet access and may ask for an administrator password or system approval. Windows C++ tools can take several minutes and several GB of disk space. If an installer requests a restart, restart and run the launcher again. On macOS, complete Apple's Command Line Tools dialog and rerun the launcher. Later launches reuse the tools and build; the application itself works offline.

Automatic setup supports macOS, Windows desktop with PowerShell 5.1 or newer, and Linux with apt, dnf, pacman, zypper, or apk. Other Linux distributions need a C compiler/linker, curl, CA certificates, and xdg-utils installed through their own package manager. A desktop browser must already be installed. Managed devices may require your administrator to allow tool installation. ZIP extraction may lose executable permissions; on macOS you can run `sh scripts/launch.sh` from Terminal if the double-click launcher cannot execute.

Keep the terminal open while playing; Ctrl+C stops the server. If browser opening fails, open the address printed in the terminal (normally **http://127.0.0.1:7878**). If that port is busy, use:

```sh
# macOS / Linux
sh scripts/launch.sh --port 8080
```

On Windows, run `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/launch.ps1 --port 8080` from the extracted folder.

For manual builds with Rust 1.70 or newer and native build tools already installed:

```sh
cargo run --release --offline -- --open
cargo run --release --offline -- --open --port 8080
```

The installers use [official Rust downloads](https://rust-lang.org/tools/install/) and [Microsoft C++ Build Tools](https://learn.microsoft.com/en-us/windows/dev-environment/rust/setup). Existing compatible Rust toolchains are reused. Older rustup-managed toolchains use a current stable toolchain for the launch; other older Rust installations need a manual update.

There are no crate dependencies, external assets, accounts, or network services. The server binds only to the local machine. Keep the terminal running while playing; Ctrl+C stops it. Tabs share one game. Refreshing returns to the setup menu; starting a game replaces the current game. Restarting the server resets it.

If the browser cannot reach the server, it shows a connection message over the falling-piece background and checks for reconnection automatically. When the server responds, the browser clears its game state and returns to the main menu.

## Play

- Select one or two players, and choose your color in one-player mode. Under Two players, Mode offers Same computer (default) or AI vs. AI. AI vs. AI plays both sides automatically, searching only for the side to move, with no pondering. The board is spectator-only; live engine diagnostics show the current search.
- Choose 1 min, 2 min + 1 sec increment, 5 min (default), or 10 min + 5 sec increment. Custom accepts 0.1–180 base minutes and a 0–600 second increment. Both players receive the same initial time and increment.
- Unlimited turns off both clocks. In one-player mode, choose a thinking time of 1–5 seconds per move (3 seconds by default), or Custom for any positive number of seconds, including fractions. AI vs. AI uses the same thinking-time setting for both sides. Same-computer Unlimited has no thinking-time setting.
- Drag a piece to a highlighted destination, or click a piece and then its destination. Mouse, pen, and touch dragging are supported. Drop outside the board or on an illegal square to cancel the drag. White moves first.
- In one-player mode, move your pieces while the computer thinks to queue premoves in order. The board previews your queued moves, so you can move the same pawn or piece again from its projected square. Purple hints show conditional destinations; purple squares and the numbered list identify the queue. Choose promotion pieces when queuing. After each computer reply, only the next premove plays, and only if it is legal. If it cannot play, the entire remaining queue is cancelled.
- Selecting or dragging from a piece's original square after earlier premoves moved it away clears the queue and starts a new line from the actual position. Selecting a piece in the preview appends to the existing line. Escape, right-click on the board, or Cancel premoves clears the whole queue.
- Use arrow keys to navigate the board, Enter or Space to select, and Escape to clear a selection.
- Castle by moving the king two squares. Choose queen, rook, bishop, or knight when promoting.
- Undo removes one move in local play or returns to your previous decision in solo play, restoring both clocks to that position. Flip board changes the view. New game asks for confirmation during an unfinished game, then opens the setup menu. Finished games return to the menu immediately.
- Checkmate, stalemate, insufficient material, threefold repetition, and the fifty-move rule end the game. Repetition and fifty-move draws are applied automatically, without a separate claim step.
- Resign game opens a confirmation before conceding. Resign is disabled in AI vs. AI; Undo takes back one move and restarts the AI for that turn.
- Moves make a short click, check plays a two-note alert, and a finished game plays a three-note chime. The menu plays an upbeat tune and the disconnect screen plays a slower, melancholy tune; music starts after a click or keypress if the browser blocks autoplay. A previously saved mute setting is respected. Audio is synthesized locally; no audio files or downloads are needed. Undoing and flipping the board are silent.

In timed games, White's clock starts when Start game is pressed. Only the player to move loses time; increment is added after an accepted move. Clocks keep running while the menu is open or the browser is inactive. Rust uses a monotonic server clock, and checks flags independently of browser polling. A late move cannot gain an increment to escape a timeout. Clocks stop when the game ends. Unlimited games never run a clock or award a timeout, including after undo.

Premoves are held in the current browser tab. The next queued move is submitted on the next board update after a computer reply (normally polled every 150ms during computer thinking). The preview includes only your queued moves; future computer replies remain unknown, and every premove is checked against the server's legal moves on its actual turn. Keep the tab visible for automatic play; normal clock time applies while waiting for that update and submission. Opening the menu, undoing, resigning, starting another game, or refreshing clears the queue. Flipping the board preserves it. Moves or game changes from another tab can invalidate it.

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

For iterations at depth six and beyond, helpers can also share replies within the leading branch, up to two plies below the root. Each phase completes and joins its helpers before the parent phase starts, reusing the same thread buffers and CPU limit. Small phases start only as many helpers as there are remaining moves. Subtree cache scores use their actual ply to preserve mate distances, and terminal/draw handling and repetition history follow the serial search. This reduces the time helpers wait for the first move without adding selective pruning or changing the single-thread search. See [the parallel load-balancing measurements](benchmarks/parallel/README.md).

The automatic thread limit uses Rust's portable available-parallelism query and reserves roughly a quarter of logical CPUs, rounding the reservation up and always keeping at least one search thread. For example, two logical CPUs use one search thread, four use three, and eight use six. Search is capped at 64 threads. The default 100 MB budget further limits search to at most 18 threads, preserving at least 32 MiB for cache storage on machines with many CPUs. This leaves CPU capacity for the browser and other applications; it does not pin or exclusively reserve particular cores. On a machine reporting only one logical CPU, search runs on one thread and takes a short pause after roughly every nine milliseconds of computation, aiming to leave about 10% CPU time available. The same policy applies to thinking and pondering on macOS, Windows, and Linux.

The Inside the engine panel shows the current search, including during pondering. Evaluation is the last completed depth’s score in pawns, positive for White and negative for Black; mate scores are shown separately. Depth reached is the last completed depth. Nodes, cache hits, and elapsed time update during an unfinished iteration and reset for each new position. Search threads shows the configured search-thread limit. Cache used / limit shows the storage currently allocated and its ceiling in decimal MB; unused budget is not preallocated. Node and hit counts combine the coordinator and helpers, with helper counters batched to avoid synchronization at every node. Pondering displays “Your turn” instead of a time allowance. The worker checks publication time every 1024 nodes and publishes a small snapshot about every 150ms, plus initial and final snapshots, without waiting on readers or locking the game state. The browser polls every 150ms during thinking and pondering, including Unlimited games; statistics-only updates preserve the board and selection. Finished searches retain their final snapshot until another search or a reset.

Timed games allocate approximately remaining time / 36 in the opening, rising toward remaining time / 18 as material is exchanged, plus 75% of the increment. The allocation reserves up to 250ms for scheduling and move submission and caps each search at 15% of usable remaining time. This is a time-management heuristic, not a claim of optimal play. Unlimited games use the selected thinking time for each computer move instead; pondering time is additional. Both modes keep deepening without a per-move depth target. A forced computer move, proven mate, or finished game can end the search early. New games, undo, resignation, and flags cancel pending searches and discard stale results. New games and successful undos also clear the search cache. The worker releases the cache allocation when the game ends or switches to Same computer, and recreates it when AI search resumes. Opening the menu during an ongoing solo game keeps pondering and its cache. Same computer never searches or ponders. AI vs. AI shares one cache and worker between both sides, starting each search only after the previous turn finishes; it never ponders.

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

The UI harness checks timed and Unlimited presets, custom controls and thinking times, live clock rendering and resynchronization, move highlighting and requests, clicking and mouse/touch dragging, illegal drops and drag cancellation, board flipping, promotion choice, solo play as Black, and engine diagnostics. Premove checks cover ordered click/drag queues, repeated moves by the same piece, projected positions, restarting from vacated squares, conditional captures and castling, underpromotion and follow-up moves, cancellation at an illegal middle move, captured pieces, clocks, game results, stale histories, immediate computer replies, and duplicate or rejected submissions. Audio checks cover menu and disconnect music, screen transitions, move, check, game-end, timeout, duplicate suppression, silent undo, and unavailable audio support. It uses a simulated DOM and audio context and does not replace visual or audible browser testing. The HTTP test needs permission to bind a local socket; restricted sandboxes may block it.

Source: `src/chess.rs` (rules/hashing), `src/tables.rs` (board geometry), `src/clock.rs` (clocks), `src/engine.rs` (AI/time allocation), `src/cache.rs` (bounded growing cache), `src/ordering.rs` (quiet-move ordering), `src/game.rs` (history/results), `src/main.rs` (local HTTP server), and `web/` (bundled UI).
