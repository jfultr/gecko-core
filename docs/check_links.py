"""Check local Markdown file links in the documentation tree."""

from pathlib import Path
import re
import sys
from urllib.parse import unquote


root = Path(__file__).resolve().parent
links = re.compile(r"\]\(([^)]+)\)")
errors = []
anchors = {}


def headings(document):
    if document not in anchors:
        found = set()
        counts = {}
        for line in document.read_text(encoding="utf-8").splitlines():
            match = re.match(r"^#{1,6} (.+?)\s*#*\s*$", line)
            if not match:
                continue
            title = re.sub(r"<[^>]+>|[`*_]", "", match.group(1)).lower()
            slug = re.sub(r"[^\w\- ]", "", title).replace(" ", "-")
            number = counts.get(slug, 0)
            counts[slug] = number + 1
            found.add(slug if number == 0 else f"{slug}-{number}")
        anchors[document] = found
    return anchors[document]

for document in root.rglob("*.md"):
    for line_number, line in enumerate(document.read_text(encoding="utf-8").splitlines(), 1):
        for match in links.finditer(line):
            target, separator, fragment = match.group(1).partition("#")
            if "://" in target or target.startswith("mailto:"):
                continue
            resolved = (document.parent / unquote(target)).resolve() if target else document
            if not resolved.is_file():
                errors.append(f"{document.relative_to(root)}:{line_number}: {target}")
            elif separator and unquote(fragment) not in headings(resolved):
                errors.append(f"{document.relative_to(root)}:{line_number}: #{fragment} in {target or document.name}")

if errors:
    print("Broken local links:\n" + "\n".join(errors))
    sys.exit(1)

print("All local Markdown file links and section anchors resolve.")
