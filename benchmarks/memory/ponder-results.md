# Pondering memory comparison

Human-turn search: 10 seconds; computer move: 2 seconds after the recorded human move. 3 cold repetitions; medians shown.

| Position | Budget MB | Ponder depth | Computer depth | Computer nodes | Cache hits | Cache MB | Peak RSS MB |
|---|---:|---:|---:|---:|---:|---:|---:|
| opening | 30 | 9 | 8 | 7.98002e+06 | 87776 | 28.0 | 32.8 |
| opening | 100 | 9 | 8 | 7.9234e+06 | 89215 | 65.5 | 67.8 |
| opening | 250 | 9 | 7 | 7.76599e+06 | 90197 | 144.7 | 147.2 |
| opening | 500 | 9 | 8 | 7.95809e+06 | 90038 | 144.3 | 146.7 |
| opening | 1000 | 9 | 8 | 7.81561e+06 | 90224 | 143.2 | 145.5 |
