import datetime
import hashlib
import shutil
import subprocess
import time
import uuid
from pathlib import Path
from shutil import which

EXECUTABLE = which("CRFS")  # CRFS production-build binary should be installed to path.
# EXECUTABLE = "..."  # Or use an explicit path

TEST_SERVER = "127.0.0.1:8000"
# TEST_SERVER = "192.168.2.4:8000"

SAMPLE_DIR = Path("./sample_files")
SCRATCH_DIR = Path("./outputs")
SCRATCH_DIR.mkdir(parents=True, exist_ok=True)

GLOBAL_CONF = SCRATCH_DIR / "global.json"

USER_ID, USER_NAME = uuid.uuid4(), f"Evaluation User at {datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")}"

DEBUG = 1

RUNS = 5


def clear_scratch_dir() -> None:
    """Remove all files in SCRATCH_DIR to ensure a clean slate."""
    for root, dirs, files in SCRATCH_DIR.walk(top_down=False):
        for f in files:
            (root / f).unlink()
        for d in dirs:
            (root / d).rmdir()


def time_command(options: list[str]) -> float:
    """Time execution of CRFS with given options."""
    assert EXECUTABLE is not None, "CRFS binary not found"
    command: list[str] = [EXECUTABLE] + options

    time_0 = time.perf_counter()
    proc = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    time_1 = time.perf_counter()

    if proc.returncode:
        raise RuntimeError(f"CRFS exited with code {proc.returncode}.\n" + (proc.stdout.decode() if DEBUG else ""))
    elif DEBUG >= 2:
        print(f"Exit code OK. Output:\n{proc.stdout.decode()}")

    return time_1 - time_0


def verify_tree(one: Path, two: Path) -> bool:
    """Check equality of two file trees."""
    IGNORES = [".crfs"]
    # Check all files exist
    for root, _, files in one.walk():
        if list(set(root.parts) & set(IGNORES)):
            continue

        for f in files:
            relative = (root / f).relative_to(one)
            if not (two / relative).exists():
                raise FileNotFoundError(f"{relative} does not exist in {two}")

    for root, _, files in two.walk():
        if list(set(root.parts) & set(IGNORES)):
            continue

        for f in files:
            relative = (root / f).relative_to(two)
            if not (one / relative).exists():
                raise FileNotFoundError(f"{relative} does not exist in {one}")

    # Check equality of files
    for root, _, files in one.walk():
        if list(set(root.parts) & set(IGNORES)):
            continue

        for f in files:
            f1 = root / f
            f2 = two / f1.relative_to(one)

            with f1.open() as file:
                hash1 = hashlib.md5(file.read().encode()).hexdigest()
            with f2.open() as file:
                hash2 = hashlib.md5(file.read().encode()).hexdigest()

            if hash1 != hash2:
                print(f"{hash1} != {hash2} for file {f}")
                return False

    return True


def cp(origin: Path, dest: Path) -> None:
    """Copy all files and directories from `origin` to `dest`."""
    for root, dirs, files in origin.walk():
        for d in dirs:
            (dest / (root / d).relative_to(origin)).mkdir()
        for f in files:
            shutil.copy(root / f, (dest / (root / f).relative_to(origin)))


def setup_pull_time() -> float:
    """Time taken to setup a replica and pull all existing files."""
    clear_scratch_dir()

    SETUP_DIR = (SCRATCH_DIR / "setup")
    TEST_DIR = (SCRATCH_DIR / "test")

    SETUP_DIR.mkdir()
    TEST_DIR.mkdir()

    FS_ID = uuid.uuid4()

    # Setup a new replica so we have a consistent test case
    time_command(["-g", str(GLOBAL_CONF), "init"])
    time_command([
        "-g", str(GLOBAL_CONF),
        "setup",
        "-s", TEST_SERVER,
        "-u", str(USER_ID), "--user-name", USER_NAME,
        "-f", str(FS_ID),
        "-d", str(SETUP_DIR)
    ])

    if DEBUG >= 1:
        print(" -> Setup control.")

    # Copy all sample files to the new replica.
    shutil.copytree((SAMPLE_DIR / "set1"), SETUP_DIR, dirs_exist_ok=True)

    setup_sync = time_command([
        "-g", str(GLOBAL_CONF),
        "sync",
        "-d", str(SETUP_DIR),
    ])

    if DEBUG >= 1:
        print(f" -> Synchronised control in {setup_sync:0.3f}s")

    time_command([
        "-g", str(GLOBAL_CONF),
        "canonize",
        "-d", str(SETUP_DIR),
    ])

    test_setup = time_command([
        "-g", str(GLOBAL_CONF),
        "setup",
        "-s", TEST_SERVER,
        "-u", str(USER_ID), "--user-name", USER_NAME,
        "-f", str(FS_ID),
        "-d", str(TEST_DIR)
    ])

    if DEBUG >= 1:
        print(f" -> Setup test in {test_setup:0.3f}s")

    test_sync = time_command([
        "-g", str(GLOBAL_CONF),
        "sync",
        "-d", str(TEST_DIR),
    ])

    if DEBUG >= 1:
        print(f" -> Synchronised test in {test_sync:0.3f}s")

    if not verify_tree(SETUP_DIR, TEST_DIR):
        raise RuntimeError("Test files do not match control.")

    return test_setup + test_sync


def sync_time() -> float:
    """Time taken to synchronise changes between replicas"""
    clear_scratch_dir()

    DIR1 = (SCRATCH_DIR / "dir1")
    DIR2 = (SCRATCH_DIR / "dir2")

    DIR1.mkdir()
    DIR2.mkdir()

    FS_ID = uuid.uuid4()

    # Setup Replicas with an initial file set
    time_command(["-g", str(GLOBAL_CONF), "init"])
    time_command([
        "-g", str(GLOBAL_CONF),
        "setup",
        "-s", TEST_SERVER,
        "-u", str(USER_ID), "--user-name", USER_NAME,
        "-f", str(FS_ID),
        "-d", str(DIR1)
    ])
    time_command([
        "-g", str(GLOBAL_CONF),
        "setup",
        "-s", TEST_SERVER,
        "-u", str(USER_ID), "--user-name", USER_NAME,
        "-f", str(FS_ID),
        "-d", str(DIR2)
    ])

    # Initialise and synchronise to the same state
    shutil.copytree((SAMPLE_DIR / "set1"), DIR1, dirs_exist_ok=True)

    time_command([
        "-g", str(GLOBAL_CONF),
        "sync",
        "-d", str(DIR1),
    ])
    time_command([
        "-g", str(GLOBAL_CONF),
        "sync",
        "-d", str(DIR2),
    ])
    time_command([
        "-g", str(GLOBAL_CONF),
        "canonize",
        "-d", str(DIR1),
    ])

    assert verify_tree(DIR1, DIR2)
    shutil.copytree((SAMPLE_DIR / "set1-update"), DIR1, dirs_exist_ok=True)

    upload_time = time_command([
        "-g", str(GLOBAL_CONF),
        "sync",
        "-d", str(DIR1),
    ])
    fetch_time = time_command([
        "-g", str(GLOBAL_CONF),
        "sync",
        "-d", str(DIR2),
    ])
    time_command([
        "-g", str(GLOBAL_CONF),
        "canonize",
        "-d", str(DIR1),
    ])

    assert verify_tree(DIR1, DIR2)

    return upload_time + fetch_time


if __name__ == "__main__":
    setup_time_test = [setup_pull_time() for n in range(RUNS)]
    print("Setup Time Test:")
    print(f"  Average {sum(setup_time_test) / RUNS:0.3f}s")
    print(f"  Runs: {", ".join(map(lambda x: f"{x:0.3f}s", setup_time_test))}")

    sync_time_test = [sync_time() for n in range(RUNS)]
    print("Sync Time Test:")
    print(f"  Average {sum(sync_time_test) / RUNS:0.3f}s")
    print(f"  Runs: {", ".join(map(lambda x: f"{x:0.3f}s", sync_time_test))}")
