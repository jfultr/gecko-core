---
name: gecko-article-public
description: Convert a private Gecko learning article from .articles/ into a public article in articles/, with strict format, size, hidden-context, and navigation checks. Use when the user asks to publish or release an article.
---

# Public learning article

A public article is derived from a private one ([gecko-article-private](../gecko-article-private/SKILL.md)) and is committed to `articles/`. It must make sense to a stranger who has never seen the repository's chats, tickets, or private notes. Never edit a public article as a new original: change the private source, then reconvert; keep both in sync.

Publishing is outward-facing: before committing or pushing, show the user the result and get approval.

## Workflow

1. Pick the private source in `.articles/`. It must pass `python3 tools/articles/articles.py check --private`. Search `python3 tools/articles/articles.py find <terms>` for overlap with existing public articles; link to them instead of repeating.
2. Copy to `articles/<same-name>.html`. For an article with libraries, copy the private source `.articlesdev/<name>.html` to `articles/src/<name>.html` instead and build the final file with `bundle --public` (see Script security review). Set `gecko:visibility` to `public` and add `<meta name="gecko:source" content="<private-file-name>.html">`.
3. **Reveal hidden context** (below), then trim to the size limits.
4. Add the navigation link back to `index.html` (below).
5. Regenerate the index: `python3 tools/articles/articles.py index --write`.
6. Run `python3 tools/articles/articles.py check` until clean. Then read the article once as a stranger would.
7. Stage `articles/` only. The pre-commit hook re-runs the check.

## Everything the private rules require, stricter

Same file naming, metadata, section order, SVG accessibility, and stack as the private skill, plus:

- `gecko:visibility` = `public`; `gecko:source` present. `gecko:kind` is kept from the private article (`interactive` needs the script review below).
- Required blocks: `span.badge` reading time, `dl.gloss` glossary, `p.final` sources, a `<a href="index.html">` back link in the header.
- Title is a stable name (2-4 words); `description` is meaningful on its own in search results.
- External resources: only the two font hosts. JavaScript only as described in the script security review below; the fixed offline policy [tools/articles/csp.txt](../../../tools/articles/csp.txt) is required in every public article.

## Script security review

A public article is code that strangers run in their browser, so scripts are reviewed, not just checked. The public rules are stricter than the private ones:

- **Only reviewed code runs.** Allowed scripts are (a) library blocks inlined by `bundle` from the registry [tools/articles/libs](../../../tools/articles/libs/README.md), whose SHA-256 must equal the value recorded in `registry.json`; (b) the unmodified code viewer `tools/articles/code-viewer.js`; (c) a short author script, rejected if it uses `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon`, `eval`, `new Function`, `import()`, `importScripts`, `postMessage`, `document.write`, cookies, storage, `window.open`, or navigation. Event-handler attributes and `javascript:` URLs are rejected, as is any `<script src>`.
- **Minimal variants only.** Public bundles use the smallest recorded variant of each library (`min` for Monaco): `python3 tools/articles/articles.py bundle --public <source>`. Convert the private source to `articles/src/<name>.html` (the usual hidden-context rewrite), then bundle into `articles/<name>.html`. A variant that is not recorded in the registry cannot be bundled.
- **Fixed offline policy.** The `Content-Security-Policy` meta equals `tools/articles/csp.txt`: no network from scripts, no frames, no forms.
- **Readable without JavaScript.** Code is `<pre><code data-lang>`; a figure needs an SVG, table, or text fallback. Check by opening the file with JavaScript disabled.

Do all of this before showing the article for approval:

1. Run `python3 tools/articles/articles.py check` (it enforces the list above).
2. Read the article source yourself and confirm there is no `<iframe>`, `<object>`, `<embed>`, or `<form>`, no remote URL except the two font hosts, and that every author script does what its comment says.
3. **Recording or updating a library** (`python3 tools/articles/articles.py libs record <lib> <variant>`; the library and its `public_variant` come from its `lib.json` in the [catalog](../../../tools/articles/libs/README.md)) is a dependency review, in its own commit: check the pinned version and lockfile integrity in `tools/articles/libs`, the upstream license, and the resulting size; show the user the diff of `registry.json` and the sizes before committing. Never record a variant just to make a check pass.
4. Changing `tools/articles/code-viewer.js` or `tools/articles/csp.txt` is a security change: no network APIs, no `innerHTML` with article-derived strings, no cookies or storage. Ask the user to review the diff.
5. Run `python3 tools/articles/articles.py verify articles/<name>.html`: it opens the built file from `file://` in Chromium and Firefox and checks the console, that the only network requests are the two font hosts, and that the page works with JavaScript disabled. It must print `VERIFY OK`.

The checker is mechanical and cannot judge what a library does; the review in steps 3 and 5 is what makes a library acceptable.

## Size limits (enforced by `articles.py check` and the pre-commit hook)

| Limit | Value |
|---|---|
| Article size without registered library blocks | 100,000 bytes |
| File size with library blocks | 5,000,000 bytes |
| Words of prose | 3,500 |
| Reading time | 15 min at 200 words per minute |

A public article with Monaco is about 3.4 MB and is committed as a built file, so add libraries only where they earn it. If an article is over, split it into a series of articles with `-2` suffixes (each self-contained, linking forward and back) instead of raising limits.

## Reveal hidden context

Hidden context is anything clear only to the author or to someone who saw the private work. The checker catches the mechanical cases; you must catch the rest.

Mechanically rejected: issue IDs (`ABC-123`), commit hashes, absolute local paths, references to a chat ("as discussed"), relative dates ("yesterday"), and acronyms never defined by `<abbr title>` or a glossary entry. Fix each one by replacing it with its meaning, for example "the review of the first spike" instead of a ticket ID, or an absolute date instead of "today".

By reading, also remove or explain:

- internal project names, module names, and decision labels (name the idea, then optionally the label);
- "we", "the team", "the reviewer", "the previous article" without saying who or what, unless the earlier article is linked;
- claims such as "as the model requires" without stating the requirement;
- repository paths without a link or one line on what the file is;
- results without the setup needed to read them (what was measured, on what, how many runs);
- jargon from the project's vocabulary used before it is defined.

A good test: could a developer who has never opened the repository follow each paragraph without asking a question? If not, add the missing sentence.

## Navigation and registration

- `articles/index.html` is generated, lists every public article (title, topic, date, reading time, description), and must be up to date: the check fails on a stale or missing index.
- Each article links back to `index.html` in its header. Sequels link to their predecessor and successor by relative path (`<a href="zenoh-spike.html">`).
- Related articles are linked in the text where they are relevant, not only at the end.
- Registration is automatic: an article is "registered" when it exists in `articles/` and the index regenerated. No other list exists.

## Pre-commit

`.githooks/pre-commit` runs `python3 tools/articles/articles.py check` whenever staged files touch `articles/`. Enable once per clone with `make hooks`. `make check` also runs it (`make articles-check`). Never bypass the hook with `--no-verify`; fix the article.
