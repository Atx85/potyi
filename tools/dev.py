#!/usr/bin/env python3
"""Build and run Potyi, run regular tests, or run the headless local checks."""
import argparse
import datetime as dt
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest="action", required=True)
    run = actions.add_parser("run", help="Build and open Potyi (the project folder by default)")
    run.add_argument("path", nargs="?", help="File or folder, relative to your current directory")
    run.add_argument("--release", action="store_true", help="Use the optimized build")
    test = actions.add_parser("test", help="Run regular tests without GUI or external servers")
    test.add_argument("filter", nargs="?", help="Optional Rust test-name filter, e.g. recovery")
    check = actions.add_parser("check", help="Run regular tests and isolated headless SDL checks")
    check.add_argument("--output", type=Path, help="Evidence directory (default: target/qa/<unique run>)")
    args = parser.parse_args(argv)

    if args.action == "run":
        target = str(Path(args.path).expanduser().absolute()) if args.path is not None else str(ROOT)
        command = ["cargo", "run", "--locked", "--bin", "potyi"]
        if args.release:
            command.append("--release")
        command.extend(["--", target])
    elif args.action == "test":
        command = ["cargo", "test", "--locked", "--bin", "potyi"]
        if args.filter is not None:
            command.append(args.filter)
        command.extend(["--", "--test-threads=1"])
    else:
        output = args.output.expanduser().absolute() if args.output else (
            ROOT / "target" / "qa" / f"{dt.datetime.now():%Y%m%d-%H%M%S}-{time.time_ns()}"
        )
        print(f"Test evidence: {output}", flush=True)
        command = [sys.executable, str(ROOT / "tools" / "qa" / "run.py"),
                   "--skip-live", "--output", str(output)]

    try:
        # Argument lists preserve spaces and shell characters in paths. Always
        # build from the repository root, even when invoked from elsewhere.
        return subprocess.run(command, cwd=ROOT).returncode
    except FileNotFoundError:
        print(f"Cannot find {command[0]}. Install Rust/Cargo and put it on PATH.", file=sys.stderr)
        return 127
    except OSError as error:
        print(f"Could not start {args.action}: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    sys.exit(main())
