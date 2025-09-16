#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
SAS-TIMESTAMPS (FAT-safe ordering, free-form PS2 bias, dash-ignoring, slots-per-category config)

Windows-only utility: deterministically set creation/modified/access times so root-level
folders (and everything inside them) are ordered newest→oldest by a stable mapping from
folder name → timestamp. Timestamps are accurate to FAT/VFAT realities.

Key behavior:
- Base time = 12/31/2098 23:59:59 (LOCAL). We subtract a deterministic offset per folder.
- Newest → oldest category blocks in this order:
    APP_*  → APPS → PS1_* → EMU_* → GME_* → DST_* → DBG_* → RAA_* → RTE_* → DEFAULT → SYS_* → ZZY_* → ZZZ_*
- Unprefixed special names are mapped to an **effective**, prefixed name (e.g., BOOT → SYS_BOOT)
  and that **effective** name is used both for:
    (1) category selection,
    (2) within-category alphabetical ordering.
- **Dashes ('-') are ignored for ordering** (removed before lex ordering).
- **FAT-safe mode** (`--fat-safe`): snaps all timestamps to **even seconds** (microseconds=0) and uses **2 s spacing**,
  matching FAT/VFAT mtime precision and preventing copy/rounding drift.
- **PS2 bias** (`--ps2-bias-seconds`): applies an additional signed seconds offset so the PS2 browser's displayed times
  match Windows Explorer display exactly, even if the PS2 reader/RTC applies a nonstandard skew.
- Dry-run (`--dry-run`) writes SAS-TIMESTAMPS-dryrun.tsv (newest→oldest) in the current working directory.

NOTE: Requires Windows (uses SetFileTime via ctypes).
"""

import argparse
import ctypes
import os
import sys
from datetime import datetime, timezone, timedelta

# =========================
# ===== USER CONFIG =======
# =========================
# FAT-safe default spacing: 2 seconds (FAT mtime has 2-second granularity)
SECONDS_BETWEEN_ITEMS = 2

# Big slot budget so each name gets a unique second within its category, even with many items.
# 86_400 seconds ≈ 1 day per category. Nice for viewing in file browser as each category will be a different day.
SLOTS_PER_CATEGORY   = 86_400

# Comma-separated lists of names (no prefixes) to be treated as if they belong to these categories.
# Edit these to add your own folder names (case-insensitive). Whitespace is ignored.
UNPREFIXED_IN_CATEGORY_CSV = {
    "APP_":      "OSDXMB, XEBPLUS",
    "APPS":      "",  # exact "APPS" is its own name
    "PS1_":      "",
    "EMU_":      "",
    "GME_":      "",
    "DST_":      "",
    "DBG_":      "",
    "RAA_":      "RESTART, POWEROFF",
    "RTE_":      "NEUTRINO",
    "SYS_":      "BOOT",
    "ZZY_":      "EXPLOITS",
    "ZZZ_":      "BM, MATRIXTEAM, OPL",
}

# Category order (newest → oldest).
CATEGORY_ORDER = [
    "APP_",
    "APPS",
    "PS1_",
    "EMU_",
    "GME_",
    "DST_",
    "DBG_",
    "RAA_",
    "RTE_",
    "DEFAULT",   # non-matching fallbacks
    "SYS_",
    "ZZY_",
    "ZZZ_",
]

# =========================
# ===== END CONFIG  =======
# =========================

# --- Build quick-lookup from CSV config ---
def _parse_csv(s: str):
    return {x.strip().upper() for x in s.split(",") if x.strip()} if s else set()

UNPREFIXED_MAP = {k: _parse_csv(v) for k, v in UNPREFIXED_IN_CATEGORY_CSV.items()}

# --- Windows FILETIME helpers (ctypes) ---
_EPOCH_AS_FILETIME = 11644473600
_HUNDREDS_OF_NS = 10_000_000

kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)

CreateFileW = kernel32.CreateFileW
CreateFileW.argtypes = [
    ctypes.c_wchar_p,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_void_p,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_void_p
]
CreateFileW.restype = ctypes.c_void_p

SetFileTime = kernel32.SetFileTime
SetFileTime.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
SetFileTime.restype = ctypes.c_int

CloseHandle = kernel32.CloseHandle
CloseHandle.argtypes = [ctypes.c_void_p]
CloseHandle.restype = ctypes.c_int

GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 0x00000001
FILE_SHARE_WRITE = 0x00000002
FILE_SHARE_DELETE = 0x00000004
OPEN_EXISTING = 3
FILE_FLAG_BACKUP_SEMANTICS = 0x02000000  # needed to open directories

class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", ctypes.c_uint32),
                ("dwHighDateTime", ctypes.c_uint32)]

def _dt_to_filetime(dt_utc: datetime) -> FILETIME:
    unix_seconds = dt_utc.timestamp()
    ft = int((unix_seconds + _EPOCH_AS_FILETIME) * _HUNDREDS_OF_NS)
    return FILETIME(ft & 0xFFFFFFFF, ft >> 32)

def _set_times_windows(path: str, dt_utc: datetime) -> None:
    handle = CreateFileW(
        path,
        GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        None,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS,
        None
    )
    if handle == ctypes.c_void_p(-1).value or handle is None:
        raise OSError(f"Failed to open handle for: {path} (WinError {ctypes.get_last_error()})")
    try:
        ft = _dt_to_filetime(dt_utc)
        if not SetFileTime(handle,
                           ctypes.byref(ft),
                           ctypes.byref(ft),
                           ctypes.byref(ft)):
            raise OSError(f"SetFileTime failed for: {path} (WinError {ctypes.get_last_error()})")
    finally:
        CloseHandle(handle)

# --- Category + name → slot mapping ---
CHARSET = tuple(" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_-.")
CHAR_INDEX = {ch: i for i, ch in enumerate(CHARSET)}
BASE = len(CHARSET)

CATEGORY_BLOCK_SECONDS = SLOTS_PER_CATEGORY * SECONDS_BETWEEN_ITEMS
CATEGORY_INDEX = {name: idx for idx, name in enumerate(CATEGORY_ORDER)}

def _effective_category_key(eff: str) -> str:
    if eff.startswith("APP_"): return "APP_"
    if eff == "APPS": return "APPS"
    if eff.startswith("PS1_"): return "PS1_"
    if eff.startswith("EMU_"): return "EMU_"
    if eff.startswith("GME_"): return "GME_"
    if eff.startswith("DST_"): return "DST_"
    if eff.startswith("DBG_"): return "DBG_"
    if eff.startswith("RAA_"): return "RAA_"
    if eff.startswith("RTE_"): return "RTE_"
    if eff.startswith("SYS_") or eff == "SYS": return "SYS_"
    if eff.startswith("ZZY_"): return "ZZY_"
    if eff.startswith("ZZZ_"): return "ZZZ_"
    return "DEFAULT"

def _category_label_for_effective(eff: str) -> str:
    key = _effective_category_key(eff)
    return "DEFAULT" if key == "DEFAULT" else (key if key == "APPS" else f"{key}*")

def _payload_for_effective(eff: str) -> str:
    """Use only the part after the category key, ignoring dashes for ordering."""
    key = _effective_category_key(eff)
    if key == "APPS": return "APPS"
    if key == "DEFAULT": return eff.replace("-", "")
    payload = eff[len(key):] if eff.startswith(key) else eff
    return payload.replace("-", "")

def _lex_fraction(payload: str) -> float:
    """Map payload string to [0,1) preserving lexicographic order (dashes ignored already)."""
    s = payload.upper()
    total = 0.0
    scale = 1.0
    for ch in s[:128]:
        scale *= BASE
        code = CHAR_INDEX.get(ch, BASE - 1)
        total += (code + 1) / scale
    return total

def _normalize_name_for_rules(name: str) -> str:
    """
    Return the EFFECTIVE (possibly prefixed) name for all logic.
    We do not strip dashes here; dashes are ignored later during ordering only.
    """
    n = name.strip().upper()

    # 1) User-configured "no-prefix" names
    for cat_key, names in UNPREFIXED_MAP.items():
        if n in names:
            return "APPS" if cat_key == "APPS" else f"{cat_key}{n}"

    # 2) Built-in defaults
    if n in ("OSDXMB", "XEBPLUS"):
        return "APP_" + n
    if n in ("RESTART", "POWEROFF"):
        return "RAA_" + n
    if n == "NEUTRINO":
        return "RTE_" + n
    if n == "BOOT":
        return "SYS_BOOT"
    if n == "EXPLOITS":
        return "ZZY_EXPLOITS"
    if n in ("BM", "MATRIXTEAM", "OPL"):
        return "ZZZ_" + n

    # 3) Otherwise, leave as-is
    return n

def _category_priority_index(effective: str) -> int:
    key = _effective_category_key(effective)
    return CATEGORY_INDEX[key]

def _slot_index_within_category(effective: str) -> int:
    """
    Compute the within-category slot index using the EFFECTIVE name (e.g., 'SYS_BOOT').
    Dashes are ignored for ordering; underscores are kept.
    """
    payload = _payload_for_effective(effective)
    frac = _lex_fraction(payload)  # [0,1)
    slot = int(frac * SLOTS_PER_CATEGORY)
    if slot >= SLOTS_PER_CATEGORY:
        slot = SLOTS_PER_CATEGORY - 1
    return slot

def _deterministic_offset_seconds(folder_name: str):
    """
    Returns (offset_seconds, cat_idx, slot, effective_name).
    Note: No 0/1-second nudge; rely on 2-second spacing for FAT safety.
    """
    eff = _normalize_name_for_rules(folder_name)
    cat_idx = _category_priority_index(eff)
    slot    = _slot_index_within_category(eff)

    nudge = 0  # removed to avoid FAT rounding collisions

    cat_offset  = cat_idx * CATEGORY_BLOCK_SECONDS
    name_offset = (slot * SECONDS_BETWEEN_ITEMS) + nudge
    return cat_offset + name_offset, cat_idx, slot, eff

# --- Timestamp planner ---
def _base_datetime_local_to_utc() -> datetime:
    """
    Convert the local anchor time to UTC (base reference for deterministic subtraction).
    """
    local_naive = datetime(2098, 12, 31, 23, 59, 59)  # can be 23:59:58 if you want an inherently even anchor
    local_tz = datetime.now().astimezone().tzinfo
    local_aware = local_naive.replace(tzinfo=local_tz)
    return local_aware.astimezone(timezone.utc)

def _planned_timestamp_for_folder(folder_name: str):
    """
    Return a tuple (utc_dt, effective_name, category_label, cat_idx, slot_idx, offset_sec).
    """
    base_utc = _base_datetime_local_to_utc()
    offset_sec, cat_idx, slot_idx, eff = _deterministic_offset_seconds(folder_name)
    ts_utc = datetime.fromtimestamp(base_utc.timestamp() - offset_sec, tz=timezone.utc)
    return ts_utc, eff, _category_label_for_effective(eff), cat_idx, slot_idx, offset_sec

# --- FAT-safe snapping ---
def _snap_even_second(dt: datetime) -> datetime:
    """
    Force timestamp to an even second and zero microseconds.
    """
    dt = dt.replace(microsecond=0)
    if dt.second % 2 == 1:
        dt = dt + timedelta(seconds=1)
    return dt

# --- Walk and set ---
def _set_folder_and_contents_times(root_folder: str, dt_utc: datetime, verbose=False):
    # Recurse and set times, then set root last (so mtime doesn't bump)
    for dirpath, dirnames, filenames in os.walk(root_folder):
        for fname in filenames:
            fpath = os.path.join(dirpath, fname)
            try:
                _set_times_windows(fpath, dt_utc)
                if verbose: print(f"Set file  : {fpath}")
            except Exception as e:
                print(f"[WARN] Could not set times for file {fpath}: {e}", file=sys.stderr)
        for dname in dirnames:
            dpath = os.path.join(dirpath, dname)
            try:
                _set_times_windows(dpath, dt_utc)
                if verbose: print(f"Set dir   : {dpath}")
            except Exception as e:
                print(f"[WARN] Could not set times for dir  {dpath}: {e}", file=sys.stderr)
    try:
        _set_times_windows(root_folder, dt_utc)
        if verbose: print(f"Set ROOT  : {root_folder}")
    except Exception as e:
        print(f"[WARN] Could not set times for ROOT {root_folder}: {e}", file=sys.stderr)

# --- Dry-run writer ---
def _write_dryrun_tsv(plan, base_path: str, verbose=False) -> str:
    """
    plan: list of tuples (name, ts_utc, eff, cat_lbl, cat_idx, slot_idx, offset_sec)
    Writes TSV in CWD (not base_path), sorted newest→oldest by ts_utc.
    """
    cwd = os.getcwd()
    out_path = os.path.join(cwd, "SAS-TIMESTAMPS-dryrun.tsv")
    plan_sorted = sorted(plan, key=lambda x: x[1], reverse=True)
    with open(out_path, "w", encoding="utf-8", newline="") as f:
        f.write("Order\tCategory\tCatIndex\tSlot\tOffsetSec\tName\tEffectiveName\tLocalTime\tUTC\tFullPath\n")
        for idx, (name, ts_utc, eff, cat_lbl, cat_idx, slot_idx, offset_sec) in enumerate(plan_sorted, start=1):
            local_str = ts_utc.astimezone().strftime("%m/%d/%Y %H:%M:%S %Z")
            utc_str = ts_utc.strftime("%Y-%m-%d %H:%M:%S UTC")
            full = os.path.join(base_path, name)
            f.write(f"{idx}\t{cat_lbl}\t{cat_idx}\t{slot_idx}\t{offset_sec}\t{name}\t{eff}\t{local_str}\t{utc_str}\t{full}\n")
    if verbose:
        print(f"[DRY-RUN] Wrote plan to: {out_path}")
        print(f"[DRY-RUN] {len(plan_sorted)} root folders listed (newest → oldest).")
    return out_path

# --- Main ---
def main():
    ap = argparse.ArgumentParser(
        description="Deterministically set ctime/mtime recursively by folder name and category."
    )
    ap.add_argument("path", nargs="?", default=".",
                    help="Top-level directory containing the root folders to timestamp (default: current dir).")
    ap.add_argument("--dry-run", action="store_true",
                    help="Do NOT modify timestamps; output SAS-TIMESTAMPS-dryrun.tsv in the current working directory.")
    ap.add_argument("--verbose", action="store_true", help="Extra logging.")
    ap.add_argument("--fat-safe", action="store_true",
                    help="Snap all times to even seconds (0 µs) to match FAT/VFAT mtime precision.")
    ap.add_argument("--ps2-bias-seconds", type=int, default=0,
                    help="Signed seconds to bias planned timestamps so PS2 display matches Windows. "
                         "Example: -3563 to counter a +59m23s skew on PS2.")

    args = ap.parse_args()

    base_path = os.path.abspath(args.path)
    if not os.path.isdir(base_path):
        print(f"Not a directory: {base_path}", file=sys.stderr)
        sys.exit(1)

    root_folders = [d for d in os.listdir(base_path) if os.path.isdir(os.path.join(base_path, d))]

    if args.verbose:
        print(f"Found {len(root_folders)} root folders under {base_path}")

    plan = []
    for name in root_folders:
        try:
            ts, eff, cat_lbl, cat_idx, slot_idx, offset_sec = _planned_timestamp_for_folder(name)

            # Apply free-form PS2 bias (to counter PS2 skew so displays match)
            if args.ps2_bias_seconds:
                ts = ts + timedelta(seconds=args.ps2_bias_seconds)

            # FAT-safe snapping to even seconds (prevents copy/rounding drift)
            if args.fat_safe:
                ts = _snap_even_second(ts)

        except Exception as e:
            print(f"[WARN] Failed to compute timestamp for {name}: {e}", file=sys.stderr)
            continue
        plan.append((name, ts, eff, cat_lbl, cat_idx, slot_idx, offset_sec))

    if args.dry_run:
        tsv_path = _write_dryrun_tsv(plan, base_path, verbose=args.verbose)
        print(f"Dry-run complete. Plan written to: {tsv_path}")
        return

    for name, ts, eff, cat_lbl, cat_idx, slot_idx, offset_sec in plan:
        full = os.path.join(base_path, name)
        if args.verbose:
            print(f"=== {name} [{cat_lbl}] cat={cat_idx} slot={slot_idx} offset={offset_sec}s -> "
                  f"{ts.astimezone().strftime('%m/%d/%Y %H:%M:%S %Z')} (UTC {ts.strftime('%Y-%m-%d %H:%M:%S')}) ===")
        _set_folder_and_contents_times(full, ts, verbose=args.verbose)

if __name__ == "__main__":
    if os.name != "nt":
        print("This script is intended for Windows (uses SetFileTime).", file=sys.stderr)
        sys.exit(1)
    main()