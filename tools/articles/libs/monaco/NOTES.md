# monaco: notes

Read this before using Monaco in an article, and append what you learn.

## Use it when
- The article has code blocks and the reader gains from an editor look (line numbers, real highlighting).
- Not when a short snippet in plain `<pre><code>` is enough: Monaco costs about 3.4-4.7 MB per article.

## How to use
1. In the source (`.articlesdev/<name>.html`) write `<pre><code data-lang="rust">…</code></pre>` and put `<!-- gecko:lib monaco -->` before `</body>`.
2. `python3 tools/articles/articles.py bundle .articlesdev/<name>.html`. The bundle inlines the library, the code viewer (`tools/articles/code-viewer.js`) and the `.code-host` CSS comes from the template.
3. Supported `data-lang` values: see `variants` in `lib.json`. Only Rust highlighting has been checked in a browser; check another language before relying on it.

## How it works (so you can debug it)
- The official AMD build needs a folder of files and does not work reliably from `file://` network paths. So `build.mjs` bundles the ESM sources with esbuild into one IIFE and one CSS file; the codicon font is a data URL.
- Language definitions use dynamic `import()`; esbuild inlines them. Language services (css/html/json/ts) are dropped: a read-only viewer does not need them.
- The editor worker is bundled separately and started from a `blob:` URL, so it works from `file://`. The CSP therefore needs `worker-src blob:` (it has it).
- `</script` and `</style` inside the bundle are escaped by the runner; without it the HTML parser would end the block early.
- The viewer replaces each marked block with a read-only editor, sets the height from `editor.getContentHeight()`, and follows `prefers-color-scheme`. `colorizeElement` only colors tokens and does not give the editor look; do not use it.

## Verified (Chromium 1148 and Firefox 132, Linux, headless, `file://`)
- Six Rust/shell blocks render, console is clean, no network except the font hosts, light and dark themes, blocks readable without JavaScript.
- Highlight appears about 1.7-2.0 s after load for the `min` and `full` variants.

## Pitfalls seen
- The folder-based setup (before the single-file bundle) did not highlight when opened through `\\wsl$` from a Windows browser; copying the folder to a Windows disk worked. The single-file bundle has no such dependency, but it was not tested on Windows browsers.
- A test that inserts `<script>` by replacing the first `</body>` hits a `</body>` inside the bundle. Use the last occurrence.
- Build output is deterministic on one machine (checked by building twice); not yet checked across machines.
