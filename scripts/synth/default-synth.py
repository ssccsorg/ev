#!/usr/bin/env python3
"""Synthesis channel — Yosys wrapper producing structured JSON report.

Usage:
  default-synth.py <file.sv> [top_module]

Runs Yosys synthesis on a SystemVerilog file and outputs a JSON report
to stdout containing gate counts, cell area, cell type breakdown,
DOT gate-level diagram path, and any Yosys warnings.
"""

import json, shutil, subprocess, sys, tempfile # noqa: E401
from pathlib import Path


def fail(reason: str) -> None:
    """Print error JSON and exit non-zero."""
    print(json.dumps({"tool": "yosys", "status": "error", "message": reason}))
    sys.exit(1)


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <file.sv> [top_module]", file=sys.stderr)
        sys.exit(1)

    sv_file = Path(sys.argv[1])
    top_module = sys.argv[2] if len(sys.argv) > 2 else sv_file.stem

    if not sv_file.exists():
        fail(f"File not found: {sv_file}")

    # ── Prerequisite ──────────────────────────────────────────────────
    yosys_bin = shutil.which("yosys")
    if not yosys_bin:
        fail("yosys not found in PATH")

    version = subprocess.run(
        [yosys_bin, "--version"], capture_output=True, text=True
    ).stdout.splitlines()[0]

    # ── Synthesis ─────────────────────────────────────────────────────
    work_dir = Path(tempfile.mkdtemp(prefix="yosys-synth-"))
    try:
        stat_log = work_dir / "stat.log"
        netlist_json = work_dir / "netlist.json"
        dot_file = work_dir / "netlist.dot"
        yosys_log = work_dir / "yosys.log"

        script = f"""
            read_verilog -sv "{sv_file}";
            hierarchy -top "{top_module}";
            proc;
            synth -top "{top_module}";
            stat -json > "{stat_log}";
            write_json "{netlist_json}";
            show -format dot -prefix "{work_dir}/netlist" "{top_module}";
        """

        result = subprocess.run(
            [yosys_bin, "-q", "-l", str(yosys_log), "-p", script],
            capture_output=True, text=True,
        )

        if result.returncode != 0:
            fail(f"Yosys exited with code {result.returncode}")

        # ── Parse statistics ──────────────────────────────────────────
        gate_count = None
        cell_area = None
        cell_types = None

        if stat_log.exists():
            try:
                data = json.loads(stat_log.read_text())
                top = data.get("top_module", data.get("design", {}))
                gate_count = top.get("num_cells")
                cell_area = top.get("area")
                modules = data.get("modules", {})
                mod_data = modules.get(top_module, {})
                cells = mod_data.get("cells", {})
                cell_types = {k: v for k, v in cells.items() if v > 0} or None
            except (json.JSONDecodeError, OSError):
                pass

        # ── DOT output ────────────────────────────────────────────────
        dot_path = None
        if dot_file.exists() and "digraph" in dot_file.read_text()[:200]:
            dot_path = str(dot_file)

        # ── Warnings ──────────────────────────────────────────────────
        warnings = None
        if yosys_log.exists():
            try:
                lines = yosys_log.read_text().splitlines()
                warns = [l.strip() for l in lines if "warning" in l.lower()]  # noqa
                if warns:
                    warnings = warns
            except OSError:
                pass

        # ── Output ────────────────────────────────────────────────────
        report = {
            "tool": "yosys",
            "version": version,
            "source": str(sv_file),
            "module_name": top_module,
            "gate_count": gate_count,
            "cell_area": cell_area,
            "cell_types": cell_types,
            "netlist_path": str(netlist_json),
            "dot_path": dot_path,
            "warnings": warnings,
            "status": "ok",
        }
        # Filter out None values for clean output
        report = {k: v for k, v in report.items() if v is not None}
        print(json.dumps(report, indent=2))

    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
