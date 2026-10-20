"""Request validation order tests (SPEC.md 6.10).

SPEC 6.10 fixes a normative order for rejecting a request:

  1. media type           -> 415
  2. payload size         -> 413
  3. JSON body structure  -> 400 INVALID_BODY
  4. anchor/path          -> 400 ANCHOR_INVALID / 404 ANCHOR_NOT_FOUND
  5. filters / metrics    -> 400 FILTER_INVALID
  6. tree generation      -> 409 GENERATION_STALE

Each test below feeds a request that violates two rules at once and asserts
that the earlier rule wins; the two rules map to distinct status codes or
error types, which makes the order observable.
"""

HEADPHONES = "/electronics/audio/headphones"
STALE_GENERATION = "1999-01-01T00:00:00Z"


def test_media_type_beats_json_structure(client):
    # Step 1 before step 3: malformed JSON sent as text/plain must be 415,
    # not 400. Head-only transmission: the server rejects on the header
    # before reading, and an unread body would race the response (RST).
    body = b"{not json"
    response = client.request_head(
        "/adc/fetch", content_type="text/plain", declared_length=len(body)
    )
    assert response.status == 415
    assert response.json["type"].endswith("/UNSUPPORTED_MEDIA_TYPE")


def test_payload_size_beats_json_structure(client):
    # Step 2 before step 3: an oversized body is 413, not 400. The server
    # validates Content-Length before reading (server.py _read_body), so the
    # head-only transmission below observes step 2 deterministically without
    # racing the connection reset that an unread oversized body would cause.
    response = client.post_oversized("/adc/fetch", declared_length=70000)
    assert response.status == 413
    assert response.json["type"].endswith("/PAYLOAD_TOO_LARGE")


def test_json_structure_beats_anchor_check(client):
    # Step 3 before step 4: syntactically invalid JSON never reaches
    # anchor parsing (which would say ANCHOR_INVALID).
    response = client.post("/adc/fetch", b'{"anchor": ')
    assert response.status == 400
    assert response.json["type"].endswith("/INVALID_BODY")


def test_post_body_must_be_an_object(client):
    # Step 3: a JSON array body is structurally invalid.
    response = client.post("/adc/fetch", b"[1, 2, 3]")
    assert response.status == 400
    assert response.json["type"].endswith("/INVALID_BODY")


def test_malformed_anchor_in_body_is_anchor_invalid(client):
    # Step 4 first half: bad anchor syntax is ANCHOR_INVALID, not
    # ANCHOR_NOT_FOUND (the anchor was never resolved).
    response = client.post_json("/adc/fetch", {"anchor": "[xyz]", "limit": 1})
    assert response.status == 400
    assert response.json["type"].endswith("/ANCHOR_INVALID")


def test_anchor_and_path_together_is_anchor_invalid(client):
    response = client.post_json("/adc/fetch", {"anchor": "[a30]", "path": "/"})
    assert response.status == 400
    assert response.json["type"].endswith("/ANCHOR_INVALID")


def test_anchor_resolution_beats_filter_validation(client):
    # Step 4 before step 5: an unknown anchor paired with an unknown filter
    # field must be 404, because resolution happens first.
    response = client.getq(
        "/adc/fetch", anchor="[a99999]", **{"filter[color]": "red"}
    )
    assert response.status == 404
    assert response.json["type"].endswith("/ANCHOR_NOT_FOUND")


def test_path_resolution_beats_filter_validation(client):
    response = client.getq(
        "/adc/fetch", path="/no/such/path", **{"filter[color]": "red"}
    )
    assert response.status == 404
    assert response.json["type"].endswith("/ANCHOR_NOT_FOUND")


def test_filter_validation_beats_generation_check(client):
    # Step 5 before step 6: an unknown filter field plus a stale generation
    # must be 400 FILTER_INVALID, not 409 GENERATION_STALE.
    response = client.getq(
        "/adc/fetch",
        path=HEADPHONES,
        tree_generation=STALE_GENERATION,
        **{"filter[color]": "red"},
    )
    assert response.status == 400
    assert response.json["type"].endswith("/FILTER_INVALID")


def test_undeclared_filter_op_beats_generation_check(client):
    response = client.getq(
        "/adc/fetch",
        path=HEADPHONES,
        tree_generation=STALE_GENERATION,
        **{"filter[brand][lt]": "Sony"},
    )
    assert response.status == 400
    assert response.json["type"].endswith("/FILTER_INVALID")


def test_metric_validation_beats_generation_check(client):
    # Step 5 applies to aggregate metrics as well: "median(price)" parses
    # but is not declared for this node.
    response = client.getq(
        "/adc/aggregate",
        path=HEADPHONES,
        metrics="median(price)",
        tree_generation=STALE_GENERATION,
    )
    assert response.status == 400
    assert response.json["type"].endswith("/METRIC_UNSUPPORTED")


def test_group_by_validation_beats_generation_check(client):
    response = client.getq(
        "/adc/aggregate",
        path=HEADPHONES,
        metrics="count()",
        group_by="model",
        tree_generation=STALE_GENERATION,
    )
    assert response.status == 400
    assert response.json["type"].endswith("/GROUP_BY_UNSUPPORTED")


def test_unknown_path_without_generation_is_404(client):
    # SPEC 6.10 precedence case 3: no generation was supplied, so an
    # unresolvable path is an unresolvable path.
    response = client.getq("/adc/fetch", path="/no/such/path")
    assert response.status == 404
    assert response.json["type"].endswith("/ANCHOR_NOT_FOUND")


def test_stale_generation_beats_unknown_path(client):
    # SPEC 6.10 precedence case 2 / 5.3.3: the same unknown path, but the
    # caller supplied a tree_generation that is not current. The anchor
    # table is provably out of date, so 409 GENERATION_STALE wins over 404 -
    # reporting "never existed" would be false.
    response = client.getq(
        "/adc/fetch", path="/no/such/path", tree_generation=STALE_GENERATION
    )
    assert response.status == 409
    assert response.json["type"].endswith("/GENERATION_STALE")
    assert response.json["details"]["current"] != STALE_GENERATION


def test_stale_generation_beats_unknown_anchor_on_post(client):
    # Same precedence on the POST profile.
    response = client.post_json(
        "/adc/fetch",
        {"anchor": "[a99999]", "tree_generation": STALE_GENERATION},
    )
    assert response.status == 409
    assert response.json["type"].endswith("/GENERATION_STALE")


def test_generation_check_beats_success(client):
    # Step 6: once everything earlier is valid, a stale generation yields
    # 409 GENERATION_STALE.
    response = client.getq(
        "/adc/fetch", path=HEADPHONES, tree_generation=STALE_GENERATION
    )
    assert response.status == 409
    assert response.json["type"].endswith("/GENERATION_STALE")


def test_generation_check_applies_to_estimate(client):
    response = client.getq(
        "/adc/estimate", path=HEADPHONES, tree_generation=STALE_GENERATION
    )
    assert response.status == 409
    assert response.json["type"].endswith("/GENERATION_STALE")


def test_post_filter_shape_is_filter_invalid(client):
    # A string filter body fails filter validation (step 5), after anchor
    # resolution succeeds.
    response = client.post_json(
        "/adc/fetch", {"anchor": "[a30]", "filter": "brand=x"}
    )
    assert response.status == 400
    assert response.json["type"].endswith("/FILTER_INVALID")


def test_post_anchor_resolution_beats_post_filter_shape(client):
    # Step 4 before step 5 for POST bodies too.
    response = client.post_json(
        "/adc/fetch", {"anchor": "[a99999]", "filter": "brand=x"}
    )
    assert response.status == 404
    assert response.json["type"].endswith("/ANCHOR_NOT_FOUND")


def test_post_generation_check_applies_too(client):
    response = client.post_json(
        "/adc/fetch",
        {"anchor": "[a30]", "tree_generation": STALE_GENERATION},
    )
    assert response.status == 409
    assert response.json["type"].endswith("/GENERATION_STALE")
