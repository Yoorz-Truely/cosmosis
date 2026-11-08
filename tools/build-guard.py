#!/usr/bin/env python3
"""Keep builds from overheating or saturating this Mac, across every agent and worktree.

Every compile of Sau, Null and Cosmosis goes through this script: Blender's C and C++
compiles through CMake's compiler launcher (`just build` sets it), and
Rust compiles through cargo's `build.rustc-wrapper` (each repository's `.cargo/config.toml`).
Each compile takes one of a few machine-wide slots: lock files that the system frees when
the compiler exits or is killed. The number of slots shrinks when macOS reports thermal
pressure or when the whole machine is overloaded, whatever project is causing it, and
compiles run at "utility" priority so the user's own apps stay responsive.
Docs: docs/agents/parallel.md ("Machine limits"). Null and Cosmosis keep identical copies
in their tools/ folders; `just check` in Sau fails when a copy differs.

  build-guard.py <compiler or rustc> <args...>   wrapper mode, called by CMake or cargo
  build-guard.py status                          thermal state, load, slots in use
  build-guard.py wait                            block while thermal pressure is heavy

Settings (environment): SAU_BUILD_SLOTS (default 4), SAU_BUILD_QOS (utility, background
or default; default utility), SAU_BUILD_GUARD=0 to bypass for one command.
"""

import ctypes
import fcntl
import os
import pathlib
import subprocess
import sys
import time

DIR = pathlib.Path.home() / "Library" / "Caches" / "sau-build-guard"
LEVELS = ["nominal", "moderate", "heavy", "trapping", "sleeping"]
HEAVY = 2


def thermal_level():
    """macOS thermal pressure: 0 nominal, 1 moderate, 2 heavy, 3 trapping, 4 sleeping."""
    try:
        lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib")
        token, state = ctypes.c_int(), ctypes.c_uint64()
        if lib.notify_register_check(b"com.apple.system.thermalpressurelevel", ctypes.byref(token)) != 0:
            return 0
        lib.notify_get_state(token, ctypes.byref(state))
        lib.notify_cancel(token)
        return int(state.value)
    except OSError:
        return 0


def base_slots():
    return max(1, int(os.environ.get("SAU_BUILD_SLOTS", "4")))


def slot_limit():
    """How many compiles may run at once across the machine right now, and why."""
    slots = base_slots()
    level = thermal_level()
    # Slower builds rather than a hotter chip: back off early (the user's rule in Crescend).
    cap = {0: slots, 1: max(1, slots // 2)}.get(level, 2)
    load, cores = os.getloadavg()[0], os.cpu_count() or 8
    if load > cores:
        cap = min(cap, 3)  # the whole machine is saturated, whoever is busy
    return cap, level, load, cores


def log(text):
    try:
        with open(DIR / "guard.log", "a") as f:
            f.write(f"{time.strftime('%Y-%m-%d %H:%M:%S')} {os.getpid()} {text}\n")
    except OSError:
        pass


def acquire():
    """Block until a slot is free; return its open file, which holds the lock until closed."""
    DIR.mkdir(parents=True, exist_ok=True)
    start, checked, cap, level, load = time.monotonic(), 0.0, 1, 0, 0.0
    while True:
        now = time.monotonic()
        if now - checked >= 2:
            cap, level, load, _ = slot_limit()
            checked = now
        for i in range(cap):
            f = open(DIR / f"slot-{i}.lock", "w")
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError:
                f.close()
                continue
            waited = time.monotonic() - start
            if waited > 5:
                log(f"waited {waited:.0f}s for slot {i} of {cap} (thermal {LEVELS[min(level, 4)]}, load {load:.1f})")
            return f
        time.sleep(0.3)


def gated(args):
    """Only real compiles take a slot; version probes and links pass straight through."""
    if os.environ.get("SAU_BUILD_GUARD", "1") == "0":
        return False
    if "--crate-name" in args:  # rustc from cargo; "___" marks cargo's target-info probe
        return "___" not in args
    return "-c" in args  # a C, C++ or Objective-C compile


def wrap(cmd):
    if gated(cmd[1:]):
        lock = acquire()
        os.set_inheritable(lock.fileno(), True)  # the compiler keeps the slot until it exits
        qos = os.environ.get("SAU_BUILD_QOS", "utility")
        if qos in ("utility", "background"):
            cmd = ["/usr/sbin/taskpolicy", "-c", qos, *cmd]
    os.execvp(cmd[0], cmd)


def wait():
    start = time.monotonic()
    # At most 15 minutes, so a long hot spell slows work down without stalling it.
    while thermal_level() >= HEAVY and time.monotonic() - start < 900:
        if time.monotonic() - start < 1:
            print("build-guard: thermal pressure is heavy; waiting for the Mac to cool", file=sys.stderr)
        time.sleep(5)
    if time.monotonic() - start > 1:
        log(f"waited {time.monotonic() - start:.0f}s for heavy thermal pressure to clear")
    return 0


def held_slots(limit=64):
    held = []
    for i in range(limit):
        p = DIR / f"slot-{i}.lock"
        if not p.exists():
            continue
        with open(p, "w") as f:
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError:
                held.append(i)
    return held


def count(name):
    return len(subprocess.run(["pgrep", "-x", name], capture_output=True, text=True).stdout.split())


def status():
    cap, level, load, cores = slot_limit()
    print(f"thermal pressure   {LEVELS[min(level, 4)]} ({level})")
    print(f"load (1 min)       {load:.1f} on {cores} cores")
    print(f"compile slots      {len(held_slots())} in use of {cap} allowed now (base {base_slots()})")
    print(f"compilers running  {count('clang') + count('clang++')} clang, {count('rustc')} rustc, machine-wide")
    lines = (DIR / "guard.log").read_text().splitlines()[-3:] if (DIR / "guard.log").exists() else []
    for line in lines:
        print(f"  recent wait      {line}")
    return 0


if __name__ == "__main__":
    a = sys.argv[1:]
    if a == ["status"]:
        sys.exit(status())
    if a == ["wait"]:
        sys.exit(wait())
    if not a or a[0].startswith("-"):
        print(__doc__)
        sys.exit(2)
    wrap(a)
