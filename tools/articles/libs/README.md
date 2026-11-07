# Article libraries: catalog

The catalog of JavaScript libraries that Gecko learning articles may inline, with the experience gathered using them. Articles are single files: nothing is linked at run time. Agents read this catalog when planning an interactive article ([gecko-article-private](../../../.agents/skills/gecko-article-private/SKILL.md)); the public checks in [gecko-article-public](../../../.agents/skills/gecko-article-public/SKILL.md) rely on its hashes.

## Using the catalog

```sh
python3 tools/articles/articles.py libs list      # names, purpose, variants, sizes, public status
```

Then read `tools/articles/libs/<lib>/NOTES.md` (when to use it, working markup, pitfalls), put the library's marker (for example `<!-- gecko:lib monaco -->`) into the article source, and run `articles.py bundle`. Check the result with `articles.py verify <article.html>` (Chromium and Firefox, light and dark, console, network, no-JavaScript).

| Library | Purpose | Notes |
|---|---|---|
| `monaco` | read-only code viewer | [monaco/NOTES.md](monaco/NOTES.md) |

(`libs list` is the authoritative list; extend this table when a library is added.)

## Layout

```text
tools/articles/libs/
├── package.json, package-lock.json   pinned npm versions of every library, esbuild, playwright
├── registry.json                     reviewed SHA-256 of built variants (needed for public articles)
├── build.mjs                         generic runner: node build.mjs <lib> <variant> -> dist/
├── verify.mjs                        browser check behind `articles.py verify`
├── <lib>/
│   ├── lib.json                      catalog entry: purpose, version, license, variants, marker
│   ├── NOTES.md                      experience: when to use, working markup, how it works, pitfalls
│   └── build.mjs                     recipe: exports `variants` and build(variant, ctx) -> {js, css}
└── _template/                        copy this to start a new library
```

`dist/` and `node_modules/` are git-ignored. Node 20+ and network (for `npm ci`) are needed only to build and verify.

## Adding a library (agents may do this after the user approves)

1. Check the catalog first. If a library already covers the need, use it.
2. Propose the library to the user in one message: name, pinned version, license, what the article gains, expected size per article. Wait for approval before installing anything.
3. `cp -r tools/articles/libs/_template tools/articles/libs/<lib>`, add the pinned dependency to `package.json` (exact version, no `^`), `npm install` in this folder, and fill `lib.json`, `build.mjs`, `NOTES.md`.
4. Build both variants (`articles.py libs build <lib> <variant>`) and write a small test article source; run `articles.py bundle` and `articles.py verify` on it. Fix until it is `VERIFY OK`.
5. Set `public_variant` to the smallest useful variant, or `null` if the library must stay out of public articles.
6. Add the library to the table above and show the user: version, license, sizes, verify output.
7. Recording for public use (`articles.py libs record <lib> <variant>`) is a separate reviewed commit; see the script security review in the public skill. Never record a variant just to make a check pass.

## Recording experience

Whoever finds a pitfall, a working recipe, or a verified claim while using a library appends it to that library's `NOTES.md` in the same change (what was done, in which browsers and versions, what happened). Keep entries factual: separate verified results from guesses.

## Rules

- Build output is deterministic for the pinned versions (checked by building twice on one machine). If a rebuild gives a different hash on another machine, the public check fails on purpose: investigate before recording.
- Public articles may use only a library's `public_variant` and only after it is recorded in `registry.json`.
- `lib.json` and `package.json` must agree on the version; `libs list` prints a warning when they do not.
