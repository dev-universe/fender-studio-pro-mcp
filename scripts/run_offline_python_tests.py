"""Run the offline unittest suite in separate processes for CI."""

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import os
from pathlib import Path
import subprocess
import sys
import unittest


def test_ids(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from test_ids(test)
        else:
            yield test.id()


def run_shard(ids, env):
    return subprocess.run(
        [sys.executable, "-m", "unittest", *ids],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("workers", type=int)
    args = parser.parse_args()
    if args.workers < 1:
        parser.error("workers must be positive")

    scripts = Path(__file__).resolve().parent
    loader = unittest.TestLoader()
    suite = loader.discover(str(scripts), pattern="test_*.py")
    if loader.errors:
        print("\n".join(loader.errors), file=sys.stderr)
        return 1

    ids = list(test_ids(suite))
    if not ids or len(ids) != len(set(ids)):
        print("Expected a nonempty suite with unique test IDs", file=sys.stderr)
        return 1

    workers = min(args.workers, len(ids))
    env = os.environ.copy()
    env["PYTHONPATH"] = os.pathsep.join(filter(None, [str(scripts), env.get("PYTHONPATH")]))
    print(f"Running {len(ids)} Python tests across {workers} processes", flush=True)

    failed = False
    with ThreadPoolExecutor(max_workers=workers) as pool:
        shards = {
            pool.submit(run_shard, ids[index::workers], env): index + 1
            for index in range(workers)
        }
        for future in as_completed(shards):
            result = future.result()
            print(f"Python shard {shards[future]}/{workers} exited {result.returncode}")
            print(result.stdout, end="", flush=True)
            failed |= result.returncode != 0
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
