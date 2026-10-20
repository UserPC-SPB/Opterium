# Agent Data Circuit (ADC) Security

## 1. Threat Model

The ADC protocol operates in an adversarial environment where data publishers and AI agent clients may not trust each other. The primary assets to protect are the integrity of the data stream, the confidentiality of user credentials, and the availability of the publishing infrastructure.

The following adversaries are assumed in this threat model:

*   **Malicious Publisher:** A publisher may embed prompt-injection payloads within data structures, manipulate aggregate statistics to leak private information, or craft error responses to exploit runtime parsers.
*   **Malicious Agent:** An agent may attempt to exhaust publisher resources through high-frequency requests, complex filter queries, or large payload submissions (Denial of Service). It may also attempt to replay billed responses to avoid costs or tamper with pagination cursors to access unauthorized data segments.
*   **Network Intermediary:** A man-in-the-middle or compromised CDN may attempt to modify response bodies, inject headers, or serve stale cached content that violates billing or freshness guarantees.
*   **Cross-Origin Attacker:** An attacker may attempt to poison the agent's anchor cache with malicious state from an untrusted origin or exploit SSRF vulnerabilities via outward anchors.

The protocol defines structural constraints to limit the attack surface, but it does not provide a complete defense against all application-layer attacks. Implementers are responsible for enforcing the operational limits defined in this document and for handling edge cases where the protocol specification is silent.

## 2. Prompt Injection Defense

The core security principle of ADC is **A1: Passive Inert Data**. All ADC payloads—including navigation trees, node passports, data blocks, aggregates, and RFC 9457 error objects—are strictly passive data. They are never executable code.

*   **Runtime Framing:** AI agent runtimes must treat all ADC content as untrusted data. Runtimes must frame this content as context for the model, never as instructions to be executed.
*   **Signed Content is Not Trustworthy Instructions:** Even if a payload is cryptographically signed (see Section 7), the signature only guarantees integrity and authenticity of the data, not safety. A signed payload may still contain prompt-injection text. The signature does not grant execution privileges.
*   **Parser Hardening:** Runtimes must sanitize or strictly type-check incoming data structures. Unexpected fields, malformed JSON, or non-standard MIME types must be rejected or ignored. Runtimes must not attempt to "helpfully" interpret embedded instructions within data fields.

This protocol does not define specific sanitization libraries. The responsibility for implementing robust parsing and isolation of data from control flow lies entirely with the agent runtime implementer.

## 3. Denial of Service Prevention

Publishers must declare operational limits in their manifest to prevent resource exhaustion. Agents must respect these limits. The following parameters are recommended defaults, but publishers may set stricter values:

*   `max_filter_depth`: 4 (maximum nesting depth of filter conditions)
*   `max_filter_conditions`: 32 (maximum number of logical conditions in a single filter)
*   `max_post_bytes`: 65536 (maximum size of a POST request body)
*   `max_aggregate_groups`: 1000 (maximum number of groups in an aggregate response)

**Rate Limiting:** Publishers should implement rate limiting with per-method weights. For example, `GET` requests for static blocks may have a higher allowance than `POST` requests for complex aggregates.

**Enforcement:** If an agent exceeds a declared limit, the publisher must return an RFC 9457 error with a specific error code (e.g., `413 Payload Too Large` or `429 Too Many Requests`). The protocol does not define automatic backoff mechanisms for agents; it is the agent's responsibility to handle rate-limit errors gracefully and avoid hammering endpoints.

## 4. Privacy for Aggregates

Server-side aggregates are subject to privacy risks, particularly differencing attacks. To mitigate these, ADC enforces the following rules:

*   **k-Anonymity:** Publishers must enforce a minimum group size (`min_group_size`). Any group with a count below this threshold must be suppressed (omitted from the response).
*   **Extremum Masking:** If a group size is less than `2 * min_group_size`, the publisher must mask extremum values (e.g., replace max/min with the median or omit them) to prevent identification of individuals in small groups.
*   **Totals Differencing Attack:** An attacker can calculate the value of a suppressed group by subtracting the sum of visible groups from the total. To mitigate this:
    *   Publishers should set `totals_visible_only=true` to ensure the total only reflects visible groups.
    *   Alternatively, publishers may omit the total field entirely.
    *   Formula for attack: `Value_suppressed = Totals - sum(Visible_groups)`.
*   **Publisher Responsibility:** `min_group_size` is a floor enforced by the publisher. The protocol does not provide cryptographic proofs of suppression. Agents must trust that publishers adhere to their declared privacy policies.

## 5. Anchor Security

Anchors are stateless pointers to data nodes. Security requirements for anchors include:

*   **Statelessness:** Anchors must not contain session state or secrets. They should be deterministic and derivable from the node's identity.
*   **Tree Generation Invalidation:** When a publisher updates the navigation tree, they must increment the `tree_generation` counter. Agents must discard cached anchors associated with older generations to prevent use of stale or deleted nodes.
*   **Uniform Error Responses:** To avoid existence leaks, publishers must return uniform error responses (e.g., `401 Unauthorized` or `404 Not Found`) for both non-existent nodes and nodes the agent is not authorized to access. Distinct error codes for "does not exist" vs. "forbidden" are discouraged.
*   **Namespace Isolation:** Anchors are scoped to the authority (origin) that issued them. Agents must not reuse anchors from one origin in requests to another origin. This prevents cross-origin anchor cache poisoning.
*   **No Secrets in Cursors:** Pagination cursors must not contain sensitive information such as API keys or session tokens. Cursors should be opaque, signed, or HMAC-protected if statefulness is required.

## 6. Caching and Billing Integrity

ADC defines strict rules for caching to ensure billing accuracy and data freshness:

*   **Degraded Responses:** If a response is degraded (e.g., due to server-side filtering or partial data), it must not be charged to the agent. The response must include a header or field indicating `DEGRADATION_BILLING_VIOLATION` if billing is attempted on degraded data.
*   **CDN Caching:** If a response is cached by a CDN, the cached copy must report zero cost in the billing metadata. Agents must verify that cached responses do not incur charges.
*   **POST Responses:** Responses to `POST` requests (e.g., for aggregates or filters) must be marked `Cache-Control: no-store` and must not include an `ETag` header. This prevents accidental caching of personalized or dynamic data.
*   **Freshness Generations:** Passports include a `freshness_generation` field. Agents must check this field to ensure they are not using stale data in critical transactions. If the generation is older than the agent's tolerance threshold, the agent should refetch the data.

## 7. Trust Tier Crypto

Optional JWS (JSON Web Signature) signatures provide integrity and authenticity for ADC payloads.

*   **Canonicalization:** Signatures must be computed over the canonical JSON representation of the payload using RFC 8785 (JSON Canonicalization Scheme, JCS).
*   **Algorithm Allowlist:** Only the following algorithms are permitted:
    *   `ES256` (ECDSA with P-256 and SHA-256)
    *   `ES384` (ECDSA with P-384 and SHA-384)
    *   `EdDSA` (Edwards-curve Digital Signature Algorithm)
    *   The `none` algorithm and MAC-based algorithms (e.g., `HS256`) are **rejected**.
*   **Key Resolution:** Public keys are resolved via a JWKS (JSON Web Key Set) endpoint. The `kid` (Key ID) in the JWS header must match a key in the JWKS.
*   **JWKS Retention and Revalidation:** Agents must cache JWKS but must revalidate keys periodically (e.g., every 24 hours) or upon signature verification failure. If a key is no longer present in the JWKS, the signature is considered invalid.
*   **Local Verification:** Agents should perform signature verification locally before processing the payload. Verification failure must result in rejection of the payload.

## 8. Reporting Vulnerabilities

To report a security vulnerability in the ADC protocol or its reference implementations, please open a confidential security advisory on the project's repository.

**What to include:**
*   A clear description of the vulnerability.
*   The affected version(s) of the software.
*   A proof-of-concept or detailed steps to reproduce the issue.
*   Any potential mitigations or workarounds.

Please do not disclose vulnerabilities publicly until they have been assessed and fixed. The project does not currently offer a paid bounty program.