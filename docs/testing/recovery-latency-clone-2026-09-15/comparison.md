# Recovery snapshot improvement

An independent filesystem clone replaces the full original-file copy where macOS/Linux support it. The journal format, commit synchronization, lazy file opening and bounded memory design are unchanged. Windows and unsupported/cross-volume filesystems retain the original streaming path and its first-edit delay.

| File | Before, first edit | After, first edit |
| --- | ---: | ---: |
| Empty | 27.35 ms | 28.75 ms |
| 1 MiB | 34.65 ms | 31.91 ms |
| 100 MiB | 78.94 ms | 27.58 ms |
| 1 GiB | 475.95 ms | 46.49 ms |

Medians from three sessions per size/mode in each run, on the same Mac with an x86_64 release build under Rosetta. The 1 GiB first edit improved about **10×** (after range **36–47 ms**). These are edit-handler timings with warm source data, not screen latency or a Windows/Linux benchmark. Native clone execution and source-write isolation were confirmed on this Mac.

Later insertions still synchronize recovery: median **9.33 ms**, worst **31.13 ms**; paced typing median **10.35 ms**. Changes in later timings are measurement variation, not a journal optimization. For 1 GiB, recovery added **0.33 MiB** to median peak RSS in the isolated test process.

Validation: **543 regular tests passed**, including **22 recovery-related tests**, and all **24 performance cases passed**. The updated release executable built successfully. Coverage includes process termination, undo/redo, Save As, source changes after snapshotting, source-path replacement before snapshotting, native cloning, forced copying, short sources, empty originals, read-only permissions and existing-destination protection. Linux native cloning and Windows fallback were not run on their respective operating systems locally; the existing release workflow runs the recovery tests on each platform.

[Before measurements](../recovery-latency-2026-09-15/summary.md) · [After measurements](summary.md) · [Raw after samples](results.json) · [Recovery tests](recovery-tests.log) · [Regular tests](regular-tests.log) · [Release build](release-build.log)
