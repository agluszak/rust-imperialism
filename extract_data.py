#!/usr/bin/env python3
import argparse, os, re, shutil, subprocess, sys, tempfile
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
    rc, out, _ = run(["wrestool", "-l", str(gob)], quiet=True)
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

def extract_one(gob: Path, item: dict, out_root: Path):
    typ, name, lang = item["type"], item["name"], item["lang"]
    folder, suggested_name = wanted_folder_and_ext(typ, name)
    out_dir = out_root / folder
    ensure_dir(out_dir)

    # one-resource temp extraction
    with tempfile.TemporaryDirectory() as tmpd:
        tmp = Path(tmpd)
        # Choose raw vs normal
        if must_raw(typ) or not can_normal_decode(typ):
            cmd = ["wrestool", "-x", "--raw", f"--type={typ}", f"--name={name}", "-o", str(tmp), str(gob)]
        else:
            cmd = ["wrestool", "-x", f"--type={typ}", f"--name={name}", "-o", str(tmp), str(gob)]
        rc, _, _ = run(cmd, quiet=True)

        # find the one file wrestool created
        produced = [p for p in tmp.iterdir() if p.is_file()]
        if not produced:
            # nothing came out; skip politely
            return

        f = produced[0]

        # Post-process for STRINGTABLE: also create a .txt skim
        if typ == "6":
            # move .bin
            dst_bin = unique_path(out_dir / suggested_name)
            shutil.move(str(f), str(dst_bin))
            # txt skim
            try:
                rc, out, _ = run(["strings", "-el", str(dst_bin)], quiet=True)
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
        for it in items:
            try:
                extract_one(gob, it, out_root)
            except Exception as e:
                print(f"    [skip] {it['type']}:{it['name']} ({e})")

    copy_fonts(data_dir, out_root)
    print(f"[✓] Done → {out_root}")

if __name__ == "__main__":
    main()
