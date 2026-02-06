#!/usr/bin/env python3
import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

# ---------- helpers ----------
def run(cmd, quiet=False):
    p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if not quiet and p.stderr.strip():
        for line in p.stderr.splitlines():
            if "don't know how to extract resource" in line:
                continue
            print(f"[warn] {line}")
    return p.returncode, p.stdout, p.stderr

def ensure_dir(p: Path):
    p.mkdir(parents=True, exist_ok=True)

def unique_path(dst: Path) -> Path:
    if not dst.exists():
        return dst
    stem, suffix = dst.stem, dst.suffix
    i = 1
    while True:
        alt = dst.with_name(f"{stem}_{i}{suffix}")
        if not alt.exists():
            return alt
        i += 1

LINE_RE = re.compile(
    r"--type=(?P<type>'[^']+'|\d+|[A-Za-z0-9_]+)\s+--name=(?P<name>'[^']+'|\d+)\s+--language=(?P<lang>\d+)"
)

def wrestool_list(gob: Path):
    _, out, _ = run(["wrestool", "-l", str(gob)], quiet=True)
    items = []
    for line in out.splitlines():
        m = LINE_RE.search(line)
        if not m:
            continue
        typ = m.group("type").strip("'")
        name = m.group("name").strip("'")
        lang = m.group("lang")
        items.append({"type": typ, "name": name, "lang": lang})
    return items

# ---------- extraction policy ----------
def wanted_folder_and_ext(typ: str, name: str):
    t = typ.upper()
    if t in ("2", "BITMAP"):
        # keep original name; ensure it ends with .BMP
        fname = name if name.lower().endswith(".bmp") else f"{name}.BMP"
        return "bitmaps", fname
    if t == "WAVE":
        # numeric -> <id>.wav ; string names -> <name>.wav
        base = name
        # strip any extension from resource name
        base = base.split(".")[0]
        return "wav", f"{base}.wav"
    if t == "6":  # STRINGTABLE
        base = name
        return "strings", f"strtbl-{base}.bin"
    if t == "TABLE":
        base = name
        return "tables", f"{base}.bin"
    # fallback
    return f"raw/{typ.lower()}", f"{name}.bin"

def can_normal_decode(typ: str):
    # wrestool can decode BITMAP etc., but not TABLE/WAVE/STRINGTABLE here
    t = typ.upper()
    return t in ("2", "BITMAP")

def must_raw(typ: str):
    t = typ.upper()
    return t in ("WAVE", "TABLE", "6")  # 6=STRINGTABLE

def build_extract_cmd(gob: Path, typ: str, out_dir: Path, name: str | None = None):
    cmd = ["wrestool", "-x"]
    if must_raw(typ) or not can_normal_decode(typ):
        cmd.append("--raw")
    cmd.append(f"--type={typ}")
    if name is not None:
        cmd.append(f"--name={name}")
    cmd.extend(["-o", str(out_dir), str(gob)])
    return cmd

def process_extracted_file(f: Path, item: dict, out_root: Path):
    typ, name = item["type"], item["name"]
    folder, suggested_name = wanted_folder_and_ext(typ, name)
    out_dir = out_root / folder
    ensure_dir(out_dir)

    # Post-process for STRINGTABLE: also create a .txt skim
    if typ == "6":
        # move .bin
        dst_bin = unique_path(out_dir / suggested_name)
        shutil.move(str(f), str(dst_bin))
        # txt skim
        try:
            _, out, _ = run(["strings", "-el", str(dst_bin)], quiet=True)
            (out_dir / "README.txt").write_text(
                "These are raw Win32 STRINGTABLE blocks. The .txt files are a quick UTF-16LE skim.\n",
                encoding="utf-8",
            )
            dst_txt = unique_path(out_dir / (Path(suggested_name).with_suffix(".txt").name))
            dst_txt.write_text(out, encoding="utf-8", errors="ignore")
        except Exception:
            pass
        return

    # WAVE: ensure .wav extension
    if typ.upper() == "WAVE":
        dst = unique_path(out_dir / suggested_name)
        shutil.move(str(f), str(dst))
        return

    # BITMAP (or anything else decoded / raw): enforce target name
    dst = unique_path(out_dir / suggested_name)
    shutil.move(str(f), str(dst))

def extract_one(gob: Path, item: dict, out_root: Path):
    typ, name = item["type"], item["name"]

    # one-resource temp extraction
    with tempfile.TemporaryDirectory() as tmpd:
        tmp = Path(tmpd)
        run(build_extract_cmd(gob, typ, tmp, name=name), quiet=True)

        # find the one file wrestool created
        produced = [p for p in tmp.iterdir() if p.is_file()]
        if not produced:
            # nothing came out; skip politely
            return

        process_extracted_file(produced[0], item, out_root)

def filename_segments(filename: str):
    return set(re.split(r"[_\.]", filename))

def match_extracted_file(name: str, available_files):
    candidates = [f for f in available_files if name in filename_segments(f.name)]
    if len(candidates) == 1:
        return candidates[0]
    return None

def extract_batch(gob: Path, items: list, out_root: Path):
    # Group by type
    by_type = defaultdict(list)
    for it in items:
        by_type[it["type"]].append(it)

    for typ, type_items in by_type.items():
        with tempfile.TemporaryDirectory() as tmpd:
            tmp = Path(tmpd)
            run(build_extract_cmd(gob, typ, tmp), quiet=True)

            available_files = {p for p in tmp.iterdir() if p.is_file()}
            retry_items = []

            for it in type_items:
                matched = match_extracted_file(it["name"], available_files)
                if matched is not None:
                    try:
                        process_extracted_file(matched, it, out_root)
                        available_files.remove(matched)
                    except Exception as e:
                        print(f"    [skip] {it['type']}:{it['name']} ({e})")
                else:
                    retry_items.append(it)

            for it in retry_items:
                try:
                    extract_one(gob, it, out_root)
                except Exception as e:
                    print(f"    [skip] {it['type']}:{it['name']} ({e})")

def copy_fonts(data_dir: Path, out_root: Path):
    font_out = out_root / "fonts"
    ensure_dir(font_out)
    for ttf in data_dir.glob("*.ttf"):
        shutil.copy2(ttf, font_out / ttf.name)
        print(f"font: {ttf.name}")

def main():
    ap = argparse.ArgumentParser(description="Extract Imperialism .gob assets to sane folders.")
    ap.add_argument("data_dir", help="Path to Imperialism/Data")
    ap.add_argument("out_dir", nargs="?", default="assets/extracted", help="Output (default: assets/extracted)")
    args = ap.parse_args()

    data_dir = Path(args.data_dir).expanduser().resolve()
    out_root = Path(args.out_dir).expanduser().resolve()
    ensure_dir(out_root)

    for tool in ("wrestool",):
        if shutil.which(tool) is None:
            print(f"error: required tool '{tool}' not found in PATH", file=sys.stderr)
            sys.exit(1)

    print(f"[*] Scanning .gob in: {data_dir}")
    gobs = sorted(data_dir.glob("*.gob")) + sorted(data_dir.glob("*.GOB"))
    for gob in gobs:
        print(f"  → {gob.name}")
        items = wrestool_list(gob)
        if not items:
            print("    (no listable resources)")
            continue
        extract_batch(gob, items, out_root)

    copy_fonts(data_dir, out_root)
    print(f"[✓] Done → {out_root}")

if __name__ == "__main__":
    main()
