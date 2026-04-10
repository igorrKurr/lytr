"""
Shared Tier A assertion runner (used by run_tier_a.py and run_llm_eval.py).
"""
from __future__ import annotations

import json
import os
import shlex
import subprocess
from pathlib import Path


class HiddenAssertionsError(Exception):
    """Malformed or invalid eval/tasks/<id>/hidden/assertions.json."""

    def __init__(self, path: Path, message: str) -> None:
        self.path = path
        super().__init__(f"{path}: {message}")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def lytr_base_cmd() -> list[str]:
    """
    Resolve how to invoke `lytr`:
    1. `LYTR` env if set (space-separated argv).
    2. Else `target/release/lytr` if present.
    3. Else `cargo run -q --bin lytr --`.
    """
    raw = os.environ.get("LYTR")
    if raw is not None and raw.strip():
        return shlex.split(raw)
    release = repo_root() / "target" / "release" / "lytr"
    if release.is_file():
        return [str(release.resolve())]
    return shlex.split("cargo run -q --bin lytr --")


def _lytr_subprocess_env() -> dict[str, str]:
    env = dict(os.environ)
    cmd = lytr_base_cmd()
    if not cmd or Path(cmd[0]).name != "cargo":
        return env
    rf = env.get("RUSTFLAGS", "")
    if "-Awarnings" not in rf.replace(" ", ""):
        env["RUSTFLAGS"] = (rf + " -A warnings").strip()
    return env


def run_lytr(args: list[str], cwd: Path) -> tuple[int, str, str]:
    cmd = lytr_base_cmd() + args
    p = subprocess.run(
        cmd,
        cwd=cwd,
        text=True,
        capture_output=True,
        timeout=120,
        env=_lytr_subprocess_env(),
    )
    return p.returncode, p.stdout, p.stderr


def lir_base_cmd() -> list[str]:
    """
    Resolve how to invoke `lir`:
    1. `LIR` env if set (e.g. `/usr/local/bin/lir` in Docker).
    2. Else `target/release/lir` if present (after `cargo build --release` — no cargo on hot path).
    3. Else `cargo run -q --bin lir --` (dev fallback).
    """
    raw = os.environ.get("LIR")
    if raw is not None and raw.strip():
        return shlex.split(raw)
    release = repo_root() / "target" / "release" / "lir"
    if release.is_file():
        return [str(release.resolve())]
    return shlex.split("cargo run -q --bin lir --")


def _lir_subprocess_env() -> dict[str, str]:
    """
    Default `LIR` is `cargo run …`, so rustc may print warnings on stderr before `lir` output.
    Append `-A warnings` so eval logs and LLM retry prompts only show LIR diagnostics.
    Skipped when `LIR` points at a prebuilt binary (no rustc in the subprocess).
    """
    env = dict(os.environ)
    cmd = lir_base_cmd()
    if not cmd or Path(cmd[0]).name != "cargo":
        return env
    rf = env.get("RUSTFLAGS", "")
    if "-Awarnings" not in rf.replace(" ", ""):
        env["RUSTFLAGS"] = (rf + " -A warnings").strip()
    return env


def run_lir(args: list[str], cwd: Path) -> tuple[int, str, str]:
    cmd = lir_base_cmd() + args
    p = subprocess.run(
        cmd,
        cwd=cwd,
        text=True,
        capture_output=True,
        timeout=120,
        env=_lir_subprocess_env(),
    )
    return p.returncode, p.stdout, p.stderr


def try_canonicalize_lir_file(root: Path, lir_path: Path) -> tuple[bool, str]:
    """
    Run `lir fmt` (no --check) and replace the file with §11 canonical text.
    Returns (ok, stderr) — ok is False if parse/format failed.
    """
    code, out, err = run_lir(["fmt", str(lir_path.resolve())], root)
    if code != 0:
        return False, norm_out(err)
    lir_path.write_text(norm_out(out), encoding="utf-8")
    return True, ""


def norm_out(s: str) -> str:
    return s.replace("\r\n", "\n")


def load_hidden_assertions(starter_lir: Path) -> list[dict]:
    """
    Optional extra assertions next to the starter: <task>/hidden/assertions.json
    Same schema as manifest assertions (kinds: lir_check, lir_run, fmt_check, codegen_check).
    Records are tagged with hidden=True in run_assertions_on_file.
    Raises HiddenAssertionsError on invalid JSON or shape.
    """
    path = starter_lir.parent / "hidden" / "assertions.json"
    if not path.is_file():
        return []
    try:
        with open(path, encoding="utf-8") as f:
            raw = json.load(f)
    except json.JSONDecodeError as e:
        raise HiddenAssertionsError(path, f"invalid JSON ({e})") from e
    if isinstance(raw, list):
        items = raw
    elif isinstance(raw, dict) and "assertions" in raw:
        items = raw["assertions"]
    else:
        raise HiddenAssertionsError(
            path,
            'must be a JSON array or an object with an "assertions" array',
        )
    if not isinstance(items, list):
        raise HiddenAssertionsError(path, '"assertions" must be an array')
    out: list[dict] = []
    for i, a in enumerate(items):
        if not isinstance(a, dict):
            raise HiddenAssertionsError(path, f"assertions[{i}] must be an object")
        b = dict(a)
        if "kind" not in b:
            raise HiddenAssertionsError(path, f"assertions[{i}] missing \"kind\"")
        b["__tier_a_hidden__"] = True
        out.append(b)
    return out


def merge_manifest_and_hidden(manifest_assertions: list, starter_lir: Path) -> list[dict]:
    return [*manifest_assertions, *load_hidden_assertions(starter_lir)]


def run_assertions_on_file(
    root: Path,
    lir_file: Path,
    task_id: str,
    tier: str,
    assertions: list,
    ts: str,
    out_log: Path | None,
    extra_fields: dict | None = None,
) -> tuple[int, list[dict]]:
    """
    Run manifest assertions against an on-disk .lir file.
    Returns (failure_count, records).
    """
    failed = 0
    records: list[dict] = []
    path_s = str(lir_file.resolve())

    for i, ass in enumerate(assertions):
        ass = dict(ass)
        is_hidden = bool(ass.pop("__tier_a_hidden__", False))
        kind = ass["kind"]
        expect_exit = ass.get("expect_exit", 0)
        ok = True
        detail: dict = {"assertion_index": i, "kind": kind, "hidden": is_hidden}
        if extra_fields:
            detail.update(extra_fields)

        if kind == "lir_check":
            code, _o, _e = run_lir(["check", path_s], root)
            ok = code == expect_exit
            detail["exit_code"] = code
            if not ok and _e:
                detail["stderr_got"] = norm_out(_e)[:800]
        elif kind == "fmt_check":
            code, _o, _e = run_lir(["fmt", "--check", path_s], root)
            ok = code == expect_exit
            detail["exit_code"] = code
            if not ok and _e:
                detail["stderr_got"] = norm_out(_e)[:800]
        elif kind == "codegen_check":
            code, _o, _e = run_lir(["codegen-check", path_s], root)
            ok = code == expect_exit
            detail["exit_code"] = code
            if not ok and _e:
                detail["stderr_got"] = norm_out(_e)[:800]
        elif kind == "lir_run":
            inp = ass.get("input")
            args = ["run", path_s]
            if inp is not None:
                args.extend(["--input", inp])
            code, out, _e = run_lir(args, root)
            expected = ass.get("expect_stdout", "")
            got = norm_out(out)
            want = norm_out(expected)
            ok = code == 0 and got == want
            detail["exit_code"] = code
            detail["stdout_got"] = got[:200]
            detail["stdout_want"] = want[:200]
            if not ok and _e:
                detail["stderr_got"] = norm_out(_e)[:800]
        else:
            ok = False
            detail["error"] = f"unknown assertion kind: {kind}"

        record = {
            "tier": tier,
            "task_id": task_id,
            "ts": ts,
            "pass": ok,
            **detail,
        }
        records.append(record)
        if out_log is not None:
            with open(out_log, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(record) + "\n")
        if not ok:
            failed += 1

    return failed, records


def run_lytr_assertions_on_file(
    root: Path,
    lytr_file: Path,
    task_id: str,
    tier: str,
    assertions: list,
    ts: str,
    out_log: Path | None,
    extra_fields: dict | None = None,
) -> tuple[int, list[dict]]:
    """
    Run LYTR manifest assertions against an on-disk .lytr file.
    Kinds: ``lytr_check``, ``lytr_run`` (optional ``expect_stdout``; ``input`` is ignored — bootstrap has no stdin).
    Returns (failure_count, records).
    """
    failed = 0
    records: list[dict] = []
    path_s = str(lytr_file.resolve())

    for i, ass in enumerate(assertions):
        ass = dict(ass)
        is_hidden = bool(ass.pop("__tier_a_hidden__", False))
        kind = ass["kind"]
        expect_exit = ass.get("expect_exit", 0)
        ok = True
        detail: dict = {"assertion_index": i, "kind": kind, "hidden": is_hidden}
        if extra_fields:
            detail.update(extra_fields)

        if kind == "lytr_check":
            code, _o, _e = run_lytr(["check", path_s], root)
            ok = code == expect_exit
            detail["exit_code"] = code
            if not ok and _e:
                detail["stderr_got"] = norm_out(_e)[:800]
        elif kind == "lytr_run":
            code, out, _e = run_lytr(["run", path_s], root)
            expected = ass.get("expect_stdout", "")
            got = norm_out(out)
            want = norm_out(expected)
            ok = code == 0 and got == want
            detail["exit_code"] = code
            detail["stdout_got"] = got[:200]
            detail["stdout_want"] = want[:200]
            if not ok and _e:
                detail["stderr_got"] = norm_out(_e)[:800]
        else:
            ok = False
            detail["error"] = f"unknown LYTR assertion kind: {kind}"

        record = {
            "tier": tier,
            "task_id": task_id,
            "ts": ts,
            "pass": ok,
            **detail,
        }
        records.append(record)
        if out_log is not None:
            with open(out_log, "a", encoding="utf-8") as lf:
                lf.write(json.dumps(record) + "\n")
        if not ok:
            failed += 1

    return failed, records
