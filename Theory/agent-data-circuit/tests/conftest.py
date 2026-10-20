"""Shared fixtures for the ADC independent test suite.

Servers bind to loopback only (127.0.0.1) on fixed high ports, one server
per fixture scope. The environment switches that silence request logging and
disable rate limiting are set before ``adc.server`` is imported.
"""

import os

os.environ["ADC_QUIET"] = "1"
os.environ["ADC_NO_RATE_LIMIT"] = "1"

import json
import socket
import threading
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlencode, urlsplit

import pytest

from adc.site import Site

REPO_ROOT = Path(__file__).resolve().parents[1]
EXAMPLE_SITE = REPO_ROOT / "examples" / "ecommerce"

MAIN_PORT = 8231
STALE_PORT = 8232
OVERSTALE_PORT = 8233
NOAI_PORT = 8234


class ApiResponse:
    """One HTTP response with case-insensitive header access."""

    def __init__(self, status, headers, body):
        self.status = status
        self.headers = {key.lower(): value for key, value in headers.items()}
        self.body = body

    @property
    def json(self):
        return json.loads(self.body.decode("utf-8"))

    def header(self, name):
        return self.headers.get(name.lower())


class Client:
    """Minimal loopback HTTP client built on urllib."""

    def __init__(self, base_url):
        self.base_url = base_url.rstrip("/")

    def get(self, path, timeout=10.0):
        request = urllib.request.Request(self.base_url + path)
        return self._send(request, timeout)

    def getq(self, endpoint, **params):
        """GET endpoint with a properly encoded query string."""
        return self.get(endpoint + query(**params))

    def post(self, path, body, content_type="application/adc+json", timeout=10.0):
        request = urllib.request.Request(self.base_url + path, data=body, method="POST")
        request.add_header("Content-Type", content_type)
        return self._send(request, timeout)

    def post_json(self, path, payload, content_type="application/adc+json"):
        body = json.dumps(payload).encode("utf-8")
        return self.post(path, body, content_type=content_type)

    @staticmethod
    def _send(request, timeout):
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                return ApiResponse(response.status, dict(response.headers), response.read())
        except urllib.error.HTTPError as exc:
            return ApiResponse(exc.code, dict(exc.headers), exc.read())

    def post_oversized(self, path, declared_length=70000):
        """POST that declares an oversized Content-Length (413 probe)."""
        return self.request_head(path, declared_length=declared_length)

    def request_head(self, path, method="POST",
                     content_type="application/adc+json", declared_length=0):
        """Send only the request head, then read the response.

        Used for rejections that happen before the server consumes the body
        (payload size, media type, method): the server then closes with the
        body still unread, and on Windows that close is a RST which destroys
        the already-sent response (WinError 10053 in urllib). Declaring the
        Content-Length a real client would send, but not transmitting the
        body, keeps the observable status/headers while leaving the receive
        buffer empty for a clean FIN.
        """
        parts = urlsplit(self.base_url)
        head = (
            "%s %s HTTP/1.1\r\n"
            "Host: %s\r\n"
            "Content-Type: %s\r\n"
            "Content-Length: %d\r\n"
            "Connection: close\r\n"
            "\r\n" % (method, path, parts.netloc, content_type, declared_length)
        )
        with socket.create_connection(
            (parts.hostname, parts.port), timeout=10
        ) as sock:
            sock.sendall(head.encode("ascii"))
            chunks = []
            while True:
                chunk = sock.recv(65536)
                if not chunk:
                    break
                chunks.append(chunk)
        raw = b"".join(chunks)
        head_bytes, _, body = raw.partition(b"\r\n\r\n")
        lines = head_bytes.decode("iso-8859-1").split("\r\n")
        status = int(lines[0].split()[1])
        headers = {}
        for line in lines[1:]:
            name, _, value = line.partition(":")
            headers[name.strip()] = value.strip()
        return ApiResponse(status, headers, body)


def query(**params):
    """Build a query string; list values become repeated parameters."""
    return "?" + urlencode(params, doseq=True)


def _start_server(port, site_dir=None, **kwargs):
    from adc.server import create_server

    server = create_server(
        str(site_dir or EXAMPLE_SITE), "127.0.0.1", port, **kwargs
    )
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, thread


def _stop_server(server, thread):
    server.shutdown()
    server.server_close()
    thread.join(timeout=5)


@pytest.fixture(scope="session")
def repo_root():
    return REPO_ROOT


@pytest.fixture(scope="session")
def example_site_dir():
    return EXAMPLE_SITE


@pytest.fixture(scope="session")
def site():
    return Site.load(EXAMPLE_SITE)


@pytest.fixture(scope="session")
def client():
    server, thread = _start_server(MAIN_PORT)
    yield Client("http://127.0.0.1:%d" % MAIN_PORT)
    _stop_server(server, thread)


@pytest.fixture(scope="session")
def stale_client():
    # SPEC 7.3.4 billing invariant is exercised against this server:
    # logistics hydration fails but staleness stays within max_staleness.
    server, thread = _start_server(
        STALE_PORT, fail_slices=["logistics"], stale_seconds=612
    )
    yield Client("http://127.0.0.1:%d" % STALE_PORT)
    _stop_server(server, thread)


@pytest.fixture(scope="session")
def overstale_client():
    # SPEC 7.3 forbids stale_fallback beyond max_staleness_seconds (900 in
    # the example passport), so this server must fail closed with 503.
    server, thread = _start_server(
        OVERSTALE_PORT, fail_slices=["logistics"], stale_seconds=1000
    )
    yield Client("http://127.0.0.1:%d" % OVERSTALE_PORT)
    _stop_server(server, thread)


@pytest.fixture(scope="session")
def noai_client(tmp_path_factory):
    # AI-ENTRY.md section 7: a site without ai.json serves no /AI/ map, so
    # the probe gets Problem Details (not a tunnel map) and the agent falls
    # back to ordinary parsing.
    import shutil

    copy = tmp_path_factory.mktemp("noai") / "site"
    shutil.copytree(EXAMPLE_SITE, copy)
    (copy / "ai.json").unlink()
    server, thread = _start_server(NOAI_PORT, site_dir=copy)
    yield Client("http://127.0.0.1:%d" % NOAI_PORT)
    _stop_server(server, thread)
