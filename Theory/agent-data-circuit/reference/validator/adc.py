"""``adc validate`` - structural conformance checker for a static ADC site.

    python reference/validator/adc.py validate ./examples/ecommerce
    python reference/validator/adc.py validate ./site --json

Exit codes: 0 conforming, 1 problems found, 2 usage error.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from adc.errors import AdcError  # noqa: E402
from adc.site import Site, check_site  # noqa: E402


def _print_human(site_dir: Path, problems: list[str]) -> None:
    if not problems:
        try:
            site = Site.load(site_dir)
            nodes = len(site.nodes)
            slices = sum(len(n.slices) for n in site.nodes.values())
            dynamic = sum(
                1 for n in site.nodes.values() for s in n.slices.values() if s.dynamic
            )
        except Exception:
            nodes = slices = dynamic = 0
        print(f"OK {site_dir}")
        print(f"  nodes: {nodes}  slices: {slices}  dynamic slices: {dynamic}")
        return
    print(f"FAIL {site_dir}: {len(problems)} problem(s)")
    for index, problem in enumerate(problems, 1):
        print(f"  {index:3d}. {problem}")


def cmd_validate(args: argparse.Namespace) -> int:
    site_dir = Path(args.site)
    if not site_dir.is_dir():
        print(f"error: {site_dir} is not a directory", file=sys.stderr)
        return 2
    problems = check_site(site_dir)
    if args.json:
        print(json.dumps({"site": str(site_dir), "ok": not problems,
                          "problems": problems}, indent=2))
    else:
        _print_human(site_dir, problems)
    if args.max_problems >= 0:
        # Threshold mode: tolerate up to N problems, still exit 1 beyond that.
        return 1 if len(problems) > args.max_problems else 0
    return 1 if problems else 0


def cmd_inspect(args: argparse.Namespace) -> int:
    try:
        site = Site.load(args.site)
        if args.path:
            node = site.resolve(path=args.path)
        elif args.anchor:
            node = site.resolve(anchor=args.anchor)
        else:
            print("error: pass --path or --anchor", file=sys.stderr)
            return 2
    except AdcError as exc:
        print(f"error: {exc.code}: {exc.detail}", file=sys.stderr)
        return 2
    except Exception as exc:
        print(f"error: cannot load site: {exc}", file=sys.stderr)
        return 2
    doc = {
        "path": node.path,
        "anchor": node.anchor,
        "kind": node.kind,
        "children": node.children,
        "filters": sorted(node.filter_specs),
        "slices": [
            {
                "name": s.name,
                "tokens": s.tokens,
                "bytes": s.bytes,
                "generation": s.generation,
                "dynamic": s.dynamic,
                "default": s.default,
                "provenance": s.provenance,
            }
            for s in node.slices.values()
        ],
    }
    print(json.dumps(doc, indent=2))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="adc", description="Agent Data Circuit validator"
    )
    sub = parser.add_subparsers(dest="command")

    validate = sub.add_parser("validate", help="validate a site directory")
    validate.add_argument("site", help="path to the site directory")
    validate.add_argument("--json", action="store_true", help="machine-readable output")
    validate.add_argument(
        "--max-problems",
        type=int,
        default=-1,
        help="fail only when more than N problems are found (-1 = any problem fails)",
    )
    validate.set_defaults(func=cmd_validate)

    inspect = sub.add_parser("inspect", help="print one node as JSON")
    inspect.add_argument("site", help="path to the site directory")
    inspect.add_argument("--path", help="canonical node path")
    inspect.add_argument("--anchor", help="wire anchor, e.g. [a10]")
    inspect.set_defaults(func=cmd_inspect)

    args = parser.parse_args(argv)
    if not getattr(args, "command", None):
        parser.print_help()
        return 2
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())