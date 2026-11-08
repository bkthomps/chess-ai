# Parallel root-phase diagnostics

Temporary per-phase instrumentation, three fresh-process repetitions. Timings are diagnostic; use the uninstrumented comparison for speed claims. CPU/elapsed is the average CPU-core use over the process search, not a claim of exclusive CPU reservation.

| Position | Variant | CPU ms | Elapsed ms | CPU / elapsed | Root-phase ms | First-move serial ms | Serial-first fraction | Maximum helpers |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| opening | baseline | 3815.28 | 1557.95 | 2.30 | 1557.21 | 996.01 | 65.2% | 5 |
| opening | balanced | 4487.86 | 1125.17 | 3.97 | 1124.19 | 72.29 | 6.4% | 5 |
| middlegame | baseline | 1755.02 | 922.86 | 1.90 | 901.30 | 609.46 | 67.9% | 5 |
| middlegame | balanced | 2149.73 | 604.43 | 3.56 | 585.38 | 103.84 | 17.9% | 5 |
| endgame | baseline | 8462.51 | 2862.65 | 3.16 | 2861.88 | 1235.77 | 45.0% | 5 |
| endgame | balanced | 8589.38 | 2429.23 | 3.60 | 2428.41 | 585.20 | 24.1% | 5 |
| tactics | baseline | 1273.70 | 285.43 | 4.44 | 281.58 | 63.94 | 22.8% | 5 |
| tactics | balanced | 1248.29 | 261.78 | 4.83 | 257.98 | 32.86 | 12.3% | 5 |
| pawns | baseline | 280.23 | 135.90 | 2.00 | 135.35 | 89.88 | 66.4% | 5 |
| pawns | balanced | 313.67 | 86.93 | 3.54 | 86.43 | 16.29 | 18.7% | 5 |
