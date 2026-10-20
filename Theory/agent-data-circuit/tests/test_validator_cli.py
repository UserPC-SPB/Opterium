"""Validator CLI tests (CONFORMANCE.md F.1, Annex E).

The CLI is exercised only through ``subprocess`` so that exit codes and
stdout/stderr formats are tested exactly as documented in
``reference/validator/adc.py``:

    Exit codes: 0 conforming, 1 problems found, 2 usage error.
"""

import json
import shutil
import subprocess
import sys

import pytest


def run_cli(repo_root, *args):
    script = repo_root / "reference" / "validator" / "adc.py"
    return subprocess.run(
        [sys.executable, str(script), *[str(a) for a in args]],
        cwd=str(repo_root),
        capture_output=True,
        encoding="utf-8",
        timeout=60,
    )


def _broken_site(example_site_dir, tmp_path):
    site_dir = tmp_path / "site"
    shutil.copytree(example_site_dir, site_dir)
    (site_dir / "nodes" / "a31.json").unlink()
    return site_dir


def test_validate_example_site_exits_zero(repo_root, example_site_dir):
    result = run_cli(repo_root, "validate", example_site_dir)
    assert result.returncode == 0, result.stdout + result.stderr
    assert result.stdout.startswith("OK ")
    assert "nodes:" in result.stdout


def test_validate_json_output_is_machine_readable(
    repo_root, example_site_dir
):
    result = run_cli(repo_root, "validate", example_site_dir, "--json")
    assert result.returncode == 0, result.stdout + result.stderr
    payload = json.loads(result.stdout)
    assert payload["ok"] is True
    assert payload["problems"] == []
    assert payload["site"] == str(example_site_dir)


def test_validate_reports_missing_node_and_exits_one(
    repo_root, example_site_dir, tmp_path
):
    site_dir = _broken_site(example_site_dir, tmp_path)
    result = run_cli(repo_root, "validate", site_dir)
    assert result.returncode == 1
    assert result.stdout.startswith("FAIL ")
    assert "a31.json" in result.stdout


def test_validate_json_reports_problems_and_exits_one(
    repo_root, example_site_dir, tmp_path
):
    site_dir = _broken_site(example_site_dir, tmp_path)
    result = run_cli(repo_root, "validate", site_dir, "--json")
    assert result.returncode == 1
    payload = json.loads(result.stdout)
    assert payload["ok"] is False
    assert payload["problems"]
    assert any("a31.json" in p for p in payload["problems"])


def test_validate_non_directory_exits_two(repo_root, tmp_path):
    result = run_cli(repo_root, "validate", tmp_path / "missing")
    assert result.returncode == 2
    assert "is not a directory" in result.stderr


def test_missing_subcommand_exits_two(repo_root):
    result = run_cli(repo_root)
    assert result.returncode == 2
    assert "usage:" in result.stdout.lower() or "usage:" in result.stderr.lower()


def test_inspect_by_path_prints_node_json(repo_root, example_site_dir):
    result = run_cli(
        repo_root, "inspect", example_site_dir, "--path", "/electronics/audio/headphones"
    )
    assert result.returncode == 0, result.stdout + result.stderr
    payload = json.loads(result.stdout)
    assert payload["path"] == "/electronics/audio/headphones"
    assert payload["anchor"] == "[a30]"
    assert payload["kind"] == "collection"
    assert payload["children"]  # non-empty child path list
    assert all(
        child.startswith("/electronics/audio/headphones/")
        for child in payload["children"]
    )
    assert "summary" in [s["name"] for s in payload["slices"]]


def test_inspect_by_anchor_prints_node_json(repo_root, example_site_dir):
    result = run_cli(repo_root, "inspect", example_site_dir, "--anchor", "[a31]")
    assert result.returncode == 0, result.stdout + result.stderr
    payload = json.loads(result.stdout)
    assert payload["anchor"] == "[a31]"
    assert payload["path"] == (
        "/electronics/audio/headphones/apple-airpods-3"
    )
    assert payload["kind"] == "entity"


def test_inspect_without_selector_exits_two(repo_root, example_site_dir):
    result = run_cli(repo_root, "inspect", example_site_dir)
    assert result.returncode == 2
    assert "--path or --anchor" in result.stderr


def test_inspect_unknown_anchor_exits_two(repo_root, example_site_dir):
    result = run_cli(repo_root, "inspect", example_site_dir, "--anchor", "[a99999]")
    assert result.returncode == 2
    assert "error:" in result.stderr


def test_max_problems_threshold_allows_exit_zero(
    repo_root, example_site_dir, tmp_path
):
    site_dir = _broken_site(example_site_dir, tmp_path)  # exactly 1 problem
    result = run_cli(
        repo_root, "validate", site_dir, "--max-problems", "5"
    )
    assert result.returncode == 0, result.stdout + result.stderr
