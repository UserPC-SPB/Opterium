"""Dynamic slice degradation tests (SPEC.md 7.3, 6.4.3, CONFORMANCE.md F.8).

Two extra servers back these tests (see conftest):

* ``stale_client``   - hydration fails but staleness (612s) stays within the
  passport max_staleness_seconds (900s)  -> ``stale_fallback``, billed zero.
* ``overstale_client`` - staleness (1000s) exceeds the maximum  -> the server
  MUST NOT serve stale_fallback; it fails closed with 503 HYDRATION_FAILED.

The healthy ``client`` proves degradation is scoped: static slices and the
rest of the tree behave identically everywhere.
"""

import pytest

AIRPODS = "/electronics/audio/headphones/apple-airpods-3"


def test_stale_fallback_served_within_max_staleness(stale_client):
    # SPEC 7.3: stale_fallback is allowed only while
    # staleness_seconds <= max_staleness_seconds.
    response = stale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    )
    assert response.status == 200
    hydration = response.json["hydration"]["logistics"]
    assert hydration["status"] == "stale_fallback"
    assert hydration["staleness_seconds"] == 612
    assert hydration["max_staleness_seconds"] == 900
    assert hydration["staleness_seconds"] <= hydration["max_staleness_seconds"]
    assert hydration["revalidation_required"] is True
    assert hydration["failure_reason"]


def test_stale_fallback_discloses_age_and_reason(stale_client):
    # SPEC 7.3: stale consumption must disclose age and failure reason.
    hydration = stale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    ).json["hydration"]["logistics"]
    assert hydration["hydrated_at"]
    assert isinstance(hydration["staleness_seconds"], int)


def test_degraded_slice_billed_zero(stale_client):
    # CONFORMANCE F.8.3: billing cost for a degraded slice MUST be zero.
    response = stale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    )
    assert response.status == 200
    cost = response.json["usage"]["cost"]
    assert cost["amount"] == 0.0
    assert cost["currency"] == "USD"
    assert response.header("ADC-Cost-Amount") == "0"


def test_degraded_response_is_private_no_store(stale_client):
    # SPEC 6.4.3: dynamic responses must not be publicly cached.
    response = stale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    )
    assert "no-store" in response.header("Cache-Control")
    assert response.header("ETag") is None


def test_degraded_response_still_carries_usage_and_generation(stale_client):
    # SPEC 6.4.7 usage and the ADC-Tree-Generation header are mandatory on
    # every successful response, degraded or not.
    response = stale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    )
    assert response.header("ADC-Tree-Generation")
    assert "tokens" in response.json["usage"]
    assert "items" in response.json["usage"]


def test_static_slice_unaffected_on_degraded_server(stale_client):
    # Degradation is scoped to the failing dynamic slice (SPEC 7.3).
    response = stale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=summary"
    )
    assert response.status == 200
    hydration = response.json.get("hydration") or {}
    assert "logistics" not in hydration
    assert response.header("ADC-Cost-Amount") == "0"
    assert response.header("Cache-Control").startswith("public")


def test_tree_and_orient_unaffected_on_degraded_server(stale_client):
    # SPEC 7.3: only hydration degrades; static tree serving is independent.
    orient = stale_client.get("/adc/orient")
    assert orient.status == 200
    tree = stale_client.get("/adc/tree?path=/electronics/audio&depth=1")
    assert tree.status == 200
    assert tree.json["nodes"]


def test_estimate_never_hydrates_on_degraded_server(stale_client):
    # SPEC 6.5: estimate MUST NOT execute hydration, so it must stay 200
    # with no hydration key even while the upstream is failing.
    response = stale_client.get(
        "/adc/estimate?path=" + AIRPODS + "&slice=logistics"
    )
    assert response.status == 200
    assert response.json["dynamic"] is True
    assert "hydration" not in response.json


def test_overstale_is_503_hydration_failed(overstale_client):
    # SPEC 7.3: beyond max_staleness_seconds the server MUST NOT return
    # stale_fallback; it MUST apply fail_closed (503 HYDRATION_FAILED).
    response = overstale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    )
    assert response.status == 503
    body = response.json
    assert body["type"].endswith("/HYDRATION_FAILED")
    assert body["title"] == "Hydration Failed"
    assert body["status"] == 503
    assert body["retryable"] is True
    assert body["instance"]


def test_overstale_fail_closed_discloses_exceeded_maximum(overstale_client):
    response = overstale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    )
    details = response.json["details"]
    assert details["slice"] == "logistics"
    assert details["staleness_seconds"] == 1000
    assert details["max_staleness_seconds"] == 900
    assert details["staleness_seconds"] > details["max_staleness_seconds"]
    assert response.header("Cache-Control") == "no-store"
    assert response.header("ADC-Tree-Generation")


def test_overstale_still_serves_static_slices(overstale_client):
    # SPEC 7.3 fail_closed aborts only the hydration-dependent fetch.
    response = overstale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=summary"
    )
    assert response.status == 200
    assert response.json["usage"]["cost"]["amount"] == 0.0


def test_overstale_estimate_still_exact(overstale_client):
    response = overstale_client.get(
        "/adc/estimate?path=" + AIRPODS + "&slice=logistics"
    )
    assert response.status == 200
    assert "hydration" not in response.json


def test_overstale_503_carries_retry_after(overstale_client):
    response = overstale_client.get(
        "/adc/fetch?path=" + AIRPODS + "&slice=logistics"
    )
    assert response.status == 503
    assert response.header("Retry-After") is not None
