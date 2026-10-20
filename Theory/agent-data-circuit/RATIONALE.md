# RATIONALE: Agent Data Circuit (ADC) Protocol Draft

This document outlines the design motivations for the Agent Data Circuit (ADC) protocol. It defines the specific structural inefficiencies in current web data access that ADC aims to resolve, compares ADC against existing data interchange formats and protocols, and addresses common objections regarding alternative solutions.

**Status Note:** ADC is an early-stage draft. There are no production implementations, no third-party audits, and no established standards body endorsement. The following claims are theoretical design goals, not proven performance metrics.

## 1. Structural Failures Addressed

Current mechanisms for providing machine-readable data to AI agents suffer from five specific structural failures. ADC is designed to mitigate these issues at the protocol layer.

### (a) Token Waste from Full-Document Transfer
Standard HTTP responses often deliver entire HTML documents or large JSON blobs. For an LLM agent, processing this data requires tokenizing the entire payload, including boilerplate, navigation, and irrelevant content. This results in significant context window consumption and increased inference costs. ADC proposes a request-response model where the agent specifies the exact entities and attributes required, allowing the server to return a minimal, structured payload containing only the requested data points.

### (b) Conflation of Official vs. Secondary Sources
In many web architectures, data about an entity (e.g., a product, a person, or a document) is scattered across multiple pages or embedded within broader content. When an agent scrapes or parses these pages, it often loses the distinction between primary/official attributes (e.g., manufacturer specifications) and secondary/derived attributes (e.g., user reviews or editorial commentary). ADC introduces a strict separation of "official" data namespaces from "derived" or "contextual" data, preserving the provenance and reliability hierarchy of each attribute.

### (c) Stale Cached Data
Web content is frequently cached at various layers (CDN, browser, client-side). For agents relying on cached snapshots, data freshness is unguaranteed. ADC defines explicit freshness metadata (e.g., `last_updated`, `ttl`) within the response envelope, allowing agents to make informed decisions about data validity without relying on opaque HTTP cache headers.

### (d) Lack of Advance Cost Signals
Agents currently have no reliable way to estimate the computational or economic cost of a data retrieval operation before executing it. They must request data to determine if it is available, expensive, or too large. ADC includes a `cost_estimate` field in the request/response lifecycle, providing a standardized signal (e.g., token count, byte size, or API credit cost) prior to data transfer.

### (e) Macro-Query Inefficiency
To obtain aggregate data (e.g., "average price of all items in category X"), agents often must download the entire collection and perform aggregation locally. This is inefficient for large datasets. ADC allows for server-side aggregation queries, where the agent requests a computed result (e.g., `aggregate: mean(price)`) rather than the raw list, reducing bandwidth and client-side processing.

## 2. Comparison with Existing Approaches

The following table contrasts ADC with common data access methods. This comparison is intended to be fair; many existing tools are superior for specific use cases.

| Feature | Raw HTML + Scraping | Schema.org JSON-LD | Sitemap.xml | llms.txt | MCP (Model Context Protocol) | REST/GraphQL APIs | AI Search Crawlers |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **What it gives an agent** | Raw document structure; requires parsing logic. | Structured metadata embedded in HTML; limited to schema.org vocabularies. | List of URLs; no content. | Plain text summary of site structure/content. | Tool definitions and execution capabilities via a server. | Structured data endpoints; requires API key/auth. | Ranked search results; snippets. |
| **Cost Signal** | None. Agent must fetch to know size. | None. | None. | None. | None (unless defined by tool). | Often in headers, but not standardized for agents. | None. |
| **Freshness** | Relies on HTTP caching; opaque. | Embedded in HTML; may be stale if page is cached. | Static list; no content freshness. | Static file; may be stale. | Depends on underlying tool implementation. | `Last-Modified` headers; API-specific. | Search index latency varies. |
| **Aggregates** | No. Agent must fetch all and compute. | No. | No. | No. | No (unless tool supports it). | Yes (GraphQL/REST aggregation). | No (snippets only). |
| **Requires Publisher Work** | Low (just serve HTML). | Medium (embed valid JSON-LD). | Low (generate XML). | Low (write text file). | High (build MCP server). | High (build/maintain API). | None (crawled). |
| **Requires Model Retraining** | Yes (parsing logic). | Partial (understanding schema.org). | No. | No. | No (tool use). | No (API interaction). | No. |

**Where Existing Tools Are Better:**
*   **REST/GraphQL APIs:** For high-volume, transactional data, dedicated APIs are superior. They offer robust authentication, rate limiting, and performance. ADC does not replace these; it aims to standardize the *data access pattern* for sites that do not have dedicated APIs.
*   **MCP:** MCP is a superior framework for *action* (executing tools). ADC focuses on *retrieval* of static/semi-static data. MCP can be used to call ADC endpoints, but ADC provides the data structure, while MCP provides the invocation mechanism.
*   **llms.txt:** For simple site navigation, `llms.txt` is low-cost and effective. It is not designed for granular data extraction or cost signaling.

## 3. "Why Not Just Use X?" FAQ

### Why not just JSON-LD?
**Concession:** JSON-LD is the current industry standard for structured data on the web. It is widely supported by search engines and is easy to embed.
**Gap:** JSON-LD is designed for search engine indexing, not agent consumption. It lacks a request/response mechanism. An agent cannot ask for "only the price and stock status" of a product; it must parse the entire embedded block. JSON-LD also lacks cost signals and explicit freshness metadata for agent decision-making. ADC complements JSON-LD by providing a query interface over structured data, rather than replacing the markup itself.

### Why not just MCP?
**Concession:** MCP is a powerful, emerging standard for connecting LLMs to tools and data sources. It solves the problem of *how* to call a function.
**Gap:** MCP does not define a standard data format for the *response*. An MCP server could return anything. ADC defines the *envelope* (cost, freshness, structure) that the data should be wrapped in. Furthermore, MCP requires a running server process for each integration, whereas ADC can be served over standard HTTP GET requests, making it easier to implement for static sites or existing CMSs without deploying new server infrastructure.

### Why not just an OpenAPI spec?
**Concession:** OpenAPI is the gold standard for documenting REST APIs. It provides precise schemas, authentication details, and interactive documentation.
**Gap:** OpenAPI is a documentation format, not a data retrieval protocol for agents. It describes how to call an API, but it does not solve the problem of token efficiency for agents that need to fetch data from sites without dedicated APIs. Additionally, OpenAPI does not include fields for `cost_estimate` or `freshness` in the response payload, which are critical for agent resource management.

### Why not just use the existing REST API?
**Concession:** If a site has a well-documented, public REST API, using it is often the best option. It is optimized for performance and reliability.
**Gap:** Most of the web does not have public REST APIs. ADC is designed for the long tail of websites that only serve HTML. It provides a standardized way to extract data from these sites without requiring the site owner to build a full API backend. It is a fallback for when no API exists.

### Doesn't llms.txt already do this?
**Concession:** `llms.txt` is a simple, low-friction file that helps agents understand a site's structure. It is easy to implement and useful for high-level navigation.
**Gap:** `llms.txt` is a static summary. It does not provide access to the actual data. It does not allow for granular queries (e.g., "get the price of item X"). It does not provide cost signals or freshness metadata. It is a map, not the territory. ADC is the mechanism to retrieve the territory.

## 4. Design Claims and Open Questions

This section honestly distinguishes between well-founded design principles and speculative claims.

**Well-Founded:**
*   **Token Efficiency:** Reducing the volume of data transferred to the LLM context window will reduce inference costs. This is a direct consequence of smaller payloads.
*   **Provenance Separation:** Separating official data from derived data is a well-understood information architecture principle. ADC formalizes this in the response structure.
*   **Cost Signaling:** Providing a `cost_estimate` before data transfer allows agents to make resource-aware decisions. This is a standard practice in API design (e.g., `X-RateLimit-Remaining`).

**Unproven/Speculative:**
*   **Server-Side Aggregation:** It is assumed that servers can efficiently handle aggregate queries (e.g., `mean(price)`) for arbitrary data sets. This may require significant backend indexing or database support that many static sites do not have.
*   **Universal Adoption:** It is speculative that publishers will adopt ADC. The burden of implementation is on the publisher, and there is currently no economic incentive for them to do so unless they want to attract AI agent traffic.
*   **Freshness Accuracy:** ADC relies on publishers to accurately report `last_updated` and `ttl`. There is no mechanism to verify this. If a publisher lies about freshness, the agent will make incorrect decisions.

## 5. How This Could Be Adopted

Standards are adopted through working code and unmet need, not by publication alone. A realistic adoption path for ADC would be:

1.  **Reference Server:** A minimal, open-source implementation of an ADC-compliant endpoint. This would serve as the "hello world" for developers. It would demonstrate how to wrap existing data (e.g., a JSON file or a database query) in the ADC envelope.
2.  **Validator:** A tool that checks if a given URL serves a valid ADC response. This would help publishers ensure their implementation is correct.
3.  **Two Independent Implementations:** To avoid vendor lock-in, at least two independent client libraries (e.g., one in Python, one in JavaScript) should be developed. These would allow agents to interact with ADC endpoints without relying on a single SDK.
4.  **Pilot Publisher:** One real-world website (e.g., a small e-commerce site or a documentation portal) should implement ADC and publish a case study. This would demonstrate the practical benefits (e.g., reduced token usage, improved data accuracy) to the community.

**Critical Success Factor:** ADC will only be adopted if it solves a problem that existing tools do not. If agents can efficiently use `llms.txt` + scraping, or if MCP becomes ubiquitous, ADC may become redundant. The value of ADC lies in the specific niche of *granular, cost-aware, fresh data retrieval from sites without dedicated APIs*.