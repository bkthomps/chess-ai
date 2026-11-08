# Tactical aspiration follow-up: profile

Baseline is PVS at `98d2f0d`; archived experimental aspiration uses ±64 centipawns. The application uses PVS without aspiration. Fresh processes, cold caches, 100 MB total budget. Timing stages discard one warm-up group; diagnostic counters need no warm-up. Variant names refer to the archived experiment. No production source changes.

Diagnostic counters only; timings from these instrumented builds are not performance evidence.

| Stage | Variant | Nodes | Negamax calls | Quiescence calls | PVS probes | PVS re-searches | Depth-zero calls | Null-window negamax | Positive-depth null-window | Null-window quiescence |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| serial-depth8 | aspiration | 7,477,148 | 2,887,462 | 4,589,686 | 1,657 | 18 | 1,777,060 | 2,887,390 | 1,110,356 | 4,589,474 |
| serial-depth8 | margin32 | 12,557,968 | 4,649,524 | 7,908,444 | 2,230 | 15 | 2,602,360 | 4,649,420 | 2,047,088 | 7,908,237 |
| serial-depth8 | baseline | 7,477,203 | 2,887,466 | 4,589,737 | 1,657 | 22 | 1,777,064 | 2,887,390 | 1,110,356 | 4,589,473 |
