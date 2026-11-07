#!/usr/bin/env python3
"""Check, index, and search Gecko learning articles.

Usage (from the repository root):
  python3 tools/articles/articles.py check [--private] [files...]
  python3 tools/articles/articles.py index [--write]
  python3 tools/articles/articles.py find TERM [TERM...] [--private]
  python3 tools/articles/articles.py bundle SOURCE [--public]
  python3 tools/articles/articles.py verify ARTICLE [--screenshot DIR]
  python3 tools/articles/articles.py libs list | build LIB VARIANT | record LIB VARIANT

Public articles live in articles/ (committed, strict rules and size limits).
Private drafts live in .articles/ (git-ignored, base format rules only).
Interactive articles are single files: `bundle` inlines libraries from tools/articles/libs (see the skills).
Rules are documented in .agents/skills/gecko-article-private and -public.
"""

from html import escape
from html.parser import HTMLParser
from pathlib import Path
import argparse
import hashlib
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent.parent
PUBLIC = ROOT / "articles"
PRIVATE = ROOT / ".articles"
INDEX = "index.html"

# Public size limits (see gecko-article-public skill).
MAX_BYTES = 100_000
MAX_WORDS = 3_500
MAX_MINUTES = 15
WORDS_PER_MINUTE = 200

NAME = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*\.html$")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
ALLOWED_HOSTS = ("fonts.googleapis.com", "fonts.gstatic.com")
# JavaScript policy: single-file articles only. Scripts are inline; libraries are
# inlined by `bundle` from tools/articles/libs; a fixed CSP forbids network access.
TOOLS = ROOT / "tools" / "articles"
VIEWER = TOOLS / "code-viewer.js"
CSP_FILE = TOOLS / "csp.txt"
LIBS = TOOLS / "libs"
REGISTRY = LIBS / "registry.json"
MAX_TOTAL_BYTES = 5_000_000   # public: whole file including registered library blocks
LIB_MARKER = re.compile(r"<!--\s*gecko:lib\s+([a-z0-9-]+)(?:\s+variant=(\w+))?\s*-->")
# APIs a public author script must not use (review required; see gecko-article-public).
BANNED_API = re.compile(
    r"\b(fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon|importScripts|eval|localStorage|"
    r"sessionStorage|indexedDB|postMessage)\b|new\s+Function|document\.(cookie|write)|"
    r"window\.open|\bimport\s*\(|location\s*(=|\.(href|assign|replace))")
# Acronyms every reader of the target audience is expected to know.
KNOWN_ACRONYMS = {
    "API", "CPU", "CSS", "HTML", "HTTP", "HTTPS", "ID", "IO", "JSON", "OS",
    "RAM", "SVG", "TCP", "UDP", "URL", "UTC", "UI", "WSL", "OK", "TL", "DR",
}

# Hidden-context patterns for public articles: (regex, explanation).
HIDDEN = [
    (re.compile(r"\b[A-Z]{2,6}-\d+\b"), "issue/ticket ID"),
    (re.compile(r"\b(?=[0-9a-f]*\d)(?=[0-9a-f]*[a-f])[0-9a-f]{7,40}\b"), "commit hash"),
    (re.compile(r"(?:/home/|/mnt/|/Users/|[A-Za-z]:\\\\)"), "absolute local path"),
    (re.compile(r"(?i)\b(?:в чате|в переписке|как обсуждали|как договорились|"
                r"as discussed|in our chat|earlier in the chat)\b"), "reference to a private conversation"),
    (re.compile(r"(?i)\b(?:вчера|сегодня|завтра|yesterday|today|tomorrow)\b"), "relative date"),
]


class Article(HTMLParser):
    """Collects the structure the rules need."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.doctype = False
        self.html_lang = None
        self.metas = {}
        self.charset = False
        self.viewport = False
        self.title = ""
        self.h1 = 0
        self.h2 = []          # (id, text)
        self.ids = set()
        self.hrefs = []
        self.external = []    # URLs loaded as resources
        self.scripts = []     # {"src", "type", "lib", "variant", "sha", "text"} per <script>
        self.libstyles = []   # same for <style data-gecko-lib>
        self.local_links = [] # href of <link> elements
        self.csp = None
        self._rec = None
        self.unsafe = []      # event-handler attributes and javascript: URLs
        self.svgs = []        # (role, labelledby, has_title)
        self.imgs_without_alt = 0
        self.abbr_titles = set()
        self.abbr_defined = set()
        self.dt = set()
        self.classes = set()
        self.text = []        # visible prose
        self.code_text = []
        self.tags = []
        self._stack = []
        self._cur = None
        self._buf = ""

    def handle_decl(self, decl):
        if decl.lower() == "doctype html":
            self.doctype = True

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        for key, value in attrs:
            if key.lower().startswith("on"):
                self.unsafe.append(f"<{tag} {key}=...> event handler")
            if key.lower() in ("href", "src", "action") and (value or "").strip().lower().startswith(("javascript:", "data:text/html")):
                self.unsafe.append(f"<{tag} {key}=\"{value[:30]}\"> script URL")
        self._stack.append(tag)
        self.tags.append(tag)
        self.classes.update((a.get("class") or "").split())
        if "id" in a:
            self.ids.add(a["id"])
        if tag == "html":
            self.html_lang = a.get("lang")
        elif tag == "meta":
            if "charset" in a:
                self.charset = True
            if (a.get("http-equiv") or "").lower() == "content-security-policy":
                self.csp = a.get("content", "")
            name = a.get("name")
            if name == "viewport":
                self.viewport = True
            if name:
                self.metas[name] = a.get("content", "")
        elif tag == "a" and "href" in a:
            self.hrefs.append(a["href"])
        elif tag == "link" and "href" in a:
            self.external.append(a["href"])
            self.local_links.append(a["href"])
        elif tag == "script":
            self._rec = {"src": a.get("src"), "type": a.get("type"), "lib": a.get("data-gecko-lib"),
                         "variant": a.get("data-variant"), "sha": a.get("data-sha256"), "text": ""}
            self.scripts.append(self._rec)
        elif tag == "style" and a.get("data-gecko-lib"):
            self._rec = {"lib": a["data-gecko-lib"], "variant": a.get("data-variant"),
                         "sha": a.get("data-sha256"), "text": ""}
            self.libstyles.append(self._rec)
        elif tag == "img":
            if a.get("alt") is None:
                self.imgs_without_alt += 1
            if a.get("src"):
                self.external.append(a["src"])
        elif tag == "svg":
            self.svgs.append([a.get("role"), a.get("aria-labelledby"), False])
        elif tag == "title" and "svg" in self._stack and self.svgs:
            self.svgs[-1][2] = True
        elif tag == "abbr":
            self._cur = "abbr"
            self._buf = ""
            if a.get("title"):
                self.abbr_titles.add(a["title"])
        elif tag in ("h2", "dt"):
            self._cur = tag
            self._buf = ""
            if tag == "h2":
                self.h2.append([a.get("id"), ""])
        elif tag == "h1":
            self.h1 += 1

    def handle_endtag(self, tag):
        if tag in ("script", "style"):
            self._rec = None
        if tag == self._cur:
            text = " ".join(self._buf.split())
            if tag == "abbr":
                self.abbr_defined.add(text)
            elif tag == "dt":
                self.dt.add(text)
            elif tag == "h2" and self.h2:
                self.h2[-1][1] = text
            self._cur = None
        while self._stack and self._stack.pop() != tag:
            pass

    def handle_data(self, data):
        if self._cur:
            self._buf += data
        top = self._stack[-1] if self._stack else ""
        if top in ("script", "style") and self._rec is not None:
            self._rec["text"] += data
        if top == "title" and "svg" not in self._stack:
            self.title += data
        if top in ("style", "script") or "style" in self._stack:
            return
        if "code" in self._stack or "pre" in self._stack:
            self.code_text.append(data)
        self.text.append(data)

    @property
    def lib_bytes(self):
        """Bytes of registered library blocks (inlined scripts and styles)."""
        blocks = [s for s in self.scripts if s["lib"]] + self.libstyles
        return sum(len(b["text"].encode("utf-8")) for b in blocks)

    @property
    def words(self):
        body = " ".join(self.text)
        return len(re.findall(r"\w+", body))


def parse(path):
    article = Article()
    article.feed(path.read_text(encoding="utf-8"))
    article.close()
    return article


def check_base(path, a, errors, public=False):
    """Format rules shared by private and public articles."""
    def err(msg):
        errors.append(f"{path}: {msg}")

    if not NAME.match(path.name):
        err("file name must be lower-case kebab-case ending in .html")
    if not a.doctype:
        err("missing <!doctype html>")
    if not a.html_lang:
        err("missing lang attribute on <html>")
    if not a.charset or not a.viewport:
        err("missing meta charset or viewport")
    if not a.title.strip():
        err("missing <title>")
    elif not 1 <= len(a.title.split()) <= 6:
        err("title should be a short name of 2-4 words (6 max)")
    if not a.metas.get("description", "").strip():
        err("missing meta description")
    if a.h1 != 1:
        err(f"expected exactly one h1, found {a.h1}")
    for key in ("gecko:visibility", "gecko:kind", "gecko:topic", "gecko:keywords", "gecko:date"):
        if not a.metas.get(key, "").strip():
            err(f"missing meta {key}")
    if a.metas.get("gecko:date") and not DATE.match(a.metas["gecko:date"]):
        err("gecko:date must be YYYY-MM-DD")
    kind = a.metas.get("gecko:kind")
    if kind and kind not in ("static", "interactive"):
        err("gecko:kind must be \"static\" or \"interactive\"")
    elif kind == "static" and (a.scripts or a.libstyles):
        err("gecko:kind is \"static\" but the article has scripts; use \"interactive\" or remove them")
    elif kind == "interactive" and not a.scripts:
        err("gecko:kind is \"interactive\" but the article has no script; use \"static\" or build the interactive element")
    if re.search(r"[A-Z]", a.metas.get("gecko:topic", "")):
        err("gecko:topic must be lower-case")
    for h_id, text in a.h2:
        if not h_id:
            err(f"h2 '{text}' has no id")
    if len(a.h2) >= 3 and "toc" not in a.classes:
        err("articles with 3+ sections need <nav class=\"toc\">")
    if "toc" in a.classes:
        for href in a.hrefs:
            if href.startswith("#") and href[1:] not in a.ids:
                err(f"anchor {href} has no target")
    if "tldr" not in a.classes:
        err("missing summary block <section class=\"tldr\">")
    check_scripts(a, err, public)
    for url in a.external:
        if url.startswith(("http://", "https://", "//")) and not any(h in url for h in ALLOWED_HOSTS):
            err(f"external resource not allowed: {url}")
    for role, labelled, has_title in a.svgs:
        if role != "img" or not labelled or not has_title:
            err("every inline <svg> needs role=\"img\", aria-labelledby and a <title>")
    if a.imgs_without_alt:
        err("<img> without alt")


def norm(text):
    return " ".join(text.split())


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def load_registry():
    return json.loads(REGISTRY.read_text(encoding="utf-8")) if REGISTRY.exists() else {}


def check_scripts(a, err, public):
    """Script rules. Private: any inline script, offline CSP. Public: only reviewed code."""
    canonical_csp = CSP_FILE.read_text(encoding="utf-8").strip() if CSP_FILE.exists() else None
    if a.scripts or a.libstyles or public:
        if a.csp is None:
            err("missing <meta http-equiv=\"Content-Security-Policy\"> (copy tools/articles/csp.txt)")
        elif norm(a.csp) != norm(canonical_csp or ""):
            err("Content-Security-Policy must equal tools/articles/csp.txt (no network access)")
    for href in a.local_links:
        if not href.startswith(("http://", "https://", "//")):
            err(f"local resource not allowed (articles are single files): {href}")
    if public:
        for what in a.unsafe:
            err(f"not allowed in public articles: {what}")
    registry = load_registry()
    viewer = norm(VIEWER.read_text(encoding="utf-8")) if VIEWER.exists() else None
    for s in a.scripts:
        if s["src"]:
            err(f"<script src> is not allowed (inline everything into one file): {s['src']}")
            continue
        if s["type"] not in (None, "", "text/javascript"):
            err(f"script type '{s['type']}' is not allowed (no modules, no importmaps)")
        if not public:
            continue
        if s["lib"]:
            check_lib_block(s, "js_sha256", registry, err)
        elif viewer is not None and norm(s["text"]) == viewer:
            continue
        else:
            hit = BANNED_API.search(s["text"])
            if hit:
                err(f"author script uses a restricted API ('{hit.group(0)}'); "
                    "use a registered library or get the script reviewed (gecko-article-public)")
    if public:
        for s in a.libstyles:
            check_lib_block(s, "css_sha256", registry, err)


def check_lib_block(block, key, registry, err):
    lib, variant = block["lib"], block["variant"]
    entry = registry.get(lib, {}).get("variants", {}).get(variant)
    if entry is None:
        err(f"library '{lib}' variant '{variant}' is not recorded in tools/articles/libs/registry.json")
    elif sha256(block["text"]) != entry[key] or block["sha"] != entry[key]:
        err(f"library block '{lib}/{variant}' does not match the registry hash")


def undefined_acronyms(a):
    defined = a.abbr_defined | a.dt
    found = set()
    for chunk in a.text:
        found.update(re.findall(r"\b[A-Z][A-Z0-9]{1,5}\b", chunk))
    code = " ".join(a.code_text)
    return sorted(
        t for t in found
        if t not in KNOWN_ACRONYMS and t not in defined
        and not re.search(rf"\b{t}\b", code) and not re.fullmatch(r"[A-Z]{2,6}-\d+", t)
    )


def check_public(path, a, errors):
    """Extra rules for articles in articles/."""
    def err(msg):
        errors.append(f"{path}: {msg}")

    if a.metas.get("gecko:visibility") != "public":
        err("gecko:visibility must be \"public\" in articles/")
    if not re.fullmatch(r"[a-z0-9-]+\.html", a.metas.get("gecko:source", "")):
        err("missing meta gecko:source (file name of the private source article)")
    size = path.stat().st_size
    own = size - a.lib_bytes
    minutes = max(1, round(a.words / WORDS_PER_MINUTE))
    if own > MAX_BYTES:
        err(f"article is {own} bytes without registered libraries, limit {MAX_BYTES}")
    if size > MAX_TOTAL_BYTES:
        err(f"file is {size} bytes with libraries, limit {MAX_TOTAL_BYTES}")
    if a.words > MAX_WORDS:
        err(f"{a.words} words, limit {MAX_WORDS}")
    if minutes > MAX_MINUTES:
        err(f"reading time {minutes} min, limit {MAX_MINUTES}")
    if "badge" not in a.classes:
        err("missing reading-time badge <span class=\"badge\">")
    if "gloss" not in a.classes:
        err("missing glossary <dl class=\"gloss\">")
    if "final" not in a.classes:
        err("missing sources line <p class=\"final\">")
    if INDEX not in a.hrefs:
        err(f"missing navigation link back to {INDEX}")
    text = " ".join(a.text)
    for pattern, what in HIDDEN:
        for m in sorted(set(pattern.findall(text)))[:5]:
            err(f"hidden context ({what}): '{m}' - explain it or remove it")
    for term in undefined_acronyms(a):
        err(f"acronym '{term}' is never defined (<abbr title> or glossary entry)")
    for chunk in a.abbr_titles:
        if len(chunk.split()) < 2:
            err(f"abbr title '{chunk}' is not a real explanation")


def meta_of(path):
    a = parse(path)
    return {
        "file": path.name,
        "title": a.title.strip(),
        "description": a.metas.get("description", ""),
        "date": a.metas.get("gecko:date", ""),
        "topic": a.metas.get("gecko:topic", ""),
        "kind": a.metas.get("gecko:kind", ""),
        "keywords": a.metas.get("gecko:keywords", ""),
        "minutes": max(1, round(a.words / WORDS_PER_MINUTE)),
        "lang": a.html_lang or "",
        "headings": [t for _, t in a.h2],
    }


def public_files():
    return sorted(p for p in PUBLIC.glob("*.html") if p.name != INDEX)


def render_index(items):
    rows = "\n".join(
        f'    <li><a href="{escape(i["file"])}">{escape(i["title"])}</a>'
        f'<span class="meta">{escape(i["topic"])} · {escape(i["kind"])} · {escape(i["date"])} · {i["minutes"]} min</span>'
        f'<p>{escape(i["description"])}</p></li>'
        for i in items
    )
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Gecko articles</title>
<meta name="description" content="Learning articles about how and why Gecko is built the way it is.">
<style>
  :root {{ --bg: #f6f5f1; --ink: #1b1e24; --muted: #5a6170; --line: #d8d5cd; --accent: #2563a6; color-scheme: light; }}
  @media (prefers-color-scheme: dark) {{
    :root {{ --bg: #111318; --ink: #e7e9ee; --muted: #9ba3b3; --line: #2b303a; --accent: #7db7ea; color-scheme: dark; }}
  }}
  body {{ margin: 0; background: var(--bg); color: var(--ink); font: 400 17px/1.65 Inter, system-ui, sans-serif; }}
  .wrap {{ max-width: 720px; margin: 0 auto; padding: 0 16px 48px; }}
  ul {{ list-style: none; padding: 0; }}
  li {{ padding: 14px 0; border-bottom: 1px solid var(--line); }}
  li p {{ margin: 0.3em 0 0; color: var(--muted); }}
  a {{ color: var(--accent); font-weight: 600; }}
  .meta {{ display: block; font-size: 0.85rem; color: var(--muted); }}
</style>
</head>
<body>
<div class="wrap">
  <h1>Gecko articles</h1>
  <p>Learning material about the ideas behind Gecko. Generated by tools/articles/articles.py; do not edit by hand.</p>
  <ul>
{rows}
  </ul>
</div>
</body>
</html>
"""


def cmd_check(args):
    errors = []
    if args.files:
        files = [Path(f).resolve() for f in args.files]
    else:
        files = sorted(PRIVATE.glob("*.html")) if args.private else public_files()
    for path in files:
        is_public = PUBLIC in path.parents
        a = parse(path)
        check_base(path.relative_to(ROOT), a, errors, is_public)
        if is_public:
            check_public(path.relative_to(ROOT), a, errors)
    if not args.private and not args.files:
        listed = PUBLIC / INDEX
        expected = render_index([meta_of(p) for p in sorted(public_files(), key=lambda p: (meta_of(p)["date"], p.name), reverse=True)])
        if public_files() and (not listed.exists() or listed.read_text(encoding="utf-8") != expected):
            errors.append("articles/index.html is missing or stale; run: python3 tools/articles/articles.py index --write")
    for e in errors:
        print(e, file=sys.stderr)
    print(f"checked {len(files)} article(s): {len(errors)} problem(s)")
    return 1 if errors else 0


def cmd_index(args):
    items = sorted((meta_of(p) for p in public_files()), key=lambda i: (i["date"], i["file"]), reverse=True)
    html = render_index(items)
    if args.write:
        (PUBLIC / INDEX).write_text(html, encoding="utf-8")
        print(f"wrote articles/{INDEX} ({len(items)} article(s))")
    else:
        print(html)
    return 0


def cmd_find(args):
    terms = [t.lower() for t in args.terms]
    folders = [(PUBLIC, "public")] + ([(PRIVATE, "private")] if args.private else [])
    hits = []
    for folder, label in folders:
        for path in sorted(folder.glob("*.html")):
            if path.name == INDEX:
                continue
            m = meta_of(path)
            weighted = (
                (m["title"] + " " + m["file"], 4),
                (m["topic"] + " " + m["keywords"], 3),
                (m["description"], 2),
                (" ".join(m["headings"]), 1),
            )
            score = sum(w for field, w in weighted for t in terms if t in field.lower())
            if score:
                hits.append((score, label, folder.name + "/" + path.name, m["title"], m["topic"] + ", " + m["kind"]))
    for score, label, rel, title, topic in sorted(hits, reverse=True):
        print(f"{score:3}  {label:7} {rel}  [{topic}]  {title}")
    if not hits:
        print("no similar articles found")
    return 0


def lib_meta(name):
    """Catalog entry of a library (tools/articles/libs/<name>/lib.json) or None."""
    path = LIBS / name / "lib.json"
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else None


def catalog():
    return [json.loads(p.read_text(encoding="utf-8"))
            for p in sorted(LIBS.glob("*/lib.json")) if not p.parent.name.startswith("_")]


def ensure_built(lib, variant, rebuild=False):
    """Returns (js, css) of a library variant, building it with Node when missing."""
    js = LIBS / "dist" / f"{lib}-{variant}.js"
    css = LIBS / "dist" / f"{lib}-{variant}.css"
    if rebuild or not (js.exists() and css.exists()):
        if not (LIBS / "node_modules").exists():
            subprocess.run(["npm", "ci", "--omit=dev", "--no-audit", "--no-fund"], cwd=LIBS, check=True)
        subprocess.run(["node", "build.mjs", lib, variant], cwd=LIBS, check=True)
    return js.read_text(encoding="utf-8"), css.read_text(encoding="utf-8")


def cmd_libs(args):
    if args.action == "list":
        pkg = json.loads((LIBS / "package.json").read_text(encoding="utf-8")).get("dependencies", {})
        registry = load_registry()
        for m in catalog():
            recorded = sorted(registry.get(m["name"], {}).get("variants", {}))
            print(f'{m["name"]} {m["version"]} ({m["license"]}) - {m["title"]}')
            print(f'  {m["purpose"]}')
            for v, text in m["variants"].items():
                tags = [t for t, on in (("private default", v == m["private_variant"]),
                                        ("public", v == m.get("public_variant"))) if on]
                print(f'  variant {v}: {text}' + (f' [{", ".join(tags)}]' if tags else ""))
            print(f'  marker: {m["marker"]}   notes: tools/articles/libs/{m["name"]}/NOTES.md')
            print(f'  public: {"recorded " + ", ".join(recorded) if recorded else "not recorded (needs review)"}')
            if pkg.get(m["npm"]) != m["version"]:
                print(f'  WARNING: package.json pins {m["npm"]} {pkg.get(m["npm"])}, lib.json says {m["version"]}')
        return 0
    meta = lib_meta(args.lib or "")
    if meta is None or not args.variant or args.variant not in meta["variants"]:
        print("usage: libs build|record <lib> <variant>; see `libs list`", file=sys.stderr)
        return 2
    js, css = ensure_built(args.lib, args.variant, rebuild=True)
    print(f"{args.lib}/{args.variant}: js {len(js.encode())} bytes sha256 {sha256(js)}")
    print(f"{args.lib}/{args.variant}: css {len(css.encode())} bytes sha256 {sha256(css)}")
    if args.action == "record":
        registry = load_registry()
        entry = registry.setdefault(args.lib, {"npm": meta["npm"], "version": meta["version"], "variants": {}})
        entry["version"] = meta["version"]
        entry.setdefault("variants", {})[args.variant] = {
            "js_sha256": sha256(js), "css_sha256": sha256(css),
            "js_bytes": len(js.encode()), "css_bytes": len(css.encode()),
        }
        REGISTRY.write_text(json.dumps(registry, indent=2) + "\n", encoding="utf-8")
        print(f"recorded in {REGISTRY.relative_to(ROOT)} (review the diff before committing)")
    return 0


def lib_block(lib, variant, meta):
    js, css = ensure_built(lib, variant)
    attrs = f'data-gecko-lib="{lib}" data-variant="{variant}"'
    parts = [f'<style {attrs} data-sha256="{sha256(css)}">{css}</style>',
             f'<script {attrs} data-sha256="{sha256(js)}">{js}</script>']
    if meta.get("viewer"):
        parts.append(f'<script>\n{VIEWER.read_text(encoding="utf-8")}</script>')
    return "\n".join(parts)


def cmd_bundle(args):
    src = Path(args.source).resolve()
    out = (PUBLIC if args.public else PRIVATE) / src.name
    if out == src:
        print("source and output are the same file; keep sources in .articlesdev/ or articles/src/", file=sys.stderr)
        return 1
    text = src.read_text(encoding="utf-8")
    if not LIB_MARKER.search(text):
        print("no <!-- gecko:lib NAME --> marker in the source (see: articles.py libs list)", file=sys.stderr)
        return 1
    registry = load_registry()

    def replace(m):
        lib = m.group(1)
        meta = lib_meta(lib)
        if meta is None:
            raise SystemExit(f"unknown library '{lib}'; see `articles.py libs list` or add it (tools/articles/libs/README.md)")
        if args.public:
            variant = meta.get("public_variant")
            if not variant:
                raise SystemExit(f"library '{lib}' is not allowed in public articles")
        else:
            variant = m.group(2) or meta["private_variant"]
        if variant not in meta["variants"]:
            raise SystemExit(f"library '{lib}' has no variant '{variant}' (available: {', '.join(meta['variants'])})")
        if args.public:
            entry = registry.get(lib, {}).get("variants", {}).get(variant)
            js, css = ensure_built(lib, variant)
            if entry is None or entry["js_sha256"] != sha256(js) or entry["css_sha256"] != sha256(css):
                raise SystemExit(f"public bundles need a recorded, reviewed library: run "
                                 f"python3 tools/articles/articles.py libs record {lib} {variant}")
        return lib_block(lib, variant, meta)

    out.write_text(LIB_MARKER.sub(replace, text), encoding="utf-8")
    print(f"wrote {out.relative_to(ROOT)} ({out.stat().st_size} bytes)")
    return 0


def cmd_verify(args):
    """Runs the browser check (tools/articles/libs/verify.mjs) on an article."""
    if not (LIBS / "node_modules" / "playwright").exists():
        subprocess.run(["npm", "ci", "--no-audit", "--no-fund"], cwd=LIBS, check=True)
    cmd = ["node", "verify.mjs", str(Path(args.article).resolve())]
    if args.screenshot:
        cmd += ["--screenshot", str(Path(args.screenshot).resolve())]
    return subprocess.run(cmd, cwd=LIBS).returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("check")
    c.add_argument("--private", action="store_true", help="check .articles/ instead of articles/")
    c.add_argument("files", nargs="*")
    c.set_defaults(func=cmd_check)
    i = sub.add_parser("index")
    i.add_argument("--write", action="store_true")
    i.set_defaults(func=cmd_index)
    f = sub.add_parser("find")
    f.add_argument("terms", nargs="+")
    f.add_argument("--private", action="store_true", help="also search .articles/")
    f.set_defaults(func=cmd_find)
    b = sub.add_parser("bundle", help="inline libraries into a single-file article")
    b.add_argument("source", help="article source with <!-- gecko:lib monaco --> markers (e.g. .articlesdev/x.html)")
    b.add_argument("--public", action="store_true", help="write to articles/ using only recorded, minimal library variants")
    b.set_defaults(func=cmd_bundle)
    lb = sub.add_parser("libs", help="list the library catalog, build or record a variant")
    lb.add_argument("action", choices=["list", "build", "record"])
    lb.add_argument("lib", nargs="?")
    lb.add_argument("variant", nargs="?")
    lb.set_defaults(func=cmd_libs)
    v = sub.add_parser("verify", help="open an article in Chromium and Firefox and check console, network, no-JS")
    v.add_argument("article")
    v.add_argument("--screenshot", help="directory for screenshots")
    v.set_defaults(func=cmd_verify)
    args = parser.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
