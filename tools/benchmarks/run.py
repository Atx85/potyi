#!/usr/bin/env python3
"""Build optimized probes, repeat them, and retain raw samples plus provenance."""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PROBES = ["benchmarks::editor_performance_probe", "renderer::terminal_selection_render_tests::terminal_command_latency_probe", "renderer::terminal_selection_render_tests::terminal_performance_probe"]


def output(args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def cpu_seconds(value):
    days, clock = value.split("-", 1) if "-" in value else ("0", value)
    parts = list(map(float, clock.split(":")))
    return int(days)*86400 + sum(part*60**i for i, part in enumerate(reversed(parts)))


def idle_samples(binary, env, count, seconds, driver):
    env = dict(env)
    if driver == "native": env.pop("SDL_VIDEODRIVER",None)
    else: env["SDL_VIDEODRIVER"] = driver
    rows = []
    with tempfile.TemporaryDirectory(prefix="potyi-idle-benchmark-") as folder:
        fixture = Path(folder)/"sample.txt"
        fixture.write_text("A small document for idle measurement.\n"*100)
        for index in range(count):
            with tempfile.TemporaryFile(mode="w+") as log:
                process = subprocess.Popen([str(binary),str(fixture)],cwd=folder,env=env,stdout=log,stderr=log)
                try:
                    time.sleep(3)  # Settle font loading and the initial render.
                    if process.poll() is not None:
                        log.seek(0)
                        raise RuntimeError("Idle editor exited: " + log.read()[-2000:])
                    def sample():
                        fields = output(["ps","-p",str(process.pid),"-o","rss=","-o","time="]).split()
                        if len(fields) != 2: raise RuntimeError("Unable to sample editor process")
                        return int(fields[0]),cpu_seconds(fields[1])
                    start_rss,start_cpu = sample()
                    start = time.monotonic()
                    time.sleep(seconds)
                    end_rss,end_cpu = sample()
                    elapsed = time.monotonic()-start
                    rows.append({"rss_mib":max(start_rss,end_rss)/1024,"cpu_percent_one_core":max(0,end_cpu-start_cpu)/elapsed*100,"seconds":elapsed})
                    print(f"Idle sample {index+1}/{count}",flush=True)
                finally:
                    if process.poll() is None: process.terminate()
                    try: process.wait(timeout=5)
                    except subprocess.TimeoutExpired: process.kill(); process.wait()
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples",type=int,default=7)
    parser.add_argument("--warmups",type=int,default=1)
    parser.add_argument("--idle-samples",type=int,default=3)
    parser.add_argument("--idle-seconds",type=float,default=5)
    parser.add_argument("--idle-driver",choices=["native","dummy"],default="native",help="Idle app display driver; native opens temporary app windows")
    parser.add_argument("--output",type=Path,default=ROOT/"temp/benchmarks/latest.json")
    args = parser.parse_args()
    if args.samples < 3 or args.warmups < 1 or args.idle_samples < 0 or args.idle_seconds < 1:
        parser.error("Use >=3 samples, >=1 warmup, >=0 idle samples and >=1 idle second")
    if os.name != "posix": parser.error("This runner currently needs macOS/Linux (POSIX shell and ps)")
    shell = os.environ.get("SHELL") or "/bin/sh"
    env = {**os.environ,"SDL_VIDEODRIVER":"dummy","SHELL":shell,"POTYI_BENCHMARK_JSON":"1"}
    print("Building optimized probes…",flush=True)
    result = subprocess.run(["cargo","test","--release","--locked","--no-run","--message-format=json"],cwd=ROOT,env=env,text=True,stdout=subprocess.PIPE,check=True)
    executables = [m["executable"] for line in result.stdout.splitlines() if line.startswith("{")
                   for m in [json.loads(line)] if m.get("reason")=="compiler-artifact" and m.get("profile",{}).get("test") and m.get("executable") and m["target"]["name"]=="potyi"]
    if len(executables)!=1: raise RuntimeError("Expected one editor test executable")
    measurements = {}
    logs = []
    for iteration in range(args.warmups+args.samples):
        current = {}
        for probe in PROBES:
            result = subprocess.run([executables[0],probe,"--exact","--ignored","--nocapture","--test-threads=1"],cwd=ROOT,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=120,check=True)
            logs.append(result.stdout)
            for line in result.stdout.splitlines():
                if line.startswith("POTYI_BENCH "):
                    item=json.loads(line.removeprefix("POTYI_BENCH "))
                    if item["name"] in current: raise RuntimeError("Duplicate benchmark sample")
                    current[item["name"]]=item
        if len(current)!=16: raise RuntimeError(f"Expected 16 metrics, got {len(current)}")
        if iteration>=args.warmups:
            for name,item in current.items(): measurements.setdefault(name,{"bytes":item["bytes"],"samples_ms":[]})["samples_ms"].append(item["milliseconds"])
        print(f"Probe run {iteration+1}/{args.warmups+args.samples}",flush=True)
    for item in measurements.values():
        samples=sorted(item["samples_ms"])
        item.update(median_ms=statistics.median(samples),min_ms=min(samples),max_ms=max(samples))
    idle=[]
    if args.idle_samples:
        subprocess.run(["cargo","build","--release","--locked"],cwd=ROOT,env=env,check=True)
        idle=idle_samples(ROOT/"target/release/potyi",env,args.idle_samples,args.idle_seconds,args.idle_driver)
    files = sorted([*ROOT.glob("src/**/*.rs"),ROOT/"Cargo.toml",ROOT/"Cargo.lock"])
    digest=hashlib.sha256()
    for path in files: digest.update(str(path.relative_to(ROOT)).encode()); digest.update(b"\0"); digest.update(path.read_bytes())
    cpu=platform.processor()
    if platform.system()=="Darwin": cpu=output(["sysctl","-n","machdep.cpu.brand_string"])
    report={"schema_version":1,"measured_at":dt.datetime.now(dt.timezone.utc).isoformat(),"git_commit":output(["git","rev-parse","HEAD"]),
        "working_tree_dirty":bool(output(["git","status","--porcelain"])),"rust_source_sha256":digest.hexdigest(),
        "environment":{"os":platform.platform(),"cpu":cpu,"logical_cpus":os.cpu_count(),"rustc":output(["rustc","-vV"]),"shell":shell,"profile":"release","sdl_video_driver":"dummy","window":"800x600 for terminal probes"},
        "warmups":args.warmups,"sample_count":args.samples,"metrics":measurements,"idle_samples":idle,"idle_video_driver":args.idle_driver,
        "limitations":["Local measurements, not a comparison with other editors.","Fixtures are freshly written and warm in the OS cache; no cold-disk claim.","Open measures the lazy piece table only; first 40 lines measures text access, not a rendered editor window.","Terminal frames use the headless SDL driver, not desktop GPU presentation.","Idle samples use the actual release app, the reported idle display driver, a small file, and LSP disabled; RSS is resident memory at sample endpoints, not a peak or allocation budget.","Idle CPU uses ps cumulative CPU time, is quantized by the OS, and is expressed as percent of one core. Zero means below measurement resolution.","Full application startup, LSP servers, large-file idle memory and other platforms are not measured."]}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+"\n")
    args.output.with_suffix(".log").write_text("\n".join(logs))
    print(f"Saved {args.output}",flush=True)


if __name__=="__main__": main()
