# GitHub Pages search metadata

The public project URL is `https://ne0ekspert.github.io/libxgwx/`. Pages serves
this repository under `/libxgwx/`; keep all local assets and editor links
relative so they work under the project path. If the public domain changes,
update the absolute URLs in `web/index.html`, `web/editor.html` and
`web/sitemap.xml`, including Open Graph and JSON-LD URLs.

## Indexing policy

The static landing page is the search entry point. Its title, description,
headings, capability text, API/documentation links and Rust quick start are
available without JavaScript. Its Tools section links the companion MCP server
and VS Code custom editor, with setup guides, guarded editing and native acceptance
boundaries. Related repository links are included in WebPage structured data.
It describes partial ladder decoding and guarded
optional library writes; it does not advertise a complete browser editor.

The file-dependent browser explorer is `noindex, follow` because its initial
screen has no project content until the user selects a local file. Its canonical
points to itself rather than misrepresenting it as a duplicate of the landing.
It remains crawlable so supported crawlers can read the noindex directive.
Do not include it in the sitemap.

The sitemap includes only the canonical landing URL. No estimated `lastmod`,
priority or change frequency is included. Submit
`https://ne0ekspert.github.io/libxgwx/sitemap.xml` in Search Console after it is
publicly deployed and the URL-prefix property is verified.

## Project-site robots constraint

A crawler looks for `https://ne0ekspert.github.io/robots.txt`, not
`https://ne0ekspert.github.io/libxgwx/robots.txt`. This project cannot publish a
host-root robots file through its own Pages artifact. No ineffective nested
robots file is shipped, and existing host-wide policy is not changed. If the
owner later controls a host-root site, a sitemap declaration can be added there:

```text
Sitemap: https://ne0ekspert.github.io/libxgwx/sitemap.xml
```

Do not disallow the explorer through robots rules if Google is expected to read
its noindex directive. Missing root robots.txt does not prevent crawling.

## Sharing and structured data

Open Graph and large-image social metadata use the canonical URL and the
1200×630 `social-card.png`. The image is a native rendered graphic of this
project's own branding and text; it contains no project fixture or third-party
file content. It is not loaded in the landing's critical rendering path.

JSON-LD describes `WebPage` and `SoftwareSourceCode`, with the actual Rust
language, repository and Apache-2.0 license. It includes no invented reviews,
ratings, organization, pricing or version history. These types do not guarantee
Google rich results or search rankings.

The approved `google-site-verification` meta value is a public ownership proof,
not an application credential. Deploying it makes it available to Google's
verification fetch; successful account verification is a separate step.

## Deployment verification

Keep the current Cargo test gate and the Pages `.xgwx` exclusion. Missing `write`
feature guards must be fixed rather than bypassing tests. After a successful
Pages deployment, check HTTP 200 responses, the actual landing's canonical and
verification meta tags, sitemap contents, sharing image, and the editor's
noindex policy and local-file workflow. Never publish development fixtures.

The landing uses static HTML, local CSS, a small deferred copy helper and system
fonts. WASM is loaded only when the explorer is opened. It supports a skip link,
visible keyboard focus, readable responsive content and reduced motion.

References:

- [GitHub Pages project URLs](https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages)
- [Google robots.txt location](https://developers.google.com/crawling/docs/robots-txt/create-robots-txt)
- [Google noindex](https://developers.google.com/search/docs/crawling-indexing/block-indexing)
- [Google sitemap guidance](https://developers.google.com/search/docs/crawling-indexing/sitemaps/build-sitemap)
- [Schema.org SoftwareSourceCode](https://schema.org/SoftwareSourceCode)
- [Open Graph protocol](https://ogp.me/)
