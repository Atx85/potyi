#!/usr/bin/env python3
"""Render reviewed benchmark JSON as a standalone local HTML report."""
import argparse
import html
import json
import math
from pathlib import Path
import statistics

ROOT=Path(__file__).resolve().parents[2]
LABELS={
    "open_1gb":"Open 1 GB (lazy piece table)", "first_40_lines_1gb":"Read first 40 lines of 1 GB", "open_and_first_40_lines_1gb":"Open 1 GB and read first 40 lines (combined)",
    "open_1mib":"Open 1 MiB (lazy piece table)","first_40_lines_1mib":"Read first 40 lines of 1 MiB", "literal_scan_1mib":"Full literal search, 1 MiB, no match", "1000_insert_delete_pairs_1mib":"1,000 insert/delete pairs, 1 MiB",
    "open_100mib":"Open 100 MiB (lazy piece table)","first_40_lines_100mib":"Read first 40 lines of 100 MiB", "literal_scan_100mib":"Full literal search, 100 MiB, no match", "1000_insert_delete_pairs_100mib":"1,000 insert/delete pairs, 100 MiB",
    "terminal_printf_first_frame":"Terminal printf: first rendered output", "terminal_grep_first_frame":"Terminal grep: first rendered output", "terminal_pipe_first_frame":"Terminal pipeline: first rendered output", "terminal_120_cached_frames":"Render 120 unchanged terminal frames", "terminal_append_layout_2mb":"Append/layout 2,060,800 bytes in 128 chunks"}
E=html.escape

def render(report):
    if report.get("schema_version")!=1 or set(report["metrics"])!=set(LABELS): raise ValueError("Unsupported or incomplete report")
    count=report["sample_count"]
    if count<3 or report["warmups"]<1: raise ValueError("Not enough samples/warmups")
    for metric in report["metrics"].values():
        samples=metric["samples_ms"]
        if len(samples)!=count or any(not math.isfinite(n) or n<0 for n in samples): raise ValueError("Invalid samples")
        for key,expected in [("median_ms",statistics.median(samples)),("min_ms",min(samples)),("max_ms",max(samples))]:
            if not math.isclose(metric[key],expected): raise ValueError("Summary differs from raw samples")
    env=report["environment"]
    date=report["measured_at"][:10]
    target=next(line.removeprefix("host: ") for line in env["rustc"].splitlines() if line.startswith("host: "))
    architecture_note = "This run uses an Intel (x86_64) build on an arm64 host; it does not measure the Apple Silicon build." if target.startswith("x86_64") and "arm64" in env["os"] else ""
    context=f'{date} · {env["cpu"]} · {env["os"]} · Rust target {target}'
    fmt=lambda n:f'{n:.3f}'
    rows='\n'.join(f'<tr><th scope="row">{E(label)}</th><td>{fmt(report["metrics"][name]["median_ms"])}</td><td>{fmt(report["metrics"][name]["min_ms"])}</td><td>{fmt(report["metrics"][name]["max_ms"])}</td></tr>' for name,label in LABELS.items())
    idle=report["idle_samples"]
    idle_driver=report.get("idle_video_driver","dummy")
    idle_section='<p>Idle memory and CPU were not measured in this run.</p>'
    if idle:
        rss=statistics.median(r["rss_mib"] for r in idle)
        cpu=statistics.median(r["cpu_percent_one_core"] for r in idle)
        idle_section=f'<p>Median resident memory: <strong>{rss:.1f} MiB</strong>. Median CPU: <strong>{cpu:.2f}% of one core</strong> across {len(idle)} idle samples, each approximately {statistics.median(r["seconds"] for r in idle):.1f} seconds after a three-second settling period. The actual release app runs with a small file, {E(idle_driver)} SDL video, and LSP disabled.</p><p>RSS is sampled at each interval’s endpoints, not a peak-memory measurement. CPU time is quantized by the operating system; a zero reading means below the measurement resolution. Desktop display drivers, language servers, and workloads can change both figures.</p>'
    limitations=''.join(f'<li>{E(item)}</li>' for item in report["limitations"])
    page=f'''<!doctype html>
<html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Pötyi Text Editor Benchmarks — Method, Timing &amp; Memory</title>
<meta name="description" content="Reproducible Pötyi (Potyi) editor benchmarks: large-file opening, search, editing, terminal latency and idle resources, with raw data and methodology.">
<style>
body {{ max-width: 960px; margin: 3rem auto; padding: 0 1.5rem; font: 16px/1.6 system-ui, sans-serif; color: #202321; }}
h1, h2 {{ line-height: 1.2; }} h2 {{ margin-top: 2rem; }}
a {{ color: #165f36; }} pre, .benchmark-table {{ overflow-x: auto; }}
pre {{ padding: 1rem; background: #f3f5f2; }} table {{ border-collapse: collapse; width: 100%; }}
th, td {{ text-align: left; padding: .65rem; border-bottom: 1px solid #ccc; }}
</style>
</head><body>
<main id="main" class="benchmark-report"><p class="eyebrow">Reproducible local measurements</p><h1>Pötyi editor benchmarks</h1>
<p>These results measure specific workloads in Pötyi’s current source tree. They are not a comparison with another editor or a promise for every machine.</p>
<p class="benchmark-context">{E(context)} {E(architecture_note)}</p>
<h2>Timing results</h2><p>All values are milliseconds. Each workload has {count} recorded samples after {report["warmups"]} discarded warm-up run(s). Build time, fixture generation, and font setup are outside the timed regions.</p>
<div class="benchmark-table" tabindex="0" role="region" aria-label="Timing results; scroll horizontally on small screens"><table><caption>Optimized release build · headless SDL</caption><thead><tr><th scope="col">Workload</th><th scope="col">Median (ms)</th><th scope="col">Minimum (ms)</th><th scope="col">Maximum (ms)</th></tr></thead><tbody>{rows}</tbody></table></div>
<h2>Idle memory and CPU</h2>{idle_section}
<h2>How to reproduce</h2><p>From the source checkout on macOS or Linux, with Rust, Python 3, the normal SDL build dependencies, a POSIX shell and grep installed:</p>
<pre><code>python3 tools/benchmarks/run.py
python3 tools/benchmarks/publish.py temp/benchmarks/latest.json</code></pre>
<p>The runner creates real, non-sparse 1 MiB, 100 MiB, and 1 GB (1,000,000,000 bytes) text fixtures. Their contents are already warm in the filesystem cache. The editor probe times opening a lazy piece table and reading its first 40 lines. For 1 GB it also records the combined open-and-read time; it does not read the whole gigabyte. The smaller fixtures additionally measure a full-file search for a missing literal and inserting/deleting one byte at the middle 1,000 times. Rendering, highlighting, and undo bookkeeping are outside these core editing timings. Temporary fixtures are removed after each probe.</p>
<p>Terminal probes use an 800×600 SDL dummy window. They measure submission to the first rendered output for <code>printf terminal-ready</code>, <code>grep -n 'fn main' src/main.rs</code>, and <code>printf 'ready\\n' | grep -n ready</code>. The grep input is this source snapshot’s main.rs. A separate probe redraws unchanged terminal content 120 times and appends/layouts a long line in 128 chunks. Those forced redraws do not model idle behavior. Terminal pixel rendering is included; presentation to a physical display is not.</p>
<h2>Build and provenance</h2><ul><li>Profile: {E(env["profile"])}; shell: {E(env["shell"])}; SDL driver: {E(env["sdl_video_driver"])}.</li><li>Logical CPUs: {env["logical_cpus"]}.</li><li>Base commit: <code>{E(report["git_commit"])}</code>. Uncommitted changes: {str(report["working_tree_dirty"]).lower()}.</li><li>Rust source fingerprint: <code>{E(report["rust_source_sha256"])}</code>.</li></ul>
<pre><code>{E(env["rustc"])}</code></pre><p>The source fingerprint covers Rust files under src plus Cargo.toml and Cargo.lock. The base commit alone does not reproduce a dirty working tree; use the matching source fingerprint when comparing runs.</p>
<h2>What these numbers do not establish</h2><ul>{limitations}</ul>
<p><a href="https://github.com/Atx85/potyi/tree/main/tools/benchmarks">Benchmark source and recorded measurements</a></p>
</main><footer><div class="footer-inner"><p>© 2026 Attila Banko</p><p>Free software under the GPLv3.</p></div></footer></body></html>'''
    return page


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report",type=Path)
    parser.add_argument("--output", type=Path, default=ROOT/"temp/benchmarks/report.html", help="Local HTML output; does not update the website")
    parser.add_argument("--check", action="store_true", help="Verify an existing local report without writing")
    args=parser.parse_args()
    report=json.loads(args.report.read_text())
    content=render(report)+"\n"
    if args.check:
        if not args.output.exists() or args.output.read_text()!=content:
            raise ValueError(f"Generated content differs: {args.output}")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(content)
    print(f"Local benchmark report {'verified' if args.check else 'written'}: {args.output}")

if __name__=="__main__": main()
