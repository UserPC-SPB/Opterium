# CASE-STUDY: twenty documents from a real catalog, scraped vs served

**One line:** the same twenty records - 21 HTTP requests, 4,139,369 bytes
and 1,034,850 agent tokens of raw HTML - arrive in 2 HTTP requests, 2,343
bytes and 587 tokens through the `/AI/` cold entry: **1,767x fewer bytes,
1,763x fewer agent tokens, 10.5x fewer requests**.

## Method

Site class: a public catalog of standards documents (an RFC editor's
site), chosen because its `robots.txt` explicitly allows the section we
need, the catalog is stable, and no authentication is involved. Selector:
the official index, then the first 20 issued entries' information pages;
five fields were extracted per document (title, status, authors, year,
word count).

`robots.txt` was fetched first and honored (the section is allowed); its
135 bytes are excluded from the totals below, because any compliant agent
pays that request either way. The scrape side fetched the index (1
request) and 20 pages (20 requests) with a 0.1 second politeness delay
between pages; the delay sits inside the scrape wall time. The serve side
wrote the extracted rows into a local ADC site (21 nodes), served it with
this repository's reference server on 127.0.0.1, and made two requests:
the tunnel map, then one projected query:

    GET /AI[s1]?get=text&fields=title,status,authors,year,words&limit=20

Bytes are UTF-8 response bytes on the wire; agent tokens are
`utf8_bytes_div_4` (`ceil(bytes / 4)`), raw HTML included per the
README's premise. Because `ceil(bytes / 4)` rounds up on each response,
the byte ratio (1,767x) and the token ratio (1,763x) are close but not
identical. Both sides serialize the rows identically (compact ASCII
JSON), so the ratio measures delivery cost, not encoding. One run,
2026-09-25, measured by `tools/measure_case_study.py`.

## Results

| Metric | Scrape (remote) | `/AI/` (local) | Ratio |
|---|---:|---:|---:|
| HTTP requests | 21 | 2 | 10.5x |
| Bytes transferred | 4,139,369 | 2,343 | 1,767x |
| Agent tokens | 1,034,850 | 587 | 1,763x |
| Time to answer (s) | 8.404 | 0.0396 | 212x (loopback) |

For scale: the scraper's own extraction - after downloading 4.1 MB - came
to 545 tokens of rows; the cold entry delivered the same rows, plus
provenance, in 587 tokens in a single request.

## Caveats

1. **Local simulation.** The target site does not publish `/AI/`. We
   extracted its data with the scraper and served it from our reference
   server on localhost. The `/AI/` timings are loopback and are not
   comparable to remote scraping; bytes and token ratios are the
   transferable numbers.
2. **One site, twenty documents, one run.** This is an illustration, not
   a benchmark. Timings vary between runs; bytes and tokens were
   identical across our runs.
3. **Minimal key set.** The demo key set is `(text, meta, price, stock)`;
   this study exercises `text` with five fields only.
4. **Compliance overhead excluded, politeness included.** The `robots.txt`
   fetch (135 bytes) is excluded from the scrape totals; the 0.1 second
   delay between page fetches is included in the scrape wall time.
5. **Raw HTML counted as agent context.** A scraper could parse part of
   each page outside the model and pay fewer tokens than 1,034,850 - but
   still 21 requests, 4.1 MB, and per-site parsing logic the agent has to
   carry.
6. **The ratio depends on what you extract.** This study retrieves five
   metadata fields per document and does not retrieve document bodies.
   An agent that needed the full text would pay roughly the same bytes
   through `/AI/` as through scraping - the cold entry removes
   presentation overhead, not content. The 1,767x figure measures
   presentation overhead on this site class, not universal compression.
7. **This is a documentation site, not a marketplace.** On a marketplace
   with structured catalog data the ratio would be smaller - likely two
   orders of magnitude in bytes and one to two in tokens.

## Reproduction

Site class: a public RFC catalog; selector: official index -> first 20
issued info pages, five fields per page. No raw site data is committed to
this repository; the script keeps rows, HTML, and the built site in a
temporary directory.

```bash
python tools/measure_case_study.py
```

Requires Python 3.11+ and network access to the catalog. The script
aborts with exit code 2 if `robots.txt` disallows the section, scrapes,
builds a temporary ADC site, starts the reference server on
127.0.0.1:8123, verifies the projected rows against the extraction, and
prints the metrics; the full JSON lands in the work directory it names.

## Why this matters

Scraping makes the agent pay for presentation: megabytes of markup it
discards, twenty round trips, and parsing logic maintained per site. A
publisher who declares the same rows through the cold entry serves them
at 1/1767 of the transfer and 1/1763 of the context, from one endpoint
any HTTP client can call - which is the economic argument of
[ECONOMICS.md](ECONOMICS.md) measured on a real site.
