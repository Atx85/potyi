# Final terminal performance assessment

The final build and evidence identities match, all six paired measurements completed, and accepted process samples stay entirely inside their recorded phases. The tested implementation has no new correctness blocker from this review. **It does not match original `:term` active performance, and a flat memory plateau has not been demonstrated.**

Measurements use one frozen x86_64 release test binary under Rosetta on ARM macOS, with a hidden 800×600 SDL dummy-video fixture. Each of three alternating old/new pairs receives a 64 KiB command, three paced 12 MiB output commands (36 MiB total), nine resizes, and clipboard operations. This is a matched local comparison, not a visible native-window benchmark.

| Measure (median of three runs) | Original | New |
| --- | ---: | ---: |
| Idle app CPU, percentage of one core | 0.52% | 0.52% |
| CPU for first / second / third 12 MiB output, app seconds | 0.59 / 0.57 / 0.59 | 1.13 / 1.19 / 1.30 |
| Output command wall time, approximately | 2.41–2.42 s | 2.61–2.62 s |
| Nine resizes, app CPU / wall time | 2.17 s / 2.31 s | 2.26 s / 2.40 s |
| Small command wall time | 53.31 ms | 197.86 ms |
| Matched 4 KiB selected copy | 0.01 ms | 1.47 ms |
| Copy All wall time / retained plaintext | 10.65 ms / 8.00 MiB | 45.83 ms / 4.56 MiB |

New streaming app CPU remains 1.9–2.2 times the original; wall time is about 8% higher. Resize cost is about 4% higher. Selected copy remains below 2 ms. Copy All is slower while copying less retained text, because the new 8 MiB file budget includes record and index metadata. It cannot be treated as an equal-byte throughput comparison.

The new small-command range is 173.62–2908.28 ms; the original range is 45.83–59.51 ms. The 2.908 s first-run delay consumed little app CPU. Existing process traces localize waiting to early helper startup, but do not prove its cause. Warm command latency also remains higher. These are unresolved performance costs; the fixture's separate helper launch does not establish the same delay in the visible running app.

| Combined app + simultaneous observed child RSS, MiB | Original | New |
| --- | ---: | ---: |
| Idle | 14.28 | 19.23 |
| Settled after first 12 MiB | 38.73 | 44.44 |
| Settled after second 12 MiB | 44.50 | 51.18 |
| Settled after third 12 MiB | 46.48 | 52.88 |
| During resize | 82.52 | 58.97 |
| Settled after Copy All | 109.52 | 78.21 |

The new app alone settles at 40.37 → 47.06 → 48.78 MiB after the three output commands, plus roughly 4.1 MiB for its persistent shell. Its settled output memory is **higher**, while resize and clipboard phases use less combined RSS. From the second to third settled phase, every new run still grows by 1.37–2.75 MiB; original growth is 1.05–1.98 MiB. These samples support continued bounded eviction, not a flat RSS plateau. RSS sums may count shared pages more than once.

Idle counters are near the 0.01-second process-counter resolution. Settled phases show no ongoing redraw loop, but equality at 0.52% is approximate. External process observation takes roughly 40–60 ms per snapshot and adds scheduling noise; child CPU can miss short-lived processes. Operation events preserve latency even when a phase is too fast for a process sample. Three pairs cannot establish results for all hardware or OSs.

Final correctness evidence on the same source includes 54/54 exact RGB frames, 20/20 exact clipboard artifacts, and an additional real accent-only filename click proof with an exact frame and clipboard. Native macOS ARM backend tests pass (184 plus two VT tests); the macOS x86_64 app tests pass under Rosetta (209 plus one wait regression); Linux ARM64 container tests pass (184 backend, two VT, 209 app). Windows compilation passes. Native Windows, native Intel macOS, native Linux x86_64, and fish runtime remain external validation gates.

The before-fix 24 MiB baseline and intermediate 36 MiB measurements are preserved separately, with their own build identities. Final conclusions above use only the final 36 MiB old/new pairs. The tested visuals use the bundled font and do not establish every font, DPI, theme or native compositor.

Final source: `a45c4bde2b9fb155d2660973e01270edc6b122c6c339810b877629269c8cd851`  
Release test binary: `465241e743302373c9622a4d2149a33a307f27d00af630432a502c1adad99904`  
Release helper: `e60d238d02dd5757c0378c8aac9ccc7da0da7a26b6865c4e042cf2fc8070d4f4`  
Results: `291023752d04654c3bddf216aec7ed4d6d76558e38fa1ea2017713cd8a63dea2`

Detailed medians, ranges, simultaneous child memory, per-run growth, observer timings and verified identities are in [resource-final-assessment.json](resource-final-assessment.json).
