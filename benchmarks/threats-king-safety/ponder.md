# Measured results

Fresh processes, each with a 500 ms pondering warmup and measured 2 s reply. Medians and ranges; scores intentionally may differ between evaluators.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | ponder / 500 | safety | 8–8 | -40–-40 | e7e5 | 2000.01 (2000.01–2000.02) | 15.66 | 17.40 |
| opening | 1 | ponder / 500 | baseline | 8–8 | -49–-49 | d7d5 | 2000.01 (2000–2000.02) | 17.37 | 19.14 |
| opening | 6 | ponder / 500 | baseline | 9–9 | -10–-10 | d7d5 | 2000.07 (2000.06–2000.14) | 63.69 | 66.14 |
| opening | 6 | ponder / 500 | safety | 8–8 | -40–-40 | e7e5 | 2000.06 (2000.05–2000.11) | 52.11 | 54.62 |
