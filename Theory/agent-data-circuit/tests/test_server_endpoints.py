"""HTTP endpoint tests against the reference server
(SPEC.md 5.4, 6.1-6.5, 6.9, 6.10, 6.12, CONFORMANCE.md F.7, F.10, F.11, F.12)."""

import math

from conftest import query

HEADPHONES = "/electronics/audio/headphones"
AIRPODS = HEADPHONES + "/apple-airpods-3"
TREE_GENERATION = "1999-01-01T00:00:00Z"


# --------------------------------------------------------------------------
# Manifest and orient (SPEC 4.2, 6.1)
# --------------------------------------------------------------------------


def test_manifest_served_at_well_known(client):
    response = client.get("/.well-known/adc.json")
    assert response.status == 200
    body = response.json
    assert body["adc"] == "0.1"
    assert body["root"]
    assert body["tree_generation"]
    assert "application/adc+json" in response.header("Content-Type")


def test_manifest_declares_tokenizer_vocabulary(client):
    # SPEC 4.2: tokenizer MUST be one of the declared vocabulary entries.
    body = client.get("/.well-known/adc.json").json
    assert body["tokenizer"] == "utf8_bytes_div_4"


def test_manifest_carries_etag(client):
    response = client.get("/.well-known/adc.json")
    assert response.header("ETag")


def test_orient_returns_circuit_configuration(client):
    response = client.get("/adc/orient")
    assert response.status == 200
    body = response.json
    for key in ("adc", "root", "tree_generation", "tokenizer", "endpoints"):
        assert key in body
    assert body["tokenizer"] == "utf8_bytes_div_4"


def test_orient_cache_headers(client):
    # SPEC 6.1: Cache-Control public max-age=3600 plus a weak ETag.
    response = client.get("/adc/orient")
    assert response.header("Cache-Control") == "public, max-age=3600"
    assert response.header("ETag").startswith('W"')


def test_orient_rate_limit_headers_present(client):
    # SPEC 6.11: responses SHOULD carry RateLimit-* headers.
    response = client.get("/adc/orient")
    assert response.header("RateLimit-Limit") is not None
    assert response.header("RateLimit-Remaining") is not None


def test_error_responses_report_true_rate_limit_state(client):
    # SPEC 6.11: RateLimit-* describe the window, not the outcome of this one
    # request. An error page claiming Remaining: 0 would make an agent
    # throttle itself to a standstill after a single bad anchor.
    response = client.getq("/adc/fetch", path="/definitely/not/here")
    assert response.status == 404
    assert int(response.header("RateLimit-Remaining")) > 0


# --------------------------------------------------------------------------
# Tree (SPEC 5.4, 6.2)
# --------------------------------------------------------------------------


def test_tree_nodes_carry_required_shape(client):
    # SPEC 5.4: every node MUST include name, anchor, kind.
    response = client.getq("/adc/tree", path="/electronics/audio", depth=1)
    assert response.status == 200
    body = response.json
    assert body["path"] == "/electronics/audio"
    for node in body["nodes"]:
        assert node["name"]
        assert node["anchor"].startswith("[a")
        assert node["kind"] in ("collection", "entity", "document")
        assert isinstance(node["has_children"], bool)


def test_tree_nodes_sorted_deterministically(client):
    # SPEC 5.4: deterministic order, stable across pagination.
    body = client.getq("/adc/tree", path="/", depth=1).json
    paths = [node["path"] for node in body["nodes"]]
    assert paths == sorted(paths)


def test_tree_collections_include_count(client):
    body = client.getq("/adc/tree", path="/electronics/audio", depth=1).json
    headphones = [n for n in body["nodes"] if n["name"] == "headphones"][0]
    assert headphones["count"] == 48


def test_tree_echoes_tree_generation(client):
    orient = client.get("/adc/orient").json
    body = client.getq("/adc/tree", path="/").json
    assert body["tree_generation"] == orient["tree_generation"]


def test_tree_depth_over_max_limit_rejected(client):
    # SPEC 6.2: depth MUST NOT exceed limits.max_depth (4 in the example).
    response = client.getq("/adc/tree", path="/", depth=5)
    assert response.status == 400
    assert response.json["type"].endswith("/INVALID_BODY")


def test_tree_limit_over_max_limit_rejected(client):
    response = client.getq("/adc/tree", path="/", limit=999)
    assert response.status == 400
    assert response.json["type"].endswith("/INVALID_BODY")


def test_tree_unknown_path_is_404_with_details_path(client):
    # SPEC 6.2: unknown path is 404 ANCHOR_NOT_FOUND with details.path.
    response = client.getq("/adc/tree", path="/no/such/path")
    assert response.status == 404
    body = response.json
    assert body["type"].endswith("/ANCHOR_NOT_FOUND")
    assert body["details"]["path"] == "/no/such/path"


def test_tree_pagination_marks_truncated_and_cursor(client):
    body = client.getq("/adc/tree", path="/", depth=1, limit=1).json
    assert body["truncated"] is True
    assert body["cursor"]


def test_tree_cursor_reuse_returns_next_page(client):
    first = client.getq("/adc/tree", path="/", depth=1, limit=1).json
    second = client.getq(
        "/adc/tree", path="/", depth=1, limit=1, cursor=first["cursor"]
    ).json
    first_paths = [n["path"] for n in first["nodes"]]
    second_paths = [n["path"] for n in second["nodes"]]
    assert first_paths != second_paths
    assert not set(first_paths) & set(second_paths)
    combined = sorted(first_paths + second_paths)
    assert combined == ["/computers", "/electronics"]


def test_tree_corrupted_cursor_rejected(client):
    # SPEC 6.4.4: tampered cursors are 400 CURSOR_INVALID.
    response = client.getq(
        "/adc/tree", path="/", depth=1, limit=1, cursor="garbage!!"
    )
    assert response.status == 400
    assert response.json["type"].endswith("/CURSOR_INVALID")


def test_tree_cursor_is_bound_to_the_query(client):
    # SPEC 6.4.4: a cursor must be reused with an identical query.
    first = client.getq("/adc/tree", path="/", depth=1, limit=1).json
    response = client.getq(
        "/adc/tree", path="/", depth=1, limit=2, cursor=first["cursor"]
    )
    assert response.status == 400
    assert response.json["type"].endswith("/CURSOR_INVALID")


# --------------------------------------------------------------------------
# Passport (SPEC 6.3)
# --------------------------------------------------------------------------


def test_passport_by_path_and_by_anchor_agree(client):
    by_path = client.getq("/adc/passport", path=HEADPHONES)
    by_anchor = client.getq("/adc/passport", anchor="[a30]")
    assert by_path.status == 200
    assert by_anchor.status == 200
    assert by_path.json["anchor"] == by_anchor.json["anchor"] == "[a30]"
    assert by_path.json["path"] == by_anchor.json["path"] == HEADPHONES


def test_passport_declares_exactly_one_default_slice(client):
    # SPEC 6.3: exactly one slice MUST carry "default": true.
    body = client.getq("/adc/passport", path=AIRPODS).json
    defaults = [s for s in body["slices"] if s.get("default")]
    assert len(defaults) == 1
    assert defaults[0]["name"] == "summary"


def test_passport_declares_filters_with_ops(client):
    # SPEC 6.3: filters MUST include name, type and ops.
    body = client.getq("/adc/passport", path=HEADPHONES).json
    by_name = {f["name"]: f for f in body["filters"]}
    assert {"brand", "price", "feature"} <= set(by_name)
    for descriptor in by_name.values():
        assert descriptor["type"] in (
            "string",
            "number",
            "boolean",
            "date",
            "enum",
        )
        assert descriptor["ops"]


def test_passport_carries_etag(client):
    response = client.getq("/adc/passport", path=HEADPHONES)
    assert response.header("ETag")


def test_passport_requires_one_of_anchor_or_path(client):
    neither = client.get("/adc/passport")
    assert neither.status == 400
    assert neither.json["type"].endswith("/ANCHOR_INVALID")
    both = client.getq("/adc/passport", anchor="[a30]", path="/")
    assert both.status == 400
    assert both.json["type"].endswith("/ANCHOR_INVALID")


# --------------------------------------------------------------------------
# Fetch GET (SPEC 6.4.1, 6.4.2, CONFORMANCE F.7)
# --------------------------------------------------------------------------


def test_collection_fetch_shape(client):
    # CONFORMANCE F.7.1.
    body = client.getq("/adc/fetch", path=HEADPHONES).json
    for key in ("parent_anchor", "items", "cursor", "truncated"):
        assert key in body
    # SPEC 6.4.2: entity responses carry top-level anchor/path/generation/
    # fields; a collection response must not. ("slice" IS present: the
    # SPEC 6.4.2 collection example includes "slice": "summary".)
    for key in ("anchor", "path", "generation", "fields"):
        assert key not in body
    assert body["parent_anchor"] == "[a30]"


def test_entity_fetch_shape(client):
    # CONFORMANCE F.7.2.
    body = client.getq("/adc/fetch", path=AIRPODS).json
    for key in ("anchor", "path", "slice", "generation", "fields"):
        assert key in body
    for key in ("items", "parent_anchor"):
        assert key not in body
    assert body["anchor"] == "[a31]"
    assert body["fields"]["brand"] == "Apple"


def test_default_slice_used_when_omitted(client):
    # CONFORMANCE F.7.3.
    passport = client.getq("/adc/passport", path=AIRPODS).json
    default_name = [s["name"] for s in passport["slices"] if s.get("default")][0]
    body = client.getq("/adc/fetch", path=AIRPODS).json
    assert body["slice"] == default_name


def test_fetch_slice_param_selects_named_slice(client):
    response = client.getq("/adc/fetch", path=AIRPODS, slice="specs")
    assert response.status == 200
    body = response.json
    assert body["slice"] == "specs"
    default = client.getq("/adc/fetch", path=AIRPODS).json
    assert body["generation"] != default["generation"]


def test_fetch_unknown_slice_is_404(client):
    response = client.getq("/adc/fetch", path=AIRPODS, slice="nosuch")
    assert response.status == 404
    assert response.json["type"].endswith("/SLICE_NOT_FOUND")
    assert response.json["details"]["available"]


def test_fetch_fields_projection_limits_fields(client):
    # SPEC 6.4.1: fields is a comma-separated list of dot paths.
    body = client.getq(
        "/adc/fetch", path=HEADPHONES, fields="brand,price", limit=3
    ).json
    for item in body["items"]:
        assert sorted(item["fields"]) == ["brand", "price"]
        assert item["anchor"]
        assert item["generation"]


def test_fetch_eq_filter(client):
    body = client.getq("/adc/fetch", path=HEADPHONES, **{"filter[brand]": "Sony"}).json
    brands = [item["fields"]["brand"] for item in body["items"]]
    assert brands
    assert set(brands) == {"Sony"}


def test_fetch_range_filter_ands_gte_with_lte(client):
    body = client.getq(
        "/adc/fetch",
        path=HEADPHONES,
        **{"filter[price][gte]": "300", "filter[price][lte]": "400"},
    ).json
    prices = [item["fields"]["price"] for item in body["items"]]
    assert prices
    assert all(300.0 <= price <= 400.0 for price in prices)


def test_fetch_repeated_in_filter_is_not_comma_split(client):
    # SPEC 6.4.1 / CONFORMANCE F.5.1: repeated parameters form the list.
    response = client.getq(
        "/adc/fetch", path=HEADPHONES, **{"filter[brand][in]": ["Sony", "Apple"]}
    )
    body = response.json
    brands = {item["fields"]["brand"] for item in body["items"]}
    assert {"Sony", "Apple"} <= brands


def test_fetch_comma_list_value_matches_nothing(client):
    # SPEC 6.4.1: comma-separated lists are NOT used, so "Sony,Apple" is a
    # single literal value.
    body = client.getq(
        "/adc/fetch", path=HEADPHONES, **{"filter[brand][in]": "Sony,Apple"}
    ).json
    assert body["items"] == []
    assert body["truncated"] is False


def test_fetch_unknown_filter_field_is_400(client):
    response = client.getq(
        "/adc/fetch", path=HEADPHONES, **{"filter[color]": "red"}
    )
    assert response.status == 400
    body = response.json
    assert body["type"].endswith("/FILTER_INVALID")
    assert body["details"]["unknown_fields"] == ["color"]


def test_fetch_undeclared_filter_op_is_400(client):
    response = client.getq(
        "/adc/fetch", path=HEADPHONES, **{"filter[brand][lt]": "1"}
    )
    assert response.status == 400
    body = response.json
    assert body["type"].endswith("/FILTER_INVALID")
    assert body["details"]["unsupported_ops"] == ["lt"]


def test_fetch_order_by_is_deterministic(client):
    # CONFORMANCE F.7.6.
    first = client.getq(
        "/adc/fetch", path=HEADPHONES, order_by="-price", limit=5
    ).json
    second = client.getq(
        "/adc/fetch", path=HEADPHONES, order_by="-price", limit=5
    ).json
    first_prices = [i["fields"]["price"] for i in first["items"]]
    second_prices = [i["fields"]["price"] for i in second["items"]]
    assert first_prices == second_prices
    assert first_prices == sorted(first_prices, reverse=True)


def test_fetch_limit_over_max_is_rejected(client):
    # CONFORMANCE F.7.4: limit above the server maximum is rejected.
    response = client.getq("/adc/fetch", path=HEADPHONES, limit=999)
    assert response.status == 400


def test_collection_fetch_pagination_cursor_reuse(client):
    first = client.getq("/adc/fetch", path=HEADPHONES, limit=5).json
    assert first["truncated"] is True
    second = client.getq(
        "/adc/fetch", path=HEADPHONES, limit=5, cursor=first["cursor"]
    ).json
    first_paths = [i["path"] for i in first["items"]]
    second_paths = [i["path"] for i in second["items"]]
    assert len(second_paths) == 5
    assert not set(first_paths) & set(second_paths)


def test_collection_fetch_corrupted_cursor_rejected(client):
    first = client.getq("/adc/fetch", path=HEADPHONES, limit=5).json
    cursor = first["cursor"]
    tampered = ("B" if cursor[0] != "B" else "C") + cursor[1:]
    response = client.getq(
        "/adc/fetch", path=HEADPHONES, limit=5, cursor=tampered
    )
    assert response.status == 400
    assert response.json["type"].endswith("/CURSOR_INVALID")


def test_fetch_dynamic_slice_is_live_and_billed_on_healthy_server(client):
    # SPEC 6.4.3/6.4.6: live dynamic slice reports live hydration; a charged
    # dynamic response must be private, no-store.
    response = client.getq("/adc/fetch", path=AIRPODS, slice="logistics")
    assert response.status == 200
    body = response.json
    hydration = body["hydration"]["logistics"]
    assert hydration["status"] == "live"
    assert hydration["staleness_seconds"] == 0
    assert body["usage"]["cost"]["amount"] > 0
    assert "no-store" in response.header("Cache-Control")


def test_fetch_static_slice_is_cacheable_with_zero_cost(client):
    response = client.getq("/adc/fetch", path=AIRPODS, slice="summary")
    assert response.status == 200
    assert response.header("ADC-Cost-Amount") == "0"
    assert response.header("Cache-Control").startswith("public")


# --------------------------------------------------------------------------
# Fetch POST (SPEC 6.4.5, CONFORMANCE F.10.5)
# --------------------------------------------------------------------------


def test_post_fetch_or_filter(client):
    body = client.post_json(
        "/adc/fetch",
        {
            "anchor": "[a30]",
            "filter": {
                "or": [
                    {"brand": {"eq": "Sony"}},
                    {"brand": {"eq": "Apple"}},
                ]
            },
            "limit": 50,
        },
    )
    assert body.status == 200
    brands = {i["fields"]["brand"] for i in body.json["items"]}
    assert brands
    assert brands <= {"Sony", "Apple"}


def test_post_fetch_and_not_filters(client):
    body = client.post_json(
        "/adc/fetch",
        {
            "anchor": "[a30]",
            "filter": {
                "and": [
                    {"not": {"brand": {"eq": "Sony"}}},
                    {"price": {"gte": 300}},
                ]
            },
            "limit": 50,
        },
    )
    assert body.status == 200
    for item in body.json["items"]:
        assert item["fields"]["brand"] != "Sony"
        assert item["fields"]["price"] >= 300


def test_post_fetch_is_no_store_without_etag(client):
    # SPEC 6.4.5 / CONFORMANCE F.10.5.
    response = client.post_json("/adc/fetch", {"anchor": "[a30]", "limit": 2})
    assert response.status == 200
    assert "no-store" in response.header("Cache-Control")
    assert response.header("ETag") is None


def test_post_fetch_rejects_wrong_media_type(client):
    # SPEC 6.10 step 1: media type is validated first (415). Head-only: the
    # server rejects before reading the body, and an unread body would make
    # the closing RST race the 415 response (WinError 10053).
    body = b'{"anchor": "[a30]"}'
    response = client.request_head(
        "/adc/fetch", content_type="text/plain", declared_length=len(body)
    )
    assert response.status == 415
    assert response.json["type"].endswith("/UNSUPPORTED_MEDIA_TYPE")


def test_post_fetch_rejects_oversized_body(client):
    # SPEC 6.10 step 1 / 6.4.5: 413 PAYLOAD_TOO_LARGE. The server checks the
    # declared Content-Length before reading, so the head-only request below
    # is the deterministic way to observe the rejection.
    response = client.post_oversized("/adc/fetch", declared_length=70000)
    assert response.status == 413
    assert response.json["type"].endswith("/PAYLOAD_TOO_LARGE")
    assert response.json["retryable"] is False


def test_post_fetch_rejects_malformed_json(client):
    response = client.post("/adc/fetch", b"{not json")
    assert response.status == 400
    assert response.json["type"].endswith("/INVALID_BODY")


# --------------------------------------------------------------------------
# Estimate (SPEC 6.5, CONFORMANCE F.12.3)
# --------------------------------------------------------------------------


def test_estimate_collection_structure(client):
    response = client.getq("/adc/estimate", path=HEADPHONES, limit=10)
    assert response.status == 200
    body = response.json
    assert body["count"] == 48
    assert body["count_exact"] is True
    assert body["items_returned"] == 10
    assert body["estimated_tokens"] > 0
    assert body["confidence"] in ("exact", "bounded", "sample")
    assert body["breakdown"]


def test_estimate_applies_filters(client):
    body = client.getq("/adc/estimate", path=HEADPHONES, **{"filter[brand]": "Sony"}).json
    assert body["count"] == 6


def test_estimate_does_not_execute_hydration(client):
    # SPEC 6.5: estimate MUST NOT execute hydration.
    response = client.getq("/adc/estimate", path=AIRPODS, slice="logistics")
    assert response.status == 200
    assert "hydration" not in response.json
    assert response.json["dynamic"] is True


def test_exact_estimate_equals_fetch_usage(client):
    # CONFORMANCE F.12.3: exact estimates must equal usage.
    estimate = client.getq("/adc/estimate", path=AIRPODS, slice="specs").json
    fetch = client.getq("/adc/fetch", path=AIRPODS, slice="specs").json
    assert estimate["confidence"] == "exact"
    assert estimate["estimated_tokens"] == fetch["usage"]["tokens"]


# --------------------------------------------------------------------------
# Aggregate endpoint (SPEC 6.9)
# --------------------------------------------------------------------------


def test_aggregate_grouped_response_shape(client):
    response = client.getq(
        "/adc/aggregate", path=HEADPHONES, metrics="count()", group_by="brand"
    )
    assert response.status == 200
    body = response.json
    assert body["groups"]
    assert body["metrics"] == [{"expr": "count()", "alias": "count_all"}]
    assert body["generation"].startswith("AGG:")
    assert body["computed_at"]
    assert body["usage"]["tokens"] > 0
    assert "values" not in body


def test_aggregate_ungrouped_response_shape(client):
    response = client.getq("/adc/aggregate", path=HEADPHONES, metrics="count()")
    body = response.json
    assert body["values"] == {"count_all": 48}
    assert "groups" not in body


def test_aggregate_metric_not_declared_is_400(client):
    # SPEC 6.9: metrics must come from the passport declaration;
    # "median(price)" parses but is not declared for this node.
    response = client.getq(
        "/adc/aggregate", path=HEADPHONES, metrics="median(price)"
    )
    assert response.status == 400
    assert response.json["type"].endswith("/METRIC_UNSUPPORTED")
    assert response.json["details"]["unsupported"] == ["median(price)"]


def test_aggregate_group_by_not_declared_is_400(client):
    response = client.getq(
        "/adc/aggregate", path=HEADPHONES, metrics="count()", group_by="model"
    )
    assert response.status == 400
    assert response.json["type"].endswith("/GROUP_BY_UNSUPPORTED")


def test_post_aggregate_is_no_store(client):
    # SPEC 6.9 POST shares the no-store rule of 6.4.5.
    response = client.post_json(
        "/adc/aggregate",
        {"anchor": "[a30]", "metrics": [{"expr": "count()", "alias": "n"}]},
    )
    assert response.status == 200
    assert "no-store" in response.header("Cache-Control")
    assert response.json["values"]["n"] == 48


def test_aggregate_on_entity_without_aggregations_is_rejected(client):
    response = client.getq("/adc/aggregate", path=AIRPODS, metrics="count()")
    assert response.status == 400
    assert response.json["type"].endswith("/AGGREGATE_UNSUPPORTED")


# --------------------------------------------------------------------------
# Errors and method availability (SPEC 6.10, 6.12, CONFORMANCE F.11)
# --------------------------------------------------------------------------


def test_unknown_anchor_is_404(client):
    response = client.getq("/adc/passport", anchor="[a99999]")
    assert response.status == 404
    assert response.json["type"].endswith("/ANCHOR_NOT_FOUND")


def test_malformed_anchor_is_400(client):
    response = client.getq("/adc/passport", anchor="not-an-anchor")
    assert response.status == 400
    assert response.json["type"].endswith("/ANCHOR_INVALID")


def test_search_endpoint_is_501_not_404(client):
    # SPEC 6: a method outside the declared tiers must be 501
    # NOT_IMPLEMENTED, not a generic 404.
    response = client.get("/adc/search?query=anything")
    assert response.status == 501
    assert response.json["type"].endswith("/NOT_IMPLEMENTED")


def test_unsupported_method_is_501(client):
    # Head-only again: _reject_method answers before consuming the body.
    response = client.request_head(
        "/adc/fetch", method="PUT", content_type="application/json",
        declared_length=2,
    )
    assert response.status == 501
    assert response.json["type"].endswith("/NOT_IMPLEMENTED")


def test_problem_details_shape_is_rfc9457(client):
    # SPEC 6.12 / CONFORMANCE F.11.1.
    response = client.getq("/adc/passport", anchor="[a99999]")
    assert "application/problem+json" in response.header("Content-Type")
    body = response.json
    assert body["type"] == (
        "https://agentdatacircuit.org/errors/ANCHOR_NOT_FOUND"
    )
    assert body["title"]
    assert body["status"] == 404
    assert body["detail"]
    assert body["instance"]
    assert body["retryable"] is False
    assert isinstance(body["details"], dict)
    assert body["tree_generation"]


def test_generation_stale_is_retryable_refresh(client):
    # SPEC 6.12: GENERATION_STALE is 409 and retryable (refresh).
    response = client.getq(
        "/adc/fetch", path=HEADPHONES, tree_generation=TREE_GENERATION
    )
    assert response.status == 409
    body = response.json
    assert body["type"].endswith("/GENERATION_STALE")
    assert body["retryable"] is True
    assert body["details"]["current"]


# --------------------------------------------------------------------------
# Cross-cutting response headers (SPEC 6, 6.4.7)
# --------------------------------------------------------------------------


def _tree_generation_paths():
    return [
        "/.well-known/adc.json",
        "/adc/orient",
        "/adc/tree?path=/",
        "/adc/passport" + query(path=HEADPHONES),
        "/adc/fetch" + query(path=HEADPHONES),
        "/adc/fetch" + query(path=HEADPHONES, limit=1),
        "/adc/fetch" + query(anchor="[a31]"),
        "/adc/fetch" + query(path=AIRPODS),
        "/adc/estimate" + query(path=HEADPHONES),
        "/adc/aggregate" + query(path=HEADPHONES, metrics="count()"),
        "/adc/passport?anchor=%5Ba99999%5D",
        "/adc/search?query=x",
    ]


def test_every_adc_response_carries_tree_generation_header(client):
    # SPEC 6: "All ADC responses MUST include ADC-Tree-Generation".
    orient = client.get("/adc/orient").json["tree_generation"]
    for path in _tree_generation_paths():
        response = client.get(path)
        assert response.header("ADC-Tree-Generation") == orient, path


def test_vary_header_present_on_responses(client):
    # SPEC 6 media types / CONFORMANCE F.10.4.
    response = client.get("/adc/orient")
    vary = response.header("Vary")
    assert vary and "Accept" in vary and "Authorization" in vary


def test_every_fetch_response_carries_usage(client):
    # SPEC 6.4.7: every successful fetch MUST include usage with tokens
    # and items.
    cases = [
        query(path=HEADPHONES),
        query(path=AIRPODS),
        query(path=AIRPODS, slice="specs"),
        query(path=HEADPHONES, **{"filter[brand]": "Sony"}),
    ]
    for suffix in cases:
        response = client.get("/adc/fetch" + suffix)
        assert response.status == 200, suffix
        usage = response.json["usage"]
        assert "tokens" in usage, suffix
        assert "items" in usage, suffix
    post = client.post_json("/adc/fetch", {"anchor": "[a31]"})
    assert "tokens" in post.json["usage"]
    assert "items" in post.json["usage"]


def test_usage_tokens_follow_utf8_bytes_div_4(client):
    # CONFORMANCE F.12.2: usage.tokens == ceil(utf8_bytes / 4).
    body = client.getq("/adc/fetch", path=HEADPHONES).json
    usage = body["usage"]
    assert usage["tokens"] == math.ceil(usage["bytes"] / 4)
    entity = client.getq("/adc/fetch", path=AIRPODS).json
    assert entity["usage"]["tokens"] == math.ceil(entity["usage"]["bytes"] / 4)


def test_fetch_usage_items_counts_returned_rows(client):
    body = client.getq("/adc/fetch", path=HEADPHONES, limit=7).json
    assert body["usage"]["items"] == len(body["items"]) == 7
