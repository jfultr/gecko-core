---
name: gecko-article-public
description: Convert a private Gecko learning article from .articles/ into a public article in articles/, with strict format, size, hidden-context, and navigation checks. Use when the user asks to publish or release an article.
---

# Public learning article

A public article is derived from a private one ([gecko-article-private](../gecko-article-private/SKILL.md)) and is committed to `articles/`. It must make sense to a stranger who has never seen the repository's chats, tickets, or private notes. Never edit a public article as a new original: change the private source, then reconvert; keep both in sync.

Publishing is outward-facing: before committing or pushing, show the user the result and get approval.

## Workflow

1. Pick the private source in `.articles/`. It must pass `python3 tools/articles/articles.py check --private`. Search `python3 tools/articles/articles.py find <terms>` for overlap with existing public articles; link to them instead of repeating.
2. Copy to `articles/<same-name>.html`. Set `gecko:visibility` to `public` and add `<meta name="gecko:source" content="<private-file-name>.html">`.
3. **Reveal hidden context** (below), then trim to the size limits.
4. Add the navigation link back to `index.html` (below).
5. Regenerate the index: `python3 tools/articles/articles.py index --write`.
6. Run `python3 tools/articles/articles.py check` until clean. Then read the article once as a stranger would.
7. Stage `articles/` only. The pre-commit hook re-runs the check.

## Everything the private rules require, stricter

Same file naming, metadata, section order, SVG accessibility, and stack as the private skill, plus:

- `gecko:visibility` = `public`; `gecko:source` present.
- Required blocks: `span.badge` reading time, `dl.gloss` glossary, `p.final` sources, a `<a href="index.html">` back link in the header.
- Title is a stable name (2-4 words); `description` is meaningful on its own in search results.
- External resources: only the two font hosts.

## Size limits (enforced by `articles.py check` and the pre-commit hook)

| Limit | Value |
|---|---|
| File size | 100,000 bytes |
| Words of prose | 3,500 |
| Reading time | 15 min at 200 words per minute |

If an article is over, split it into a series of articles with `-2` suffixes (each self-contained, linking forward and back) instead of raising limits.

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
