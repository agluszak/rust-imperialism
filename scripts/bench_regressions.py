#!/usr/bin/env python3
import json
import math
import os
import sys
from pathlib import Path

THRESHOLD = float(os.environ.get("BENCH_REGRESSION_THRESHOLD", "0.05"))
CRITERION_DIR = Path(os.environ.get("CRITERION_DIR", "target/criterion"))


def bench_name_from(path: Path) -> str:
    rel = path.relative_to(CRITERION_DIR)
    parts = rel.parts
    if len(parts) < 3:
        return rel.as_posix()
    # Remove trailing change/estimates.json
    return "/".join(parts[:-2])


def main() -> int:
    if not CRITERION_DIR.exists():
        print("::notice::No Criterion data found; skipping regression check.")
        return 0

    change_files = list(CRITERION_DIR.glob("**/change/estimates.json"))
    if not change_files:
        print("::notice::No benchmark change data found (missing baseline?).")
        return 0

    regressions = []
    for path in change_files:
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue

        mean = data.get("mean", {}).get("point_estimate")
        if mean is None or not isinstance(mean, (int, float)):
            continue
        if not math.isfinite(mean):
            continue

        if mean > THRESHOLD:
            regressions.append((bench_name_from(path), mean))

    if not regressions:
        print("::notice::No benchmark regressions beyond threshold.")
        return 0

    regressions.sort(key=lambda item: item[1], reverse=True)
    for name, mean in regressions:
        pct = mean * 100.0
        threshold_pct = THRESHOLD * 100.0
        print(
            "::warning title=Benchmark regression::"
            f"{name} regressed by {pct:.1f}% (mean change). "
            f"Threshold {threshold_pct:.1f}%."
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
