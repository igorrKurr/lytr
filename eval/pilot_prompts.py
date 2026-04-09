"""Load fair (parity) system prompts for pilot A/B — shared stem + arm-specific suffix."""
from __future__ import annotations

from pathlib import Path


def _read(root: Path, rel: str) -> str:
    p = root / "eval" / "pilot" / rel
    return p.read_text(encoding="utf-8").strip()


def fair_lir_system(root: Path) -> str:
    return _read(root, "system_shared.md") + "\n\n" + _read(root, "system_arm_lir.md")


def fair_python_system(root: Path) -> str:
    return _read(root, "system_shared.md") + "\n\n" + _read(root, "system_arm_python.md")
