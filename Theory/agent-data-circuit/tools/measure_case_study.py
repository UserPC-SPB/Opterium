#!/usr/bin/env python3
"""Measure the ADC cold entry case study against a real site (CASE-STUDY.md).

Phase 1 scrapes a public RFC catalog: robots.txt is fetched and honored
first, then the official index, then the first 20 info pages with a 0.1 s
politeness delay. Five fields are extracted per document. Phase 2 builds a
local ADC site from that extraction, starts the reference server on
127.0.0.1, and measures GET /AI/ plus the projected query.

No third-party content is written into the repository. Rows, raw HTML, and
the built site stay in a temporary work directory; only aggregate metrics
are printed.

Usage:
    python tools/measure_case_study.py [--workdir DIR]

Exit codes: 2 robots.txt disallows the section, 3 index parse failed.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
import urllib.robotparser
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "reference"))

from adc.anchors import AnchorTable  # noqa: E402
from adc.site import payload_stats  # noqa: E402

UA = "adc-case-study/0.1 (local research; polite)"
BASE = "https://www.rfc-editor.org"
INDEX_URL = BASE + "/rfc-index.txt"
ROBOTS_URL = BASE + "/robots.txt"
N_ITEMS = 20
POLITE_DELAY_S = 0.1
FIELDS = ("title", "status", "authors", "year", "words")
GEN = "2026-09-25T00:00:00Z"
UPDATED = "2026-09-25T00:00:00Z"
TOKENIZER = "utf8_bytes_div_4"
PORT = 8123
ORIGIN = "http://127.0.0.1:%d" % PORT
QUERY = "/AI[s1]?get=text&fields=%s&limit=20" % ",".join(FIELDS)
STATUSES = (
    "Internet Standard",
    "Proposed Standard",
    "Best Current Practice",
    "Informational",
    "Experimental",
    "Unknown",
    "None",
    "Not Issued",
)

LOCAL_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def tokens_of(raw: bytes) -> int:
    return math.ceil(len(raw) / 4)


def fetch(url: str, timeout: float = 45.0, retries: int = 1) -> tuple[bytes, float, str]:
    last_exc: Exception | None = None
    for attempt in range(retries + 1):
        started = time.perf_counter()
        try:
            request = urllib.request.Request(url, headers={"User-Agent": UA})
            with urllib.request.urlopen(request, timeout=timeout) as response:
                body = response.read()
                final = response.geturl()
            return body, time.perf_counter() - started, final
        except Exception as exc:
            last_exc = exc
            if attempt < retries:
                time.sleep(1.0)
    raise RuntimeError("fetch failed for %s: %s" % (url, last_exc)) from last_exc


def check_robots() -> tuple[dict, bool]:
    body, seconds, _ = fetch(ROBOTS_URL)
    text = body.decode("utf-8", "replace")
    parser = urllib.robotparser.RobotFileParser()
    parser.parse(text.splitlines())
    allowed_index = parser.can_fetch(UA, INDEX_URL)
    allowed_items = parser.can_fetch(UA, BASE + "/info/rfc1/")
    report = {
        "url": ROBOTS_URL,
        "bytes": len(body),
        "seconds": round(seconds, 3),
        "allow_index": allowed_index,
        "allow_items": allowed_items,
        "disallow_lines": [
            line for line in text.splitlines() if line.lower().startswith("disallow")
        ],
    }
    return report, allowed_index and allowed_items


def parse_index(text: str) -> list[int]:
    lines = text.splitlines()
    start = 0
    for index, line in enumerate(lines):
        if line.strip() == "RFC INDEX":
            start = index
            break
    numbers: list[int] = []
    for line in lines[start:]:
        match = re.match(r"^(\d{1,4}) [A-Za-z(\[]", line)
        if not match:
            continue
        if re.match(r"^\d{1,4} Not Issued", line):
            continue
        numbers.append(int(match.group(1)))
    return numbers


def visible_text(html: str) -> str:
    text = re.sub(r"<script.*?</script>", " ", html, flags=re.S | re.I)
    text = re.sub(r"<style.*?</style>", " ", text, flags=re.S | re.I)
    text = re.sub(r"<[^>]+>", " ", text)
    text = re.sub(r"&nbsp;|&#160;", " ", text)
    text = text.replace("&amp;", "&")
    return re.sub(r"\s+", " ", text).strip()


def dl_rows(html: str) -> dict:
    out: dict[str, str] = {}
    pattern = (
        r"<dt[^>]*>\s*(?:<!--.*?-->)?\s*(.*?)\s*</dt>\s*<dd[^>]*>(.*?)</dd>"
    )
    for label, value in re.findall(pattern, html, re.S):
        label = re.sub(r"<[^>]+>", "", label).strip()
        text = re.sub(r"<[^>]+>", " ", value)
        out[label] = re.sub(r"\s+", " ", text).strip()
    return out


def extract_row(rfc_number: int, html_bytes: bytes) -> dict:
    html = html_bytes.decode("utf-8", "replace")
    title_match = re.search(r"<title>(.*?)</title>", html, re.S)
    title = title_match.group(1).strip() if title_match else None
    if title:
        title = re.sub(r"^RFC\s*%d:\s*" % rfc_number, "", title)
        title = re.sub(r"\s*\|\s*RFC Editor\s*$", "", title)
    badge = re.search(r"bg-blue-400[^>]*><!--\[-->([^<]+?)<!--\]-->", html)
    status = (
        badge.group(1)
        if badge
        else next((s for s in STATUSES if s in html), None)
    )
    rows = dl_rows(html)
    authors = rows.get("Authors")
    if authors:
        authors = re.sub(r"\s*\.\s*$", "", authors).strip()
    date = rows.get("Date published", "")
    year_match = re.search(r"\b(\d{4})\b", date)
    year = int(year_match.group(1)) if year_match else None
    visible = visible_text(html)
    start = visible.find("RFC %d Info" % rfc_number)
    end = visible.find("About this RFC")
    words = len(visible[start:end].split()) if start >= 0 and end > start else None
    return {
        "title": title,
        "status": status,
        "authors": authors,
        "year": year,
        "words": words,
    }


def write_json(path: Path, payload) -> None:
    path.write_text(
        json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8"
    )


def scrape(workdir: Path) -> tuple[dict, list[dict], list[int]]:
    started = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    robots, allowed = check_robots()
    if not allowed:
        print("ABORT: robots.txt disallows the chosen section", file=sys.stderr)
        print(json.dumps(robots, indent=2), file=sys.stderr)
        raise SystemExit(2)
    print(
        "robots.txt: allowed (index=%s, items=%s)"
        % (robots["allow_index"], robots["allow_items"])
    )

    t_start = time.perf_counter()
    index_bytes, index_seconds, index_final = fetch(INDEX_URL)
    index_requests = 1 + (1 if index_final != INDEX_URL else 0)
    numbers = parse_index(index_bytes.decode("utf-8", "replace"))
    if len(numbers) < N_ITEMS or numbers[:N_ITEMS] != sorted(numbers[:N_ITEMS]):
        print(
            "ABORT: index parse unexpected (%d entries)" % len(numbers),
            file=sys.stderr,
        )
        raise SystemExit(3)
    sample = numbers[:N_ITEMS]
    print(
        "index: %d bytes, %d entries, sample=%s..."
        % (len(index_bytes), len(numbers), sample[:5])
    )

    pages: list[dict] = []
    rows_out: list[dict] = []
    for position, number in enumerate(sample):
        url = "%s/info/rfc%d/" % (BASE, number)
        if position:
            time.sleep(POLITE_DELAY_S)
        body, seconds, final = fetch(url)
        requests_for_page = 1 + (1 if final != url else 0)
        row = extract_row(number, body)
        encoded = json.dumps(row, ensure_ascii=True, separators=(",", ":")).encode(
            "utf-8"
        )
        missing = [key for key, value in row.items() if value is None]
        pages.append(
            {
                "rfc": number,
                "requests": requests_for_page,
                "html_bytes": len(body),
                "html_tokens": tokens_of(body),
                "seconds": round(seconds, 4),
                "extracted_bytes": len(encoded),
                "extracted_tokens": tokens_of(encoded),
                "missing_fields": missing,
            }
        )
        rows_out.append(row)
        print(
            "  rfc%-5d html=%-8d tokens=%-7d extracted=%-4d missing=%s"
            % (number, len(body), tokens_of(body), len(encoded), missing)
        )
    t_end = time.perf_counter()

    missing_counts: dict[str, int] = {}
    for page in pages:
        for field in page["missing_fields"]:
            missing_counts[field] = missing_counts.get(field, 0) + 1

    metrics = {
        "phase": "scrape-baseline",
        "started_utc": started,
        "site": {
            "class": "public RFC catalog (standards documents)",
            "origin": BASE,
            "section": "official RFC index -> first 20 info pages",
            "user_agent": UA,
        },
        "robots": robots,
        "parameters": {
            "n_items": N_ITEMS,
            "polite_delay_s": POLITE_DELAY_S,
            "tokenizer": "utf8_bytes_div_4 (ceil(bytes/4))",
        },
        "index": {
            "url": INDEX_URL,
            "bytes": len(index_bytes),
            "tokens": tokens_of(index_bytes),
            "seconds": round(index_seconds, 4),
            "entries": len(numbers),
        },
        "items": {
            "opened": len(pages),
            "bytes": sum(p["html_bytes"] for p in pages),
            "tokens": sum(p["html_tokens"] for p in pages),
            "seconds": round(sum(p["seconds"] for p in pages), 4),
            "html_bytes_min": min(p["html_bytes"] for p in pages),
            "html_bytes_max": max(p["html_bytes"] for p in pages),
            "html_bytes_avg": round(
                sum(p["html_bytes"] for p in pages) / len(pages)
            ),
            "extracted_bytes": sum(p["extracted_bytes"] for p in pages),
            "extracted_tokens": sum(p["extracted_tokens"] for p in pages),
            "extracted_bytes_avg": round(
                sum(p["extracted_bytes"] for p in pages) / len(pages)
            ),
            "missing_field_pages": missing_counts,
            "pages": pages,
        },
        "totals": {
            "requests": index_requests + sum(p["requests"] for p in pages),
            "bytes": len(index_bytes) + sum(p["html_bytes"] for p in pages),
            "tokens_raw_html": tokens_of(index_bytes)
            + sum(p["html_tokens"] for p in pages),
            "tokens_extracted": sum(p["extracted_tokens"] for p in pages),
            "seconds": round(t_end - t_start, 3),
        },
    }
    write_json(workdir / "scrape_metrics.json", metrics)
    write_json(workdir / "rows.json", rows_out)
    return metrics, rows_out, sample


def slice_descriptor(payload: dict) -> dict:
    tokens, size, generation = payload_stats(payload, TOKENIZER)
    return {
        "name": "summary",
        "tokens": tokens,
        "bytes": size,
        "generation": generation,
        "generated_at": UPDATED,
        "dynamic": False,
        "provenance": "publisher",
        "default": True,
    }


def write_node(
    site: Path,
    anchor: str,
    path: str,
    kind: str,
    summary_payload: dict,
    count: int | None = None,
) -> None:
    passport = {
        "tree_generation": GEN,
        "path": path,
        "anchor": anchor,
        "kind": kind,
        "updated_at": UPDATED,
        "slices": [slice_descriptor(summary_payload)],
    }
    if count is not None:
        passport["count"] = count
    anchor_id = anchor.strip("[]")[1:]
    write_json(
        site / "nodes" / ("a%s.json" % anchor_id),
        {"passport": passport, "slices": {"summary": summary_payload}},
    )


def build_site(site: Path, rows: list[dict], rfcs: list[int]) -> list[dict]:
    if site.exists():
        shutil.rmtree(site)
    (site / "nodes").mkdir(parents=True)

    paths = ["/", "/rfcs"] + ["/rfcs/rfc%d" % n for n in rfcs]
    table = AnchorTable.from_paths(GEN, paths)

    manifest = json.loads(
        (REPO / "examples" / "ecommerce" / "manifest.json").read_text(encoding="utf-8")
    )
    manifest["root"] = ORIGIN
    manifest["namespace"] = "org.rfc-editor"
    manifest["tree_generation"] = GEN
    manifest["updated_at"] = UPDATED
    write_json(site / "manifest.json", manifest)

    entries = [
        {
            "name": "/",
            "path": "/",
            "anchor": table.anchor_for("/"),
            "kind": "collection",
            "has_children": True,
            "count": 1,
            "updated_at": UPDATED,
        },
        {
            "name": "rfcs",
            "path": "/rfcs",
            "anchor": table.anchor_for("/rfcs"),
            "kind": "collection",
            "has_children": True,
            "count": len(rows),
            "updated_at": UPDATED,
        },
    ]
    for rfc in rfcs:
        entries.append(
            {
                "name": "rfc%d" % rfc,
                "path": "/rfcs/rfc%d" % rfc,
                "anchor": table.anchor_for("/rfcs/rfc%d" % rfc),
                "kind": "entity",
                "has_children": False,
                "updated_at": UPDATED,
            }
        )
    write_json(site / "tree.json", {"tree_generation": GEN, "nodes": entries})

    write_node(
        site,
        entries[0]["anchor"],
        "/",
        "collection",
        {"items": [{"path": "/rfcs", "fields": {"name": "rfcs", "count": len(rows)}}]},
        count=1,
    )
    items = [
        {"path": "/rfcs/rfc%d" % rfc, "fields": dict(row)}
        for rfc, row in zip(rfcs, rows)
    ]
    write_node(
        site,
        entries[1]["anchor"],
        "/rfcs",
        "collection",
        {"items": items},
        count=len(rows),
    )
    for rfc, row in zip(rfcs, rows):
        write_node(
            site,
            table.anchor_for("/rfcs/rfc%d" % rfc),
            "/rfcs/rfc%d" % rfc,
            "entity",
            {"fields": dict(row)},
        )
    write_json(
        site / "ai.json",
        {"ai": "1.0", "vars": {"s1": "/rfcs"}, "keys": ["text", "meta"]},
    )
    return entries


def timed_get(path: str) -> tuple[int, bytes, float]:
    request = urllib.request.Request(ORIGIN + path)
    started = time.perf_counter()
    with LOCAL_OPENER.open(request, timeout=30.0) as response:
        body = response.read()
        status = response.status
    return status, body, time.perf_counter() - started


def wait_ready(timeout: float = 20.0) -> None:
    deadline = time.time() + timeout
    last: str = "no attempt"
    while time.time() < deadline:
        try:
            status, _, _ = timed_get("/adc/orient")
            if status == 200:
                return
            last = "status %d" % status
        except Exception as exc:
            last = repr(exc)
        time.sleep(0.15)
    raise SystemExit("server did not become ready: %s" % last)


def serve_and_measure(
    workdir: Path, rows: list[dict], rfcs: list[int]
) -> tuple[dict, list[dict]]:
    site = workdir / "site"
    entries = build_site(site, rows, rfcs)
    env = dict(os.environ, ADC_QUIET="1", ADC_NO_RATE_LIMIT="1")
    process = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "reference.adc.server",
            str(site),
            "--host",
            "127.0.0.1",
            "--port",
            str(PORT),
        ],
        cwd=str(REPO),
        env=env,
    )
    try:
        wait_ready()
        wall_start = time.perf_counter()
        map_status, map_body, map_seconds = timed_get("/AI/")
        query_status, query_body, query_seconds = timed_get(QUERY)
        wall_seconds = time.perf_counter() - wall_start

        payload = json.loads(query_body.decode("utf-8"))
        items = payload.get("items", [])
        if query_status != 200 or payload.get("count") != len(rows):
            raise SystemExit(
                "query failed: status=%d count=%r"
                % (query_status, payload.get("count"))
            )
        if items != [dict(row) for row in rows]:
            raise SystemExit("projected rows do not match the extraction")

        metrics = {
            "phase": "adc-local",
            "site": {
                "class": "local ADC site built from the phase-1 extraction",
                "nodes": len(entries),
                "rows": len(rows),
                "tokenizer": TOKENIZER,
            },
            "server": {"host": "127.0.0.1", "port": PORT, "readiness_probed": True},
            "requests": [
                {
                    "path": "/AI/",
                    "status": map_status,
                    "bytes": len(map_body),
                    "seconds": round(map_seconds, 4),
                    "tokens": tokens_of(map_body),
                },
                {
                    "path": QUERY,
                    "status": query_status,
                    "bytes": len(query_body),
                    "seconds": round(query_seconds, 4),
                    "tokens": tokens_of(query_body),
                    "items": len(items),
                },
            ],
            "totals": {
                "requests": 2,
                "bytes": len(map_body) + len(query_body),
                "tokens": tokens_of(map_body) + tokens_of(query_body),
                "seconds_wall": round(wall_seconds, 4),
                "seconds_sum": round(map_seconds + query_seconds, 4),
            },
        }
        write_json(workdir / "adc_metrics.json", metrics)
        return metrics, entries
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--workdir",
        default=None,
        help="where to keep rows and the built site (default: temp dir)",
    )
    args = parser.parse_args()
    if args.workdir:
        workdir = Path(args.workdir)
        workdir.mkdir(parents=True, exist_ok=True)
    else:
        workdir = Path(tempfile.mkdtemp(prefix="adc-case-study-"))

    scrape_metrics, rows, rfcs = scrape(workdir)
    adc_metrics, _ = serve_and_measure(workdir, rows, rfcs)
    write_json(workdir / "metrics.json", {"scrape": scrape_metrics, "adc": adc_metrics})

    s = scrape_metrics["totals"]
    a = adc_metrics["totals"]
    print(
        "scrape : requests=%d bytes=%d tokens=%d seconds=%.3f"
        % (s["requests"], s["bytes"], s["tokens_raw_html"], s["seconds"])
    )
    print(
        "ai     : requests=%d bytes=%d tokens=%d seconds=%.4f"
        % (a["requests"], a["bytes"], a["tokens"], a["seconds_wall"])
    )
    print(
        "ratio  : bytes=%.0fx tokens=%.0fx requests=%.1fx"
        % (
            s["bytes"] / a["bytes"],
            s["tokens_raw_html"] / a["tokens"],
            s["requests"] / a["requests"],
        )
    )
    print("metrics: %s" % (workdir / "metrics.json"))


if __name__ == "__main__":
    main()
