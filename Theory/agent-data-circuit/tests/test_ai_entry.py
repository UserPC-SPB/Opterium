"""Cold entry (/AI/) tests - AI-ENTRY.md section 11.

The bundled site ships `ai.json`; the `noai_client` fixture serves a copy
without it, which is the absence case every agent must be able to survive.
"""

import json
import shutil

from adc.site import AI_KEYS, check_site


def test_ai_map_is_served(client):
    response = client.get("/AI/")
    assert response.status == 200
    assert response.header("content-type") == "application/json; charset=utf-8"
    payload = response.json
    assert payload["ai"] == "1.0"
    assert payload["vars"]["h1"] == "/electronics/audio/headphones"
    assert payload["vars"]["p1"].endswith("sony-wh-1000xm5")
    assert payload["keys"] == ["text", "meta", "price", "stock"]
    assert set(payload["keys"]) <= set(AI_KEYS)
    assert response.header("cache-control") == "public, max-age=3600"


def test_ai_map_without_trailing_slash(client):
    assert client.get("/AI").status == 200


def test_ai_map_without_ai_json_is_problem_details(noai_client):
    # AI-ENTRY.md section 7: not a tunnel map -> the agent falls back.
    response = noai_client.get("/AI/")
    assert response.status == 404
    assert response.header("content-type") == "application/problem+json; charset=utf-8"
    payload = response.json
    assert payload["status"] == 404
    assert "vars" not in payload
    assert "keys" not in payload

    probe = noai_client.get("/AI[h1]")
    assert probe.status == 404
    assert "vars" not in probe.json


def test_ai_variable_defaults_to_text(client):
    response = client.get("/AI[h1]")
    assert response.status == 200
    payload = response.json
    assert payload["get"] == "text"
    assert payload["path"] == "/electronics/audio/headphones"
    assert payload["name"] == "headphones"
    assert payload["text"].startswith("headphones (48)")


def test_ai_unknown_variable_lists_available(client):
    response = client.get("/AI[zz]")
    assert response.status == 404
    assert response.header("content-type") == "application/json; charset=utf-8"
    assert response.json == {
        "error": "unknown_variable",
        "available": ["c1", "c2", "e1", "h1", "s1", "p1"],
    }


def test_ai_key_unavailable_on_collection_lists_text_and_meta(client):
    response = client.get("/AI[h1]?get=price")
    assert response.status == 400
    assert response.json == {
        "error": "key_unavailable",
        "available": ["text", "meta"],
    }


def test_ai_key_unavailable_for_vocabulary_key_not_declared(client):
    # "image" is in the vocabulary (AI-ENTRY.md section 5) but this dataset
    # cannot produce it, so the site never declares it.
    response = client.get("/AI[p1]?get=image")
    assert response.status == 400
    payload = response.json
    assert payload["error"] == "key_unavailable"
    assert "image" not in payload["available"]


def test_ai_entity_dynamic_keys(client):
    price = client.get("/AI[p1]?get=price")
    assert price.status == 200
    assert price.json["get"] == "price"
    assert price.json["currency"] == "USD"
    assert isinstance(price.json["price"], (int, float))

    stock = client.get("/AI[p1]?get=stock")
    assert stock.status == 200
    assert stock.json["get"] == "stock"
    assert isinstance(stock.json["stock"], int)

    meta = client.get("/AI[p1]?get=meta")
    assert meta.status == 200
    assert meta.json["kind"] == "entity"
    assert meta.json["anchor"].startswith("[a")
    assert meta.json["updated_at"]


def test_ai_collection_field_projection(client):
    response = client.get("/AI[h1]?get=text&fields=brand,model&limit=3")
    assert response.status == 200
    payload = response.json
    assert payload["get"] == "text"
    assert payload["fields"] == ["brand", "model"]
    assert payload["count"] == 3
    assert len(payload["items"]) == 3
    for row in payload["items"]:
        assert set(row) == {"brand", "model"}
    assert "text" not in payload


def test_ai_limit_truncates_collection_names(client):
    # AI-ENTRY.md section 11: the total count stays, only N names are listed.
    response = client.get("/AI[h1]?get=text&limit=2")
    assert response.status == 200
    payload = response.json
    assert payload["text"].startswith("headphones (48): ")
    listed = payload["text"].split(": ", 1)[1]
    assert len(listed.split(", ")) == 2


def test_ai_entity_fields_projection(client):
    response = client.get("/AI[p1]?get=text&fields=brand,price")
    assert response.status == 200
    payload = response.json
    lines = payload["text"].splitlines()
    assert lines[0].startswith("brand: ")
    assert any(line.startswith("price: ") for line in lines)
    assert not any(line.startswith("model: ") for line in lines)


def test_ai_invalid_limit_is_a_tunnel_error(client):
    response = client.get("/AI[h1]?get=text&limit=abc")
    assert response.status == 400
    payload = response.json
    assert payload["error"] == "invalid_parameter"
    assert payload["parameter"] == "limit"
    assert "integer" in payload["detail"]


def test_ai_invalid_fields_is_a_tunnel_error(client):
    response = client.get("/AI[h1]?get=text&fields=")
    assert response.status == 400
    payload = response.json
    assert payload["error"] == "invalid_parameter"
    assert payload["parameter"] == "fields"


def test_ai_limit_rejected_outside_text(client):
    response = client.get("/AI[p1]?get=meta&limit=1")
    assert response.status == 400
    payload = response.json
    assert payload["error"] == "invalid_parameter"
    assert payload["parameter"] == "limit"


def test_ai_post_is_not_part_of_the_entry(client):
    # The handler rejects the method before reading the body, so the head
    # is sent without it (Windows would RST an already-sent response).
    response = client.request_head("/AI/", method="POST", declared_length=2)
    assert response.status == 501
    assert response.json["title"] == "Not Implemented"


def test_adc_endpoints_unaffected_by_cold_entry(client):
    assert client.get("/adc/orient").status == 200
    assert client.get("/.well-known/adc.json").status == 200
    assert client.get("/definitely/not/an/endpoint").status == 404


# -- site-level validation (validator CLI goes through check_site) -------


def _copy_site(example_site_dir, tmp_path):
    target = tmp_path / "site"
    shutil.copytree(example_site_dir, target)
    return target


def test_check_site_accepts_bundled_entry(example_site_dir):
    assert check_site(example_site_dir) == []


def test_check_site_rejects_unknown_var_target(example_site_dir, tmp_path):
    target = _copy_site(example_site_dir, tmp_path)
    (target / "ai.json").write_text(
        json.dumps({"ai": "1.0", "vars": {"x": "/nope"}, "keys": ["text"]}),
        encoding="utf-8",
    )
    problems = check_site(target)
    assert any("points at unknown path" in problem for problem in problems)


def test_check_site_rejects_unknown_key(example_site_dir, tmp_path):
    target = _copy_site(example_site_dir, tmp_path)
    (target / "ai.json").write_text(
        json.dumps({"ai": "1.0", "vars": {"x": "/"}, "keys": ["text", "clickbait"]}),
        encoding="utf-8",
    )
    problems = check_site(target)
    assert any("unknown key 'clickbait'" in problem for problem in problems)


def test_check_site_rejects_entry_without_version(example_site_dir, tmp_path):
    target = _copy_site(example_site_dir, tmp_path)
    (target / "ai.json").write_text(
        json.dumps({"vars": {"x": "/"}, "keys": ["text"]}),
        encoding="utf-8",
    )
    problems = check_site(target)
    assert any(
        "INVALID_BODY" in problem and "version string" in problem
        for problem in problems
    )
