"""The figures a Measure timing table is read for, above the table itself.

A performance pull request registers its rows in these terms: the median head /
base time over the samples of Revit 2023 to 2027, the largest files, any file
past a bar, and the peak memory of the largest files. This reads Measure's
merged `timing.md` (file | base ms | head ms | ratio | base peak MiB | head peak
MiB | ratio) and prints them, each ratio taken from the medians in the table
rather than from its rounded ratio column:

- the median, lowest and highest head / base time and peak for each group: the
  models, the samples of Revit 2023 to 2027, and the samples of 2015 to 2022;
- the three files whose base takes longest, and the three whose base peak is
  highest, with both ratios;
- every file more than 3% slower, or more than 2% higher in peak, than the
  base, the bars most rows use (a row with another bar reads the table);
- every file whose peak rounds on either side differ by more than 2%, with its
  rounds: such a file's peak takes one of two values from round to round on
  the same code (Snowdon Towers HVAC and Plumbing, #1857), so its median is
  either. It counts as higher only where every head round is more than 2%
  above every base round;
- any row whose figures do not read as numbers.

The rounds are the table's last four columns (base and head time, base and
head peak in KiB, each space-separated); a table without them is read as before.

A sample is a row named `YYYY-<lowercase stem>`; every other row is a model. It
judges no registered row.

Usage: python3 -I timing_verdict.py <timing.md>
"""
import re
import statistics
import sys

SLOWER = 1.03
HIGHER = 1.02


def rows(path):
    good, bad = [], []
    for line in open(path, errors="replace").read().splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 7 or cells[0] in ("file", "") or set(cells[0]) <= set("-: "):
            continue
        try:
            base_ms, head_ms = float(cells[1].replace(",", "")), float(cells[2].replace(",", ""))
            base_mib, head_mib = float(cells[4].replace(",", "")), float(cells[5].replace(",", ""))
        except ValueError:
            bad.append(cells[0])
            continue
        if base_ms <= 0 or base_mib <= 0:
            bad.append(cells[0])
            continue
        # A sample's row is `YYYY-<its file's lowercase stem>`; a model's name
        # has capitals or another separator (`2024_Core_Interior`).
        year = re.fullmatch(r"(\d{4})-[a-z0-9_]+", cells[0])
        good.append({
            "name": cells[0],
            "year": int(year.group(1)) if year else None,
            "base_ms": base_ms,
            "base_mib": base_mib,
            "time": head_ms / base_ms,
            "peak": head_mib / base_mib,
            # Each round's peak, base then head, where the table has them.
            "peak_rounds": (rounds(cells[9]), rounds(cells[10])) if len(cells) >= 11 else None,
        })
    return good, bad


def rounds(cell):
    """A cell's space-separated rounds as numbers, or None where any is not one
    (a failed export, a runner without GNU time)."""
    try:
        values = [float(v) for v in cell.split()]
    except ValueError:
        return None
    return values if values and min(values) > 0 else None


def two_values(row):
    """Whether either side's peak rounds differ by more than HIGHER."""
    sides = row["peak_rounds"]
    return bool(sides) and all(sides) and any(max(side) > HIGHER * min(side) for side in sides)


def mib(side):
    """Rounds in KiB, as whole MiB."""
    return " ".join(str(round(v / 1024)) for v in side)


def higher(row):
    """Whether the head's peak is past HIGHER: by the medians, or, where the
    rounds take two values, by every head round against every base round."""
    if two_values(row):
        base, head = row["peak_rounds"]
        return min(head) > HIGHER * max(base)
    return row["peak"] > HIGHER


def spread(values):
    return f"median {statistics.median(values):.3f} ({min(values):.3f} to {max(values):.3f})"


def main(argv):
    if len(argv) != 2:
        sys.exit(__doc__)
    try:
        table, bad = rows(argv[1])
    except OSError:
        print("### Timing verdict\n\nNo timing table.\n")
        return
    print("### Timing verdict")
    print()
    if not table:
        print("No row of the table reads as numbers.\n")
        return
    groups = [
        ("Models", [r for r in table if r["year"] is None]),
        ("Samples of Revit 2023 to 2027", [r for r in table if r["year"] and r["year"] >= 2023]),
        ("Samples of Revit 2015 to 2022", [r for r in table if r["year"] and r["year"] < 2023]),
    ]
    print("| group | files | time head / base | peak head / base |")
    print("|---|---:|---|---|")
    for label, group in groups:
        if group:
            print(f"| {label} | {len(group)} | {spread([r['time'] for r in group])} | {spread([r['peak'] for r in group])} |")
    print()
    for label, key in (("longest base time", "base_ms"), ("highest base peak", "base_mib")):
        top = sorted(table, key=lambda r: r[key], reverse=True)[:3]
        print(f"- The three files with the {label}: " + "; ".join(
            f"{r['name']} time {r['time']:.3f}, peak {r['peak']:.3f}" for r in top))
    slower = [r for r in table if r["time"] > SLOWER]
    high = [r for r in table if higher(r)]
    split = [r for r in table if two_values(r)]
    print(f"- More than {round((SLOWER - 1) * 100)}% slower: " + (
        "; ".join(f"**{r['name']}** {r['time']:.3f}" for r in slower) if slower else "none"))
    print(f"- Peak more than {round((HIGHER - 1) * 100)}% higher: " + (
        "; ".join(f"**{r['name']}** {r['peak']:.3f}" for r in high) if high else "none"))
    print(f"- Peak in two values from round to round, rounds more than {round((HIGHER - 1) * 100)}% apart, "
          "so the median is either (higher above only where every head round is): " + (
        "; ".join(f"**{r['name']}** {r['peak']:.3f} (base {mib(r['peak_rounds'][0])}, head {mib(r['peak_rounds'][1])} MiB)"
                  for r in split) if split else "none"))
    if bad:
        print(f"- Rows whose figures do not read as numbers: {', '.join(bad)}")
    print()


if __name__ == "__main__":
    main(sys.argv)
