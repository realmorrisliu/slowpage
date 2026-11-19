#!/usr/bin/env python3
"""The opened chapter of the admitted 朝花夕拾 file, not the whole book.

This follows the same skip the reader uses: a spine document whose first
body text says Project Gutenberg is front matter, and a document under
1500 characters is not a chapter. It does not rebuild the EPUB.
"""

import hashlib
import json
import re
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
RECORD = json.loads((HERE / "chinese-admission.json").read_text())
BOOK = Path(RECORD["fixture"])
WANT = RECORD["sha256"]
CHAPTER = RECORD["chapter"]
HREF = RECORD["chapter_href"]
ANCHOR = RECORD["chapter_anchor"]

SKIP = (
    "imprint",
    "colophon",
    "titlepage",
    "cover",
    "toc",
    "nav",
    "license",
    "uncopyright",
    "dedication",
)


def plain_text(xhtml):
    out = []
    in_tag = False
    for char in xhtml:
        if char == "<":
            in_tag = True
        elif char == ">":
            in_tag = False
        elif not in_tag:
            out.append(char)
    return " ".join("".join(out).split())


def body_inner(xhtml):
    lower = xhtml.lower()
    start = lower.find("<body")
    if start < 0:
        return xhtml
    gt = xhtml.find(">", start)
    if gt < 0:
        return xhtml
    begin = gt + 1
    end = lower.find("</body>", begin)
    if end < 0:
        end = len(xhtml)
    return xhtml[begin:end]


def front_matter(href, xhtml):
    lowered = href.lower()
    if any(part in lowered for part in SKIP):
        return True
    head = plain_text(body_inner(xhtml))[:240]
    return "Project Gutenberg" in head


def fail(message):
    print(message, file=sys.stderr)
    sys.exit(1)


def main():
    if not BOOK.is_file():
        fail(f"missing {BOOK}")
    raw = BOOK.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != WANT or len(raw) != RECORD["bytes"]:
        fail(f"bytes {len(raw)} sha {digest}; admitted {RECORD['bytes']} {WANT}")
    with zipfile.ZipFile(BOOK) as book:
        if book.namelist()[0] != "mimetype" or book.read("mimetype") != b"application/epub+zip":
            fail("not an epub zip")
        opf = book.read("OEBPS/content.opf").decode("utf-8", "replace")
        manifest = {}
        for item in re.finditer(r"<item\b([^>]*)/?>", opf):
            attrs = item.group(1)
            ident = re.search(r'\bid="([^"]+)"', attrs)
            href = re.search(r'\bhref="([^"]+)"', attrs)
            if ident and href:
                manifest[ident.group(1)] = href.group(1)
        opened = None
        first_chunk = None
        for idref in re.findall(r'<itemref[^>]*idref="([^"]+)"', opf):
            href = manifest[idref]
            path = href if href.startswith("OEBPS/") else f"OEBPS/{href}"
            xhtml = book.read(path).decode("utf-8", "replace")
            text = plain_text(xhtml)
            if path.endswith("25271-0-0.txt.html"):
                first_chunk = text
            if front_matter(path, xhtml) or len(text) < 1500:
                continue
            opened = (path, xhtml, text)
            break
    if opened is None:
        fail("no chapter selected")
    path, xhtml, _text = opened
    if path != HREF:
        fail(f"opened {path}; admitted chapter is {HREF}")
    if first_chunk is None or CHAPTER in first_chunk:
        fail("the first text chunk was missing, or it contains the essay title")
    # <title> is the license heading and is not the page. The body is.
    body = plain_text(body_inner(xhtml))
    head = body[:40]
    if CHAPTER not in head:
        fail(f"opened body does not start at {CHAPTER}: {head}")
    anchor = f'<p id="{ANCHOR}">{CHAPTER}</p>'
    if xhtml.count(anchor) != 1:
        fail(f"{anchor} count is {xhtml.count(anchor)}, not 1")
    if xhtml.count("範愛農") != 6:
        fail(f"the edition's 範愛農 count is {xhtml.count('範愛農')}, not 6")
    if body.count("客店") != 2:
        fail(f"opened chapter 客店 count is {body.count('客店')}, not the essay's 2")
    print(f"opened {path}")
    print(f"sha256 {digest} bytes {len(raw)}")
    print(f"body head {head}")
    print("title paragraph once; first chunk does not contain it")


if __name__ == "__main__":
    main()
