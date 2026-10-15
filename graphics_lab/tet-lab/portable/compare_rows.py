#!/usr/bin/env python3
"""Compare a portable driver's CSV against the recorded gpu/ result.

    python3 portable/compare_rows.py cuda      # every spiral, backend cuda
    python3 portable/compare_rows.py rocm

Deterministic columns (names, sizes, reps, upload bytes, correctness metrics)
must match the recorded rows exactly on cuda. On other backends correctness
columns may differ in the last digits (vendor libm), so they are reported, not
required. Timing columns are never required to match: the report gives the
median ratio portable/recorded per column, which on cuda is the
KernelAbstractions launch overhead and on rocm is the AMD-vs-RTX-5060 ratio.
"""
import statistics
import sys
from pathlib import Path

RESULTS = Path(__file__).resolve().parent.parent / "results"

# stem -> (key columns, exact-on-cuda columns, timing columns); by header name
SPIRALS = {
    "spiral-e-gpu-confirm": None,  # every data row is deterministic
    "spiral-f-frame-budget": (
        ["kind", "name", "h", "tets", "verts", "family"],
        ["reps", "ident_max", "dev_px95_cpugpu"],
        ["us_wall_deform", "us_dev_deform", "us_wall_recon", "us_dev_recon", "us_wall_frame", "us_dev_frame"],
    ),
    "spiral-h-fusion": (
        ["kind", "name", "h", "tets", "verts", "family"],
        ["reps", "ident_max", "bfuse_max"],
        ["us_A_split", "us_B_fused", "us_C_graphsplit", "us_D_graphfused", "us_E_nosync", "us_A_dev", "us_B_dev"],
    ),
    "spiral-h2-wind-ladder": (
        ["kind", "family", "name", "h", "tets", "verts", "N"],
        ["ntot", "reps", "ident_max"],
        ["wall_us", "dev_us"],
    ),
    "spiral-i-comparators": (
        ["kind", "path", "name", "verts", "tets", "N"],
        ["bytes_up_frame"],
        ["us_wall", "us_dev"],
    ),
}


def rows_by_kind(path):
    """{kind: (header, [rows])} using each '# ... rows: a,b,c' comment as header."""
    headers, out = {}, {}
    for line in path.read_text().splitlines():
        if line.startswith("#") and "rows:" in line:
            cols = line.split("rows:", 1)[1].strip().split(" (")[0].split(",")
            headers[cols[0] if cols[0] != "kind" else None] = cols
            continue
        if not line or line.startswith("#"):
            continue
        f = line.split(",")
        hdr = next((h for h in headers.values() if len(h) == len(f)), None)
        out.setdefault(f[0], (hdr, []))[1].append(f)
    return out


def compare(stem, backend):
    rec = RESULTS / f"{stem}.csv"
    port = RESULTS / f"{stem}-portable-{backend}.csv"
    if not port.exists():
        return f"{stem}: MISSING {port.name}", False
    spec = SPIRALS[stem]
    if spec is None:
        a = [l for l in rec.read_text().splitlines() if l and not l.startswith("#")]
        b = [l for l in port.read_text().splitlines() if l and not l.startswith("#")]
        same = a == b
        diff = sum(x != y for x, y in zip(a, b)) + abs(len(a) - len(b))
        ok = same or backend != "cuda"
        return f"{stem}: {len(a)} rows, {'all identical' if same else f'{diff} differ'}", ok
    keys, exact, timing = spec
    R, P = rows_by_kind(rec), rows_by_kind(port)
    lines, ok = [], True
    for kind, (hdr, rrows) in R.items():
        if hdr is None or not set(keys) <= set(hdr):
            continue  # derived rows (e.g. cap) follow from timings
        ix = {c: hdr.index(c) for c in hdr}
        key = lambda r: tuple(r[ix[c]] for c in keys)
        prow = {key(r): r for r in P.get(kind, (hdr, []))[1]}
        mism, missing, ratios = [], 0, {c: [] for c in timing if c in ix}
        for r in rrows:
            p = prow.get(key(r))
            if p is None:
                missing += 1
                continue
            for c in exact:
                if c in ix and r[ix[c]] != p[ix[c]]:
                    mism.append(f"{'/'.join(key(r)[1:])} {c}: {r[ix[c]]} -> {p[ix[c]]}")
            for c in ratios:
                try:
                    ratios[c].append(float(p[ix[c]]) / float(r[ix[c]]))
                except (ValueError, ZeroDivisionError):
                    pass
        strict = backend == "cuda"
        ok &= missing == 0 and (not mism or not strict)
        lines.append(f"{stem} [{kind}]: {len(rrows)} rows, {missing} missing, "
                     f"{len(mism)} deterministic-column differences{'' if strict else ' (reported only)'}")
        lines += [f"    {m}" for m in mism[:8]]
        rs = ", ".join(f"{c} x{statistics.median(v):.2f}" for c, v in ratios.items() if v)
        if rs:
            lines.append(f"    timing median ratio portable/recorded: {rs}")
    return "\n".join(lines), ok


if __name__ == "__main__":
    backend = sys.argv[1] if len(sys.argv) > 1 else "cuda"
    all_ok = True
    for stem in SPIRALS:
        text, ok = compare(stem, backend)
        all_ok &= ok
        print(("OK   " if ok else "FAIL ") + text)
    print("ROWS: ALL DETERMINISTIC COLUMNS MATCH" if all_ok else "ROWS: MISMATCH")
    sys.exit(0 if all_ok else 1)
