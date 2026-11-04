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
3. **Copy [template.html](template.html)** to `.articles/<name>.html` and fill it. Keep the template's CSS unchanged so articles look alike.
4. **Run `python3 tools/articles/articles.py check --private`** and fix every problem.
5. Report the file path, the main takeaway, and the sources used. Do not commit.

## Naming and structure

- File: `.articles/<subject>[-<slug>][-<n>].html`, lower-case kebab-case, ASCII. The first segment is the subject (module or concept, e.g. `zenoh`, `operation`), `-2`, `-3` mark sequels. Example: `zenoh-spike.html`, `zenoh-spike-2.html`.
- One article = one file = one question. Self-contained: inline CSS and inline SVG, no scripts, no local assets.
- Required `<head>` metadata (checked): `lang`, charset, viewport, `<title>` (a 2-4 word name, not "Name: subtitle"), `description` (one sentence), and
  `gecko:visibility` (`private`), `gecko:topic` (`area/subject`, lower-case, e.g. `operation/zenoh`), `gecko:keywords` (comma list, used by `find`), `gecko:date` (`YYYY-MM-DD`).
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

Static HTML5, CSS in one `<style>`, inline SVG. Fonts only from Google Fonts with system fallbacks (Inter, JetBrains Mono). No JavaScript, no external images, no CDN libraries, no build step. Private articles may mention tickets, commits, and local paths, but keep them in kicker or `Sources` so the public conversion can drop them.

## Tools

- `python3 tools/articles/articles.py check --private`: format check.
- `python3 tools/articles/articles.py find <terms> [--private]`: ranked search by title, topic, keywords, description, headings.
