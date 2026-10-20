# ADC Non-Goals

- **Not a general web-crawling or scraping framework.** ADC does not define how agents discover, crawl, or extract arbitrary web pages. It does not specify robots.txt behavior, rate-limit etiquette, HTML parsing, or anti-bot countermeasures. It only describes how a publisher can expose declared, structured data over HTTP+JSON.

- **Not a replacement for schema.org, sitemaps, llms.txt, or MCP.** ADC can coexist with existing metadata, site maps, agent instructions, and tool protocols. It does not require publishers to replace those systems or map them one-to-one. Its role is a bounded publisher-side data circuit, not a universal web metadata standard.

- **Not a data storage or generation specification.** ADC does not require a database, data lake, ETL pipeline, or content generation workflow. It does not dictate how publishers create, normalize, or version their source data. It only constrains the shape and semantics of the data exposed through ADC endpoints.

- **Not a semantic embedding or search engine.** The optional search/grep tier does not require ADC itself to compute embeddings, rank by semantic similarity, or maintain a global index. It points to an index or query interface that the implementer provides. ADC defines the request/response contract, not the retrieval algorithm.

- **Not a payment network or settlement system.** ADC may disclose token or byte costs per request, but it does not create wallets, invoices, escrow, billing, or settlement rails. It does not enforce payment, refund, or dispute resolution. Cost fields are informational for agent budgeting and publisher policy.

- **Not a model retraining or agent reasoning standard.** ADC does not require fine-tuning, prompt templates, chain-of-thought formats, or specific agent architectures. It does not prescribe how agents plan, verify, summarize, or act on the data. It provides structured inputs, not a cognitive protocol.

- **Not a CDN or cache implementation.** ADC does not require a particular caching layer, edge network, or storage backend. It only defines cacheable semantics, freshness generations, and invalidation invariants that implementations may honor. Publishers remain responsible for their own delivery and performance.

- **Not a central authority or trust root.** ADC does not create a registry, certification body, or global trust anchor. Optional JWS signatures allow publishers to sign responses, but ADC does not define universal key issuance or verification policy. Trust decisions remain with agents, users, and publisher-specific policy.

- **Not a backend database or language mandate.** ADC is transport- and implementation-agnostic beyond HTTP+JSON. It does not require SQL, NoSQL, GraphQL, a particular framework, or a specific programming language. Any backend that can produce the required JSON payloads is in scope.

- **Not a human-facing page layout standard.** ADC does not define HTML layout, CSS, accessibility, or visual design for humans. It may expose data that also appears on web pages, but it does not standardize how those pages are rendered. Human-facing UX remains a separate publishing concern.

- **Not an agent identity or authorization system.** ADC does not define universal agent identities, credentials, or permission models. Publishers may require API keys, tokens, or other auth mechanisms, but ADC does not standardize them. It only assumes that access control is handled outside the data circuit contract.

- **Not a global content index or discovery network.** ADC does not require a central directory of all ADC endpoints. Discovery can be local, federated, or manual, but the protocol itself does not define a global registry. It describes what a single publisher exposes, not how the internet indexes those publishers.

**Scope creep guard.** If a proposed feature requires a new global service, payment rail, agent reasoning rule, storage mandate, or universal trust decision, it is out of scope. ADC should stay a publisher-side contract for structured, budgeted, freshness-tagged, navigable data over HTTP+JSON.