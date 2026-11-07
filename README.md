# chess-ai

A dependency-free Rust chess program with a local browser interface. Choose one-player mode against the computer or two-player mode on the same device. The compact menu contains mode, color, and time controls, with cream and sage colors and continuously falling chess pieces in the background. The animation pauses during gameplay and is hidden when reduced motion is enabled. The game screen contains the board, clocks for timed games, turn status, and basic controls. A right-hand sidebar shows move history and an expandable Inside the engine panel with the thinking allowance, depth, nodes searched, cache hits, and search time. On small screens, the sidebar moves below the board. SVG pieces, legal-move hints, and check indicators keep the board easy to read.

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

Rust handles every rule and move decision. The computer uses iterative-deepening negamax, a form of minimax, with alpha-beta pruning, capture and cached-move ordering, quiescence search, and material/positional evaluation. Incremental 64-bit Zobrist hashing accounts for pieces, side to move, castling rights, and en-passant targets. The bounded transposition table stores depth, bound type, score, and best move; its identity also includes the reversible-history context and halfmove clock. Repetition keys ignore en-passant targets that cannot legally be captured.

Search runs on a worker thread and keeps the last completed depth under a time budget. Timed games allocate approximately remaining time / 36 in the opening, rising toward remaining time / 18 as material is exchanged, plus 75% of the increment. The allocation reserves up to 250ms for scheduling and move submission and caps each search at 15% of usable remaining time. This is a time-management heuristic, not a claim of optimal play. Unlimited games use the selected thinking time for each computer move instead. Both modes keep deepening without a per-move depth target. A forced move, proven mate, or finished game can end the search early. New games, undo, resignation, and flags cancel pending searches and discard stale results.

## Verify

```sh
cargo test --offline
cargo clippy --all-targets --offline -- -D warnings
cargo build --release --offline
python3 tests/http_smoke.py
node tests/ui_smoke.cjs
```

The Rust tests include 152 regression fixture cases: recursive move counts with promotion, castling, and en passant counts; smoke positions; material draws; and terminal positions. They also check incremental hashes, pins, repetition, undo, forced-mate search, and the HTTP handler for both modes. Clock tests cover increments, exact-zero flags, late moves, timeout draws, stopped clocks, undo, search allocation, and cancellation. The handler tests use an in-memory connection and require no socket permissions.

The UI harness checks timed and Unlimited presets, custom controls and thinking times, live clock rendering and resynchronization, move highlighting and requests, board flipping, promotion choice, solo play as Black, and engine diagnostics. Audio checks cover move, check, game-end, timeout, mute, duplicate suppression, silent undo/resume, and unavailable audio support. It uses a simulated DOM and audio context and does not replace visual or audible browser testing. The HTTP test needs permission to bind a local socket; restricted sandboxes may block it.

Source: `src/chess.rs` (rules/hashing), `src/clock.rs` (clocks), `src/engine.rs` (AI/time allocation), `src/game.rs` (history/results), `src/main.rs` (local HTTP server), and `web/` (bundled UI).
