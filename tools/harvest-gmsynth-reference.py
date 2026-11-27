#!/usr/bin/env python3
"""Updates the rectangles of the GM Synth panel's reference table from the panel test's failure messages.

After a layout change in `src/panels/inst/gm_synth.rs`, `cargo test --lib panels::tests::reference` fails with one line per
control: `#<id> of GMSynth [<case>]: edges [x0, y0, x1, y1] px, the table [...]`. This script reads those lines and rewrites
the `[x, y, w, h]` of the matching `row(<id>, ...)` in the case (in U; the test runs at 100 px per U). Other differences
(order of rows, widget kinds, margins, size rules, ink, labels) it only prints: change those rows by hand first.
Run it twice: the first run writes the rectangles, the second confirms the test passes.

Usage: tools/harvest-gmsynth-reference.py
"""
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
path = root / "src/panels/inst/gm_synth.rs"
out = subprocess.run(
    "cargo test --lib panels::tests::reference::every_panel 2>&1",
    shell=True, cwd=root, capture_output=True, text=True,
).stdout
other = [l for l in out.split("\n") if l.startswith(("GMSynth", "#")) and "edges [" not in l]
print("\n".join(other[:20]))
source = path.read_text()
cases = {}
for line in out.split("\n"):
    m = re.match(r"#(\d+) of GMSynth \[([^\]]+)\]: edges \[([\d\., ]+)\] px", line)
    if m:
        cases.setdefault(m.group(2), {})[int(m.group(1))] = [float(x) for x in m.group(3).split(",")]
for name, edges in cases.items():
    a = source.index('name: "%s"' % name)
    b = source.index("            ],\n", a)
    rows = []
    for line in source[a:b].split("\n"):
        m = re.match(r"\s*row\((\d+),", line)
        if m and int(m.group(1)) in edges:
            x0, y0, x1, y1 = edges[int(m.group(1))]
            rect = "[%.2f, %.2f, %.2f, %.2f]" % (x0 / 100, y0 / 100, (x1 - x0) / 100, (y1 - y0) / 100)
            line = re.sub(
                r"(row\(\d+, Widget::\w+, \"[^\"]*\", )\[[^\]]*\]",
                lambda mm: mm.group(1) + rect,
                line,
                count=1,
            )
        rows.append(line)
    source = source[:a] + "\n".join(rows) + source[b:]
path.write_text(source)
print([l for l in out.split("\n") if "test result" in l][:1])
