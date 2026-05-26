#!/usr/bin/env python3
"""Generate arch/sv_architecture.qmd from DOT file."""

from pathlib import Path

ARCH_DIR = Path(__file__).resolve().parent
DOT_FILE = ARCH_DIR / "sv_architecture.dot"
QMD_FILE = ARCH_DIR / "sv_architecture.qmd"


def generate_qmd():
    dot_content = DOT_FILE.read_text()
    qmd = f"""---
title: "SV Architecture Diagram"
subtitle: "Auto-generated from POC SystemVerilog modules"
date: last-modified
metadata-files:
  - ../../_include/author.yml
abstract: |
  Module hierarchy and port connectivity extracted from
  poc/baremetal_riscv/sv/ via sv2dot.py.
---

{{{{< include ../../_include/_title_meta_items.qmd >}}}}

```{{python}}
#| include: false
#| context: local
%run ../../_include/_graphviz.py
```

```{{python}}
#| label: fig-sv-architecture
#| fig-cap: "SV module architecture: constraints, composition, projectors, observation pipeline"
dot(\"\"\"
{dot_content}
\"\"\")
```
"""
    QMD_FILE.write_text(qmd)
    print(f"Generated {QMD_FILE}")


if __name__ == "__main__":
    generate_qmd()
