"""The scan inventory's memory limit, measured on this system (CI runs it on Windows, macOS and
Linux).

The new laptop's reader counts what each folder and item will take once read and refuses a list
past pctwin_record::MAX_HELD. This builds the test `inventory_memory` in release and runs it once
per shape, each in its own process so each peak is its own. Most shapes are hostile lists that
must be refused at the limit; the `accept-` shapes send a list just within it and then approve,
plan or save it, so the whole path after receiving is measured too. For each process it measures
the peak: peak commit on Windows, the largest memory footprint on macOS (what the system itself
counts against the process, compressed memory included), the largest resident size on Linux. It
fails if any shape passes MAX_HELD plus ALLOWANCE: the test process's own fixed running room (the
program, its runtime, one page's bytes of at most 8 MB and the pieces in flight). The "baseline"
run, which reads nothing, shows how much of the allowance the running room itself takes. Then it
runs the pinned capacity: 680,000 files eight folders deep move, 700,000 do not.
"""

import json
import os
import re
import subprocess
import sys
import threading

SHAPES = [
    "baseline", "rootwin", "rootunix", "wide4000", "deep1", "cheapest", "folders", "owners",
    "sharedwin", "roottext", "accept-approve", "accept-plan", "accept-plan-text",
    "accept-persist",
]
ALLOWANCE = 64 * 1024 * 1024
TEST = "the_new_laptop_holds_no_more_than_its_count"
CAPACITY = "an_honest_laptop_of_680000_files_moves_and_one_of_700000_does_not"


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


def peak_linux(proc):
    _, status, usage = os.wait4(proc.pid, 0)
    proc.returncode = os.waitstatus_to_exitcode(status)
    return proc.returncode, usage.ru_maxrss * 1024, "largest resident size"


# rusage_info_v4 (sys/resource.h): 16 bytes of UUID, then 64-bit fields. Field 8 is the current
# memory footprint, field 29 the largest it has been in the process's life.
FOOTPRINT, LIFETIME_MAX_FOOTPRINT = 1 + 8, 1 + 29


def peak_macos(proc, done):
    import ctypes

    libproc = ctypes.CDLL("/usr/lib/libproc.dylib")
    libproc.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]

    def footprint():
        info = (ctypes.c_uint64 * 64)()
        if libproc.proc_pid_rusage(proc.pid, 4, ctypes.byref(info)) != 0:
            return None
        now, most = info[FOOTPRINT], info[LIFETIME_MAX_FOOTPRINT]
        # The largest is updated by the system now and then, so the current one can be ahead of
        # it for a moment: the larger of the two counts. Both must be of a believable size, or
        # the fields are not where they are expected.
        if not (0 < now < 1 << 36 and most < 1 << 36):
            raise RuntimeError(f"unexpected memory counters: footprint {now}, largest {most}")
        return max(now, most)

    # Read while it runs (the lifetime largest only grows) and once its work is done: the test
    # then waits, still running, until its input is closed, so nothing is missed.
    peak = 0
    while True:
        finished = done.wait(0.02)
        seen = footprint()
        if seen is not None:
            peak = max(peak, seen)
        if finished or proc.poll() is not None:
            break
    proc.stdin.close()
    proc.wait()
    if peak == 0:
        sys.exit("could not read the process's memory footprint")
    return proc.returncode, peak, "largest footprint"


def run(exe, test, shape):
    env = dict(os.environ, PCTWIN_MEMORY_SHAPE=shape)
    macos = sys.platform == "darwin"
    if macos:
        env["PCTWIN_MEMORY_HOLD"] = "1"
    proc = subprocess.Popen(
        [exe, "--ignored", "--exact", test, "--nocapture", "--test-threads=1"],
        env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        text=True,
    )
    lines = []
    done = threading.Event()

    def read():
        for line in proc.stdout:
            lines.append(line)
            if line.strip().endswith("memory: done"):
                done.set()

    reader = threading.Thread(target=read, daemon=True)
    reader.start()
    try:
        if os.name == "nt":
            proc.stdin.close()
            measured = peak_windows(proc)
        elif macos:
            measured = peak_macos(proc, done)
        else:
            proc.stdin.close()
            measured = peak_linux(proc)
    finally:
        # Whatever went wrong here, the test process is never left waiting.
        if proc.poll() is None:
            proc.kill()
    reader.join()
    return measured, "".join(lines)


def main():
    exe = build()
    worst = 0
    failed = []
    max_held = None
    for shape in SHAPES:
        (code, peak, what), output = run(exe, TEST, shape)
        found = re.search(r"max_held (\d+)", output)
        if not found:
            sys.exit(f"{shape}: the test did not run:\n{output}")
        max_held = int(found.group(1))
        limit = max_held + ALLOWANCE
        ok = code == 0 and peak <= limit
        print(f"{shape:>14}: {what}: {peak / 1e6:8.1f} MB, limit {limit / 1e6:.1f} MB"
              f" ({'ok' if ok else 'FAILED'})", flush=True)
        if code != 0:
            print(output)
        if not ok:
            failed.append(shape)
        if shape != "baseline":
            worst = max(worst, peak)
    print(f"worst {worst / 1e6:.1f} MB; MAX_HELD {max_held / 1e6:.1f} MB;"
          f" allowance {ALLOWANCE / 1e6:.1f} MB")
    (code, _, _), output = run(exe, CAPACITY, "capacity")
    print(f"capacity (680,000 files move, 700,000 do not): {'ok' if code == 0 else 'FAILED'}")
    if code != 0:
        print(output)
        failed.append("capacity")
    if failed:
        sys.exit(f"over the limit or failed: {', '.join(failed)}")


main()
