#!/usr/bin/env python3
"""Deterministic generator for the examples/ecommerce ADC reference dataset.

Writes manifest.json, tree.json and nodes/aN.json next to this file.
Standard library only. Every value is derived from plain arithmetic and
SHA-256 of the node path, never from an RNG or the system clock, so two
runs produce byte-identical files. Use --check to verify the committed
dataset against a fresh build (CI drift guard).

Tokenizer is utf8_bytes_div_4: tokens = ceil(bytes / 4) over the exact
UTF-8 bytes of json.dumps(payload, sort_keys=True, separators=(",", ":"),
ensure_ascii=False); slice generation = "V1:" + sha256(bytes)[:8].
"""

import argparse
import hashlib
import json
import math
import sys
import tempfile
from pathlib import Path

TREE_GENERATION = "2026-10-01T12:00:00Z"
UPDATED_AT = "2026-10-01T10:00:00Z"
MANIFEST_ROOT = "http://127.0.0.1:8080"
NAMESPACE = "com.example"
TOKENIZER = "utf8_bytes_div_4"
LICENSE = "CC-BY-4.0"

METRICS = [
    "count()",
    "sum(price)",
    "avg(price)",
    "min(price)",
    "max(price)",
    "p50(price)",
    "p95(price)",
]
GROUP_BY = ["brand", "feature"]

COLLECTIONS = [
    "/",
    "/computers",
    "/computers/laptops",
    "/electronics",
    "/electronics/audio",
    "/electronics/audio/headphones",
    "/electronics/audio/speakers",
]

HEADPHONE_FEATURES = [
    "anc", "bluetooth", "foldable", "gaming", "mic", "sport", "studio", "wireless",
]
SPEAKER_FEATURES = ["bluetooth", "portable", "smart", "stereo", "waterproof", "wifi"]
LAPTOP_FEATURES = [
    "2-in-1", "business", "creator", "gaming", "light", "thin", "touchscreen",
]

HEADPHONES = [
    ("Sony", "WH-1000XM5", 398),
    ("Sony", "WH-1000XM4", 278),
    ("Sony", "WH-CH720N", 118),
    ("Sony", "WH-CH520", 78),
    ("Sony", "WF-1000XM5", 248),
    ("Sony", "WF-C700N", 118),
    ("Bose", "QuietComfort Ultra Headphones", 429),
    ("Bose", "QuietComfort 45", 279),
    ("Bose", "Noise Cancelling 700", 269),
    ("Bose", "QuietComfort Earbuds II", 249),
    ("Bose", "Sport Earbuds", 149),
    ("Apple", "AirPods Max", 549),
    ("Apple", "AirPods Pro 2", 249),
    ("Apple", "AirPods 3", 179),
    ("Apple", "AirPods 4", 129),
    ("Sennheiser", "HD 660S2", 499),
    ("Sennheiser", "HD 560S", 199),
    ("Sennheiser", "Momentum 4", 299),
    ("Sennheiser", "Momentum True Wireless 4", 299),
    ("Sennheiser", "IE 600", 499),
    ("Audio-Technica", "ATH-M50x", 169),
    ("Audio-Technica", "ATH-M40x", 99),
    ("Audio-Technica", "ATH-ANC900BT", 199),
    ("Audio-Technica", "ATH-SR30BT", 119),
    ("JBL", "Tune 770NC", 129),
    ("JBL", "Tune 520BT", 49),
    ("JBL", "Live Pro 2", 149),
    ("JBL", "Live 660NC", 129),
    ("JBL", "Vibe Beam", 79),
    ("Beats", "Studio Pro", 349),
    ("Beats", "Solo3 Wireless", 199),
    ("Beats", "Fit Pro", 199),
    ("Beats", "Powerbeats Pro", 249),
    ("Beyerdynamic", "DT 770 Pro", 159),
    ("Beyerdynamic", "DT 990 Pro", 159),
    ("Beyerdynamic", "DT 700 Pro X", 259),
    ("Beyerdynamic", "DT 900 Pro X", 259),
    ("Marshall", "Monitor II ANC", 249),
    ("Marshall", "Major IV", 149),
    ("Marshall", "Mode II", 179),
    ("Jabra", "Elite 10", 249),
    ("Jabra", "Elite 85t", 179),
    ("Jabra", "Elite 4 Active", 99),
    ("Jabra", "Evolve2 65", 199),
    ("Soundcore", "Space One", 99),
    ("Soundcore", "Life Q30", 79),
    ("Soundcore", "Liberty 4 NC", 99),
    ("Technics", "EAH-A80", 199),
]

SPEAKERS = [
    ("Sonos", "One SL", 199),
    ("Sonos", "Roam 2", 179),
    ("Sonos", "Move 2", 399),
    ("Bose", "SoundLink Flex", 149),
    ("Bose", "SoundLink Max", 399),
    ("JBL", "Charge 5", 179),
    ("JBL", "Flip 6", 129),
    ("JBL", "Xtreme 4", 349),
    ("Marshall", "Acton III", 279),
    ("Marshall", "Stanmore II", 379),
    ("Apple", "HomePod mini", 99),
    ("Amazon", "Echo 5th Gen", 99),
    ("Google", "Nest Audio", 99),
    ("Ultimate Ears", "BOOM 4", 179),
    ("Ultimate Ears", "MEGABOOM 3", 199),
    ("Bang & Olufsen", "Beosound Explore", 199),
]

LAPTOPS = [
    ("Apple", "MacBook Air 13 M3", 1099),
    ("Apple", "MacBook Pro 14 M3 Pro", 1999),
    ("Apple", "MacBook Pro 16 M3 Max", 3499),
    ("Apple", "MacBook Air 15 M3", 1299),
    ("Dell", "XPS 13", 1299),
    ("Dell", "XPS 15", 1799),
    ("Dell", "XPS 16", 2399),
    ("Dell", "Inspiron 14 Plus", 899),
    ("Lenovo", "ThinkPad X1 Carbon Gen 11", 1849),
    ("Lenovo", "ThinkPad T14 Gen 4", 1399),
    ("Lenovo", "IdeaPad Slim 3", 649),
    ("Lenovo", "Yoga 9i", 1499),
    ("HP", "Spectre x360 14", 1449),
    ("HP", "Pavilion Plus 14", 999),
    ("HP", "EliteBook 840 G10", 1599),
    ("ASUS", "ZenBook 14", 999),
    ("ASUS", "ROG Zephyrus G14", 1699),
    ("ASUS", "VivoBook 15", 649),
    ("ASUS", "ProArt Studiobook 16", 2499),
    ("Acer", "Swift 3", 749),
    ("Acer", "Nitro V 15", 899),
    ("Microsoft", "Surface Laptop 5", 1299),
    ("Microsoft", "Surface Laptop Studio 2", 2099),
    ("Razer", "Blade 14", 2199),
]

CATEGORIES = [
    ("/computers/laptops", LAPTOPS, LAPTOP_FEATURES, 24),
    ("/electronics/audio/headphones", HEADPHONES, HEADPHONE_FEATURES, 48),
    ("/electronics/audio/speakers", SPEAKERS, SPEAKER_FEATURES, 16),
]

SHIPS_FROM = [
    "Memphis, TN", "Louisville, KY", "Reno, NV",
    "Allentown, PA", "Flower Mound, TX", "Chino, CA",
]

# Wired studio/open models: no battery, no wireless, no ANC, no codecs.
WIRED_HEADPHONES = {
    "ATH-M40x", "ATH-M50x", "DT 700 Pro X", "DT 770 Pro", "DT 900 Pro X",
    "DT 990 Pro X", "DT 990 Pro", "HD 560S", "HD 660S2", "IE 600",
}

# In-ear / true-wireless models: single-unit weights in grams.
IN_EAR_HEADPHONES = {
    "AirPods 3", "AirPods 4", "AirPods Pro 2", "Elite 10", "Elite 4 Active",
    "Elite 85t", "Fit Pro", "IE 600", "Liberty 4 NC", "Live Pro 2",
    "Mode II", "Momentum True Wireless 4", "Powerbeats Pro", "QuietComfort Earbuds II",
    "Sport Earbuds", "Vibe Beam", "WF-C700N", "WF-1000XM5",
}

# Laptop display resolution keyed by exact model, else by the size token
# in the model name, else by the seed.
DISPLAY_OVERRIDES = {
    "MacBook Air 13 M3": "13.6 inch 2560x1664",
    "MacBook Air 15 M3": "15.3 inch 2880x1864",
    "MacBook Pro 14 M3 Pro": "14.2 inch 3024x1964",
    "MacBook Pro 16 M3 Max": "16.2 inch 3456x2234",
    "ThinkPad X1 Carbon Gen 11": "14.0 inch 2240x1400",
    "ThinkPad T14 Gen 4": "14.0 inch 2240x1400",
    "Yoga 9i": "14.0 inch 2880x1800",
    "IdeaPad Slim 3": "15.6 inch 1920x1080",
    "EliteBook 840 G10": "14.0 inch 1920x1200",
    "Surface Laptop 5": "13.5 inch 2256x1504",
    "Surface Laptop Studio 2": "14.4 inch 2400x1600",
    "Swift 3": "14.0 inch 2240x1400",
    "ROG Zephyrus G14": "14.0 inch 2560x1600",
}
SIZE_DISPLAY = {
    "13": "13.3 inch 1920x1200",
    "14": "14.0 inch 2240x1400",
    "15": "15.6 inch 1920x1080",
    "16": "16.0 inch 2560x1600",
}
FALLBACK_DISPLAYS = [
    "14.0 inch 2240x1400", "15.6 inch 1920x1080", "16.0 inch 2560x1600",
]


def _check(condition, message):
    if not condition:
        raise AssertionError(message)


def seed(path):
    """Stable per-path seed; unaffected by PYTHONHASHSEED."""
    return int(hashlib.sha256(path.encode("utf-8")).hexdigest()[:16], 16)


def canonical_json(payload):
    return json.dumps(
        payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def dump(obj):
    return (json.dumps(obj, ensure_ascii=True, indent=2) + "\n").encode("ascii")


def slugify(text):
    tokens, current = [], []
    for ch in text.lower():
        if ("a" <= ch <= "z") or ("0" <= ch <= "9"):
            current.append(ch)
        elif current:
            tokens.append("".join(current))
            current = []
    if current:
        tokens.append("".join(current))
    return "-".join(tokens)


def build_entity(category, brand, model, base, pool):
    path = category + "/" + slugify(brand + " " + model)
    s = seed(path)
    price = round(base * (1 + ((s % 9) - 4) / 100.0), 2)
    picks = {pool[(s >> (2 * i + 1)) % len(pool)] for i in range(1 + (s >> 5) % 4)}
    if category == "/electronics/audio/headphones":
        if model in WIRED_HEADPHONES:
            picks.discard("wireless")
            picks.discard("anc")
            if not picks:
                picks.add("studio")
        else:
            picks.add("wireless")
            if base >= 99:
                picks.add("anc")
            else:
                picks.discard("anc")
    features = sorted(picks)

    weights = [
        35 + (s % 56),
        8 + ((s // 5) % 23),
        3 + ((s // 11) % 13),
        1 + ((s // 17) % 13),
        1 + ((s // 29) % 10),
    ]
    total = sum(weights)
    review_count = 25 + ((s // 7) % 3000)
    histogram = [review_count * w // total for w in weights]
    histogram[0] += review_count - sum(histogram)
    weighted = sum((5 - i) * histogram[i] for i in range(5)) / review_count
    rating = round(min(5.0, max(3.5, weighted)), 1)
    stock = (s // 19) % 61

    if category == "/electronics/audio/headphones":
        if model in WIRED_HEADPHONES:
            codec = []
        elif brand == "Apple":
            codec = ["aac", "sbc"]
        else:
            codec = ["sbc", "aac"]
            if s % 2 == 0:
                codec.append("aptx")
            if s % 3 == 0:
                codec.append("ldac")
        if model in IN_EAR_HEADPHONES:
            weight_g = 4 + (s % 14)
        else:
            weight_g = 180 + ((s // 3) % 420)
        specs = {
            "anc": "anc" in features,
            "battery_life": (
                "%d hours" % (20 + (s % 26)) if "wireless" in features else "wired"
            ),
            "weight_g": weight_g,
            "codec": codec,
        }
    elif category == "/electronics/audio/speakers":
        connectivity = ["bluetooth", "aux"]
        if "wifi" in features:
            connectivity.append("wifi")
        specs = {
            "driver_mm": 40 + ((s // 5) % 61),
            "power": "%dW" % (20 + (((s // 9) % 6) * 10)),
            "connectivity": connectivity,
            "weight_g": 300 + ((s // 13) % 3001),
        }
    else:
        if brand == "Apple":
            cpu = "Apple " + model[model.index("M3"):]
            ram = [8, 16, 16, 24][(s // 5) % 4]
        else:
            cpu = [
                "Intel Core Ultra 7 155H",
                "Intel Core i7-1360P",
                "Intel Core i5-13420H",
                "AMD Ryzen 7 7840U",
                "AMD Ryzen 5 7640HS",
            ][(s // 3) % 5]
            ram = [8, 16, 16, 32][(s // 5) % 4]
        if model in DISPLAY_OVERRIDES:
            display = DISPLAY_OVERRIDES[model]
        else:
            display = next(
                (SIZE_DISPLAY[token] for token in model.split()
                 if token in SIZE_DISPLAY),
                FALLBACK_DISPLAYS[s % len(FALLBACK_DISPLAYS)],
            )
        specs = {
            "cpu": cpu,
            "ram_gb": ram,
            "storage": ["256GB SSD", "512GB SSD", "512GB SSD", "1TB SSD", "2TB SSD"][
                (s // 7) % 5
            ],
            "display": display,
            "weight_g": 950 + ((s // 17) % 950),
        }

    fields = {
        "brand": brand,
        "model": model,
        "price": price,
        "currency": "USD",
        "feature": features,
        "rating": rating,
        "stock": stock,
    }
    return {
        "path": path,
        "fields": fields,
        "specs": {"fields": specs},
        "reviews": {
            "fields": {
                "rating": rating,
                "count": review_count,
                "histogram": histogram,
            }
        },
        "logistics": {
            "fields": {
                "in_stock": stock > 0,
                "ships_from": SHIPS_FROM[(s // 23) % len(SHIPS_FROM)],
                "delivery_days": 1 + ((s // 31) % 7),
            }
        },
    }


def slice_descriptor(name, payload, provenance, default=False, dynamic=False,
                     moderated=False):
    blob = canonical_json(payload)
    descriptor = {
        "name": name,
        "tokens": math.ceil(len(blob) / 4),
        "bytes": len(blob),
        "generation": "V1:" + hashlib.sha256(blob).hexdigest()[:8],
        "generated_at": UPDATED_AT,
        "dynamic": dynamic,
        "provenance": provenance,
    }
    if moderated:
        descriptor["moderated"] = True
    if dynamic:
        descriptor["price"] = 0.0005
        descriptor["currency"] = "USD"
        descriptor["unit"] = "call"
        descriptor["degradation"] = {
            "mode": "stale_fallback",
            "max_staleness_seconds": 900,
        }
    if default:
        descriptor["default"] = True
    return descriptor


def list_item_stats(per_item_bytes):
    count = len(per_item_bytes)
    mean_bytes = math.ceil(sum(per_item_bytes) / count)
    ordered = sorted(per_item_bytes)
    p95_bytes = max(ordered[math.ceil(0.95 * count) - 1], mean_bytes)
    return math.ceil(mean_bytes / 4), mean_bytes, math.ceil(p95_bytes / 4)


def build_manifest():
    return {
        "adc": "0.1",
        "adc_min": "0.1",
        "root": MANIFEST_ROOT,
        "namespace": NAMESPACE,
        "tree_generation": TREE_GENERATION,
        "tokenizer": TOKENIZER,
        "conformance": ["core", "dynamic", "commerce"],
        "endpoints": {
            "orient": "/adc/orient",
            "tree": "/adc/tree",
            "passport": "/adc/passport",
            "fetch": "/adc/fetch",
            "estimate": "/adc/estimate",
            "aggregate": "/adc/aggregate",
        },
        "policy": {
            "agents": "welcome",
            "license": LICENSE,
            "attribution": "required",
            "provenance_disclosure": True,
            "rate_limit": {"requests_per_minute": 120, "burst": 30},
        },
        "auth": {"required": False, "schemes": ["bearer"], "realm": "example"},
        "defaults": {"lang": "en", "currency": "USD", "page_size": 20},
        "limits": {
            "max_limit": 200,
            "max_depth": 4,
            "max_tree_nodes": 500,
            "max_aggregate_groups": 1000,
            "max_aggregate_group_by": 3,
            "max_post_bytes": 65536,
            "max_filter_conditions": 32,
            "max_filter_depth": 4,
        },
        "rate_limits": [
            {
                "scope": "anonymous",
                "requests": 60,
                "window": "PT1M",
                "burst": 10,
                "weights": {
                    "orient": 1,
                    "tree": 1,
                    "passport": 1,
                    "fetch": 1,
                    "estimate": 1,
                    "aggregate": 5,
                    "post_fetch": 2,
                },
            }
        ],
        "features": {
            "post_fetch": True,
            "fields": True,
            "estimate": True,
            "ndjson_streaming": False,
            "grep_regex": False,
            "aggregate": {"metrics": list(METRICS), "group_by": list(GROUP_BY)},
        },
        "trust": {
            "algorithm": "ES256",
            "jwks_uri": "/.well-known/jwks.json",
            "sla": {"max_dynamic_latency_ms": 450, "fallback_allowed": True},
        },
        "updated_at": TREE_GENERATION,
    }


def build_files():
    entities = {}
    for category, table, pool, expected in CATEGORIES:
        _check(len(table) == expected,
               "%s: expected %d rows, got %d" % (category, expected, len(table)))
        for brand, model, base in table:
            record = build_entity(category, brand, model, base, pool)
            _check(record["path"] not in entities, "duplicate " + record["path"])
            entities[record["path"]] = record

    collection_set = set(COLLECTIONS)
    _check(len(collection_set) == len(COLLECTIONS), "duplicate collection path")
    all_paths = sorted(collection_set | set(entities))
    children = {path: [] for path in all_paths}
    for path in all_paths:
        if path == "/":
            continue
        parent = path.rsplit("/", 1)[0] or "/"
        _check(parent in children, "missing parent for " + path)
        children[parent].append(path)
    for kids in children.values():
        kids.sort()
    ranks = {path: i for i, path in enumerate(all_paths, 1)}

    def anchor(path):
        return "[a%d]" % ranks[path]

    summary_payloads = {}
    item_bytes = {}
    for path in COLLECTIONS:
        items, blobs = [], []
        for child in children[path]:
            if child in entities:
                child_fields = entities[child]["fields"]
            else:
                child_fields = {
                    "name": child.rsplit("/", 1)[-1],
                    "count": len(children[child]),
                }
            items.append({"path": child, "fields": child_fields})
            blobs.append(len(canonical_json(items[-1])))
        summary_payloads[path] = {"items": items}
        item_bytes[path] = blobs

    brands = sorted({rec["fields"]["brand"] for rec in entities.values()})
    feature_enum = sorted(
        {f for rec in entities.values() for f in rec["fields"]["feature"]}
    )
    prices = [rec["fields"]["price"] for rec in entities.values()]
    filters = [
        {
            "name": "brand",
            "type": "string",
            "ops": ["eq", "in", "prefix"],
            "enum": brands,
        },
        {
            "name": "price",
            "type": "number",
            "ops": ["eq", "lt", "lte", "gt", "gte"],
            "unit": "USD",
            "min": math.floor(min(prices)),
            "max": math.ceil(max(prices)),
        },
        {
            "name": "feature",
            "type": "string",
            "ops": ["eq", "in", "contains", "contains_all", "contains_any"],
            "enum": feature_enum,
        },
    ]

    tree_nodes, node_docs = [], {}
    for path in all_paths:
        kind = "collection" if path in collection_set else "entity"
        node = {
            "name": "/" if path == "/" else path.rsplit("/", 1)[-1],
            "path": path,
            "anchor": anchor(path),
            "kind": kind,
            "has_children": bool(children[path]),
        }
        if kind == "collection":
            node["count"] = len(children[path])
        node["updated_at"] = UPDATED_AT
        tree_nodes.append(node)

        passport = {
            "tree_generation": TREE_GENERATION,
            "path": path,
            "anchor": anchor(path),
            "kind": kind,
        }
        if kind == "collection":
            per_item = item_bytes[path]
            tokens, byte_count, tokens_p95 = list_item_stats(per_item)
            passport["count"] = len(per_item)
            passport["list_item_tokens"] = tokens
            passport["list_item_bytes"] = byte_count
            passport["list_item_tokens_p95"] = tokens_p95
            payload = summary_payloads[path]
            slices = {"summary": payload}
            descriptors = [
                slice_descriptor("summary", payload, "publisher", default=True)
            ]
        else:
            record = entities[path]
            payload = record["fields"]
            slices = {
                "summary": {"fields": payload},
                "specs": record["specs"],
                "reviews": record["reviews"],
                "logistics": record["logistics"],
            }
            descriptors = [
                slice_descriptor("summary", slices["summary"], "publisher",
                                 default=True),
                slice_descriptor("specs", slices["specs"], "publisher"),
                slice_descriptor("reviews", slices["reviews"], "user_generated",
                                 moderated=True),
                slice_descriptor("logistics", slices["logistics"], "publisher",
                                 dynamic=True),
            ]
        passport["updated_at"] = UPDATED_AT
        passport["filters"] = filters
        passport["aggregations"] = {
            "supported": kind == "collection",
            "metrics": list(METRICS) if kind == "collection" else [],
            "group_by": list(GROUP_BY) if kind == "collection" else [],
            "min_group_size_default": 5,
        }
        passport["slices"] = descriptors
        passport["quality"] = 0.98
        passport["official"] = True
        passport["attribution"] = "required"
        passport["license"] = LICENSE
        node_docs[path] = {"passport": passport, "slices": slices}

    files = {
        "manifest.json": dump(build_manifest()),
        "tree.json": dump({"tree_generation": TREE_GENERATION,
                           "nodes": tree_nodes}),
    }
    for path in all_paths:
        files["nodes/a%d.json" % ranks[path]] = dump(node_docs[path])
    return files


def verify(files):
    for rel, blob in files.items():
        _check(all(byte < 128 for byte in blob), rel + ": non-ASCII byte")
    manifest = json.loads(files["manifest.json"])
    tree = json.loads(files["tree.json"])
    _check("$schema" not in manifest, "manifest must not carry $schema")
    _check(manifest["tree_generation"] == TREE_GENERATION, "manifest generation")
    _check(manifest["tokenizer"] == TOKENIZER, "manifest tokenizer")
    _check(manifest["features"]["aggregate"]["metrics"] == METRICS,
           "manifest metrics != passport metrics")
    _check(set(manifest["endpoints"]) == {
        "orient", "tree", "passport", "fetch", "estimate", "aggregate",
    }, "manifest endpoints")

    paths = [node["path"] for node in tree["nodes"]]
    _check(paths == sorted(paths), "tree nodes are not sorted by path")
    declared = set(paths)
    ranks = {path: i for i, path in enumerate(paths, 1)}
    parent_items = {}
    entities_seen = 0
    collections_seen = 0

    for index, node in enumerate(tree["nodes"], 1):
        path = node["path"]
        rel = "nodes/a%d.json" % index
        _check(node["anchor"] == "[a%d]" % index, rel + ": tree anchor != rank")
        _check(ranks[path] == index, rel + ": rank mismatch")
        doc = json.loads(files[rel])
        passport = doc["passport"]
        _check(passport["path"] == path, rel + ": passport path")
        _check(passport["anchor"] == node["anchor"], rel + ": passport anchor")
        _check(passport["kind"] == node["kind"], rel + ": kind mismatch")
        _check(passport["tree_generation"] == TREE_GENERATION, rel + ": generation")
        defaults = [s for s in passport["slices"] if s.get("default")]
        _check(len(defaults) == 1 and defaults[0]["name"] == "summary",
               rel + ": exactly one default slice (summary)")
        _check(len(passport["filters"]) >= 3, rel + ": filters")
        for descriptor in passport["slices"]:
            payload = doc["slices"][descriptor["name"]]
            blob = canonical_json(payload)
            _check(descriptor["bytes"] == len(blob), rel + ": bytes for " + descriptor["name"])
            _check(descriptor["tokens"] == math.ceil(len(blob) / 4),
                   rel + ": tokens for " + descriptor["name"])
            _check(descriptor["generation"]
                   == "V1:" + hashlib.sha256(blob).hexdigest()[:8],
                   rel + ": generation for " + descriptor["name"])
            _check(descriptor["provenance"] in
                   ("publisher", "user_generated", "third_party"),
                   rel + ": provenance")

        if node["kind"] == "collection":
            collections_seen += 1
            items = doc["slices"]["summary"]["items"]
            _check(list(doc["slices"]) == ["summary"], rel + ": collection slices")
            _check(passport["count"] == len(items) == node["count"],
                   rel + ": count")
            _check(node["has_children"] == (len(items) > 0), rel + ": has_children")
            _check("count" in passport, rel + ": collection must carry count")
            _check(passport["aggregations"]["supported"] is True,
                   rel + ": collections aggregate")
            _check(passport["aggregations"]["metrics"] == METRICS,
                   rel + ": collection metrics")
            parent_items[path] = {}
            for item in items:
                _check(item["path"] in declared, rel + ": unresolved item path")
                parent = item["path"].rsplit("/", 1)[0] or "/"
                _check(parent == path, rel + ": item is not a child: " + item["path"])
                parent_items[path][item["path"]] = item["fields"]
        else:
            entities_seen += 1
            _check("count" not in passport, rel + ": entity must not carry count")
            _check(sorted(doc["slices"]) ==
                   ["logistics", "reviews", "specs", "summary"],
                   rel + ": entity slices")
            _check(passport["aggregations"]["supported"] is False,
                   rel + ": entities do not aggregate")
            fields = doc["slices"]["summary"]["fields"]
            _check(fields["currency"] == "USD", rel + ": currency")
            _check(isinstance(fields["price"], float), rel + ": price float")
            _check(isinstance(fields["feature"], list) and fields["feature"],
                   rel + ": feature non-empty")
            _check(3.5 <= fields["rating"] <= 5.0, rel + ": rating range")
            _check(isinstance(fields["stock"], int), rel + ": stock int")
            reviews = doc["slices"]["reviews"]["fields"]
            _check(sum(reviews["histogram"]) == reviews["count"],
                   rel + ": histogram sum")
            _check(reviews["rating"] == fields["rating"], rel + ": rating agrees")
            parent = path.rsplit("/", 1)[0] or "/"
            _check(parent in parent_items and
                   parent_items[parent].get(path) == fields,
                   rel + ": missing or mismatched parent summary item")

    _check(entities_seen == 88, "expected 88 entities, got %d" % entities_seen)
    _check(collections_seen == 7, "expected 7 collections, got %d" % collections_seen)
    _check(len(files) == 97, "expected 97 files (2 + 95 nodes), got %d" % len(files))


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Generate the examples/ecommerce ADC reference dataset."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="regenerate into a temp dir and byte-compare against committed files",
    )
    args = parser.parse_args(argv)
    files = build_files()
    verify(files)
    here = Path(__file__).resolve().parent

    if args.check:
        drift = []
        with tempfile.TemporaryDirectory(prefix="adc-ecommerce-") as tmp:
            tmpdir = Path(tmp)
            for rel, blob in files.items():
                dest = tmpdir / rel
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(blob)
            for rel, blob in files.items():
                committed = here / rel
                if not committed.is_file():
                    drift.append(rel + " (missing)")
                elif committed.read_bytes() != blob:
                    drift.append(rel + " (content differs)")
            nodes_dir = here / "nodes"
            if nodes_dir.is_dir():
                expected = {rel.split("/", 1)[1] for rel in files
                            if rel.startswith("nodes/")}
                for stale in sorted(nodes_dir.glob("*.json")):
                    if stale.name not in expected:
                        drift.append("nodes/" + stale.name + " (stale)")
        if drift:
            for line in drift:
                print("drift: " + line, file=sys.stderr)
            return 1
        print("ok: %d files match the committed dataset" % len(files))
        return 0

    rewritten = 0
    for rel, blob in files.items():
        dest = here / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        if not dest.is_file() or dest.read_bytes() != blob:
            dest.write_bytes(blob)
            rewritten += 1
    print(
        "wrote %d files (%d rewritten): 95 nodes = 7 collections + 88 entities"
        % (len(files), rewritten)
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
