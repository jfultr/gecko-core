# Public articles

Learning articles about the ideas behind Gecko, written to be understood without access to the project's private notes. They are derived from private drafts and checked on every commit.

- Rules and workflow: [gecko-article-public](../.agents/skills/gecko-article-public/SKILL.md) and [gecko-article-private](../.agents/skills/gecko-article-private/SKILL.md).
- Tool: `python3 tools/articles/articles.py check | index --write | find TERM`.
- Enable the pre-commit check: `make hooks`.
- `index.html` is generated; do not edit it by hand.
