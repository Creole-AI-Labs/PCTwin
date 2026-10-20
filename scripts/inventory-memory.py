"""The scan inventory's memory limit, measured on this system (CI runs it on Windows, macOS and
Linux).

The new laptop's reader counts what each folder and item will take once read and refuses a list
past pctwin_record::MAX_HELD. This builds the test `inventory_memory` in release, runs it once
per hostile shape (each in its own process, so each peak is its own), measures the process's
peak (peak commit on Windows, largest resident size on macOS and Linux), and fails if any shape
passes MAX_HELD plus ALLOWANCE: the test process's own fixed running room (the program, its
runtime, one page's bytes of at most 8 MB and the pieces in flight). The "baseline" run, which
reads nothing, shows how much of the allowance the running room itself takes.
"""

import json
import os
import re
import subprocess
import sys

SHAPES = ["baseline", "rootwin", "rootunix", "wide4000", "deep1", "cheapest", "folders", "owners"]
ALLOWANCE = 64 * 1024 * 1024
TEST = "the_new_laptop_holds_no_more_than_its_count"


def build():
    out = subprocess.run(
        ["cargo", "test", "--release", "--locked", "-p", "pctwin-transfer",
         "--test", "inventory_memory", "--no-run", "--message-format=json"],
        check=True, capture_output=True, text=True,
    ).stdout
    for line in out.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg.get("executable") \
                and msg["target"]["name"] == "inventory_memory":
            return msg["executable"]
    sys.exit("the inventory_memory test was not built")


def peak_windows(proc):
    import ctypes
    from ctypes import wintypes

    class Counters(ctypes.Structure):
        _fields_ = [
            ("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD),
            ("PeakWorkingSetSize", ctypes.c_size_t), ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t), ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t), ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    proc.wait()
    # The process's own handle stays open after it ends, so its lifetime peak can still be read.
    c = Counters()
    c.cb = ctypes.sizeof(c)
    get = ctypes.WinDLL("kernel32").K32GetProcessMemoryInfo
    get.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
    if not get(int(proc._handle), ctypes.byref(c), c.cb):
        sys.exit("could not read the process's memory")
    return proc.returncode, c.PeakPagefileUsage, "peak commit"


def peak_unix(proc):
    _, status, usage = os.wait4(proc.pid, 0)
    proc.returncode = os.waitstatus_to_exitcode(status)
    scale = 1 if sys.platform == "darwin" else 1024
    return proc.returncode, usage.ru_maxrss * scale, "largest resident size"


def main():
    exe = build()
    worst = 0
    failed = []
    for shape in SHAPES:
        env = dict(os.environ, PCTWIN_MEMORY_SHAPE=shape)
        proc = subprocess.Popen(
            [exe, "--ignored", "--exact", TEST, "--nocapture", "--test-threads=1"],
            env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
        output = proc.stdout.read()
        code, peak, what = peak_windows(proc) if os.name == "nt" else peak_unix(proc)
        found = re.search(r"max_held (\d+)", output)
        if not found:
            sys.exit(f"{shape}: the test did not run:\n{output}")
        max_held = int(found.group(1))
        limit = max_held + ALLOWANCE
        ok = code == 0 and peak <= limit
        print(f"{shape:>9}: {what} {peak / 1e6:8.1f} MB, limit {limit / 1e6:.1f} MB"
              f" ({'ok' if ok else 'FAILED'})", flush=True)
        if code != 0:
            print(output)
        if not ok:
            failed.append(shape)
        if shape != "baseline":
            worst = max(worst, peak)
    print(f"worst {worst / 1e6:.1f} MB; MAX_HELD {max_held / 1e6:.1f} MB;"
          f" allowance {ALLOWANCE / 1e6:.1f} MB")
    if failed:
        sys.exit(f"over the limit or failed: {', '.join(failed)}")


main()
