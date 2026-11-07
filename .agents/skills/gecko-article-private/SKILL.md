---
name: gecko-article-private
description: Write a private learning article about a topic inside the Gecko repository (spike result, design idea, mechanism, review). Use when the user asks for an explanatory article, a "study" page, or a write-up for learning. Output goes to the git-ignored .articles/ folder.
---

# Private learning article

A private article teaches the author (and the agent's next session) one topic of this repository. It is a draft of record: free in scope, strict in format. It lives in `.articles/`, which is git-ignored and never published. To publish, use [gecko-article-public](../gecko-article-public/SKILL.md).

This skill is not an explicit role from [AGENTS.md](../../../AGENTS.md); it may be used alongside any role or none. Documentation rules still apply: project instructions are English, article text may use the author's language (see Style).

## Workflow

1. **Find similar articles first.** Run `python3 tools/articles/articles.py find <terms> --private` with the topic's key terms. Read hits in `articles/` (public) and `.articles/`. Extend, link to, or number a sequel (`-2`) instead of duplicating. State in the lede what the article adds over earlier ones.
2. **Ground the facts.** Read the repository sources the article describes (models, specs, reports, code, commit history). Every claim must trace to one; list them in the closing `Sources` line. Mark what is accepted, proposed, implemented, and measured, as the project docs do.
3. **Choose the type, then plan.** Decide `static` or `interactive` before outlining (see Article type below), state the choice and its reason in one line, and plan the article around it.
4. **Copy [template.html](template.html)** to `.articles/<name>.html` (static) or `.articlesdev/<name>.html` (interactive source) and fill it. Keep the template's CSS unchanged so articles look alike. Set `gecko:kind`.
5. **Interactive only:** run `python3 tools/articles/articles.py bundle .articlesdev/<name>.html` after every edit of the source, and `python3 tools/articles/articles.py verify .articles/<name>.html` before reporting (see Allowed stack).
6. **Run `python3 tools/articles/articles.py check --private`** and fix every problem.
7. Report the file path, the type chosen and why (and that it can be switched), the main takeaway, and the sources used. Do not commit.

## Article type

Every article has a type, recorded as `<meta name="gecko:kind" content="static|interactive">`. The type is chosen **before** the outline because it shapes the article; it is not a finishing step. The default is `static`.

Choose `interactive` only when at least one holds:

1. The user asks for interactivity ("interactive", a chart, 3D, "let me change the parameter").
2. The article has code blocks and the user wants the editor look (Monaco viewer).
3. An idea is explained better by the reader's action than by text (a parameter whose effect is the point, switchable data series). Then say so in one line and wait for the user's confirmation before building.

Plan an interactive article **by element, not by decoration**. For each interactive element write down: what the reader changes, what they should understand from the result, and the fallback without JavaScript (an SVG or a table). An element that cannot answer "what does the reader learn from interacting?" stays static. Interactive elements go into the section where the idea is explained, inside the usual order (answer first, then why, how, results); do not append them at the end or add JavaScript to a finished static draft. If the needed library is not in the registry (charts, 3D), stop and propose adding it as a separate reviewed change instead of improvising.

`articles.py check` enforces the type: `interactive` needs at least one script, `static` must have none.

## Naming and structure

- File: `.articles/<subject>[-<slug>][-<n>].html`, lower-case kebab-case, ASCII. The first segment is the subject (module or concept, e.g. `zenoh`, `operation`), `-2`, `-3` mark sequels. Example: `zenoh-spike.html`, `zenoh-spike-2.html`.
- One article = one file = one question. Self-contained single file: inline CSS, inline SVG, and, when the topic needs it, inline JavaScript (see Allowed stack). No local assets, no remote scripts.
- Required `<head>` metadata (checked): `lang`, charset, viewport, `<title>` (a 2-4 word name, not "Name: subtitle"), `description` (one sentence), and
  `gecko:visibility` (`private`), `gecko:kind` (`static` or `interactive`), `gecko:topic` (`area/subject`, lower-case, e.g. `operation/zenoh`), `gecko:keywords` (comma list, used by `find`), `gecko:date` (`YYYY-MM-DD`).
- Required body order: `header` (kicker, one `h1`, lede, reading-time badge) → `nav.toc` (3+ sections) → `section.tldr` → `h2` sections with `id` → glossary `dl.gloss` → `p.final` sources.
- Open with the answer (summary first), then why, how, results, decisions, open risks.

## Style

- Teach from zero: define each term on first use (`<abbr title="full sentence explanation">`) and repeat it in the glossary.
- Short paragraphs, concrete numbers, one idea per section. Prefer a figure or a table to a long list of facts.
- Figures are inline SVG using the template's classes and theme variables; each has `role="img"`, `aria-labelledby`, a `<title>` and a `<figcaption>` stating the takeaway.
- Distinguish observed facts, interpretation, and open questions; say how a number was obtained.
- Language: the article body may be Russian or English (set `lang`). Keep identifiers, code, and file paths verbatim.
- Light and dark themes both work; layout holds at phone width.

## Allowed stack

Static HTML5, CSS in one `<style>`, inline SVG, and **inline JavaScript when it helps the reader** (interactive figures, charts, 3D, a code viewer). Fonts only from Google Fonts with system fallbacks (Inter, JetBrains Mono). No external images, no remote scripts or CDN libraries, no local assets.

Private articles are the lab: any inline script is allowed, but the page must stay **offline and single-file**:

- Every article carries the fixed policy `<meta http-equiv="Content-Security-Policy" content="…">` from [tools/articles/csp.txt](../../../tools/articles/csp.txt) (the template already has it). It forbids network access from scripts (`connect-src 'none'`), frames, forms, and remote scripts; only the two font hosts are reachable for CSS and fonts. `articles.py check` requires it whenever an article has a script and rejects `<script src>` and local resources.
- Content must stay readable without JavaScript: a figure has an SVG or table fallback, a code block is `<pre><code data-lang="rust">`.
- Libraries are never linked. They live in the catalog [tools/articles/libs](../../../tools/articles/libs/README.md): `python3 tools/articles/articles.py libs list` shows what exists, and each library has `NOTES.md` with the experience gathered (when to use it, working markup, pitfalls). Read it while planning the article, not after. Libraries are pinned, built with Node and esbuild, and inlined by `bundle`. Today the catalog holds Monaco (code viewer); charts and 3D engines are added the same way.
- **Adding a library is allowed, with the user's approval first.** If nothing in the catalog fits an interactive element you planned, propose a library in one message (name, version, license, what the article gains, size) and wait. After approval follow "Adding a library" in the catalog README. Do not link a CDN or paste library code into an article.
- **Record what you learn.** A pitfall, a working recipe, or a verified claim about a library goes into its `NOTES.md` in the same change.

**Interactive articles.** Write the article source as `.articlesdev/<name>.html` and put the library marker from the catalog (for example `<!-- gecko:lib monaco -->`) just before `</body>`. Mark code blocks with `data-lang` (`rust`, `shell`, `ini`, `python`, `cpp`; `full` also has `javascript`, `typescript`, `json`, `yaml`, `markdown`). Then run `python3 tools/articles/articles.py bundle .articlesdev/<name>.html`: it builds the library on first use (needs Node and network for `npm ci`; the output is cached in git-ignored `tools/articles/libs/dist/`), inlines the `full` variant and the code viewer ([tools/articles/code-viewer.js](../../../tools/articles/code-viewer.js)), and writes the single file `.articles/<name>.html` (about 4.8 MB with Monaco; the size is not a problem for private articles). Edit the source and bundle again; never edit the generated file. The template already has the `.code-host` CSS. Articles without libraries need no `src/` and no bundling.

Private articles may mention tickets, commits, and local paths, but keep them in kicker or `Sources` so the public conversion can drop them.

## Tools

- `python3 tools/articles/articles.py check --private`: format check.
- `python3 tools/articles/articles.py find <terms> [--private]`: ranked search by title, topic, keywords, description, headings.
- `python3 tools/articles/articles.py libs list`: the library catalog (purpose, variants, sizes, public status).
- `python3 tools/articles/articles.py bundle <source>`: inline catalog libraries into a single-file article.
- `python3 tools/articles/articles.py verify <article.html> [--screenshot dir]`: open the built article in Chromium and Firefox (light and dark) and check console errors, network requests, and the no-JavaScript view. Run it for every interactive article before reporting.
