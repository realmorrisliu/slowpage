#!/usr/bin/env python3
"""The opened plain-zh chapter is the derived 范愛農 text, not a short stub.

plain-zh is an unstyled test fixture derived from the admitted original.
It is not that EPUB, and this check does not rewrite the original.
"""

import hashlib
import json
import re
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_books  # noqa: E402

RECORD = json.loads((HERE / "chinese-admission.json").read_text())
SOURCE = Path(RECORD["fixture"])
WANT_SOURCE = RECORD["sha256"]


def fail(message):
    print(message, file=sys.stderr)
    sys.exit(1)


def stripped(text):
    return re.sub(r"\s+", "", text)


def body_text(xhtml):
    match = re.search(r"(?is)<body[^>]*>(.*)</body>", xhtml)
    inner = match.group(1) if match else xhtml
    return re.sub(r"(?is)<[^>]+>", "", inner)


def main():
    try:
        build_books.essay_paragraphs("<html><body><p>范愛農</p><p>在客店。</p></body></html>")
    except ValueError:
        pass
    else:
        fail("a short extract was accepted")

    if not SOURCE.is_file():
        fail(f"missing original {SOURCE}")
    original = SOURCE.read_bytes()
    if hashlib.sha256(original).hexdigest() != WANT_SOURCE or len(original) != RECORD["bytes"]:
        fail("original 朝花夕拾 bytes do not match the admission")
    manifest = build_books.write_plain_zh()
    if SOURCE.read_bytes() != original:
        fail("write_plain_zh modified the original EPUB")
    derived_path = Path(manifest["derived_path"])
    derived = derived_path.read_bytes()
    digest = hashlib.sha256(derived).hexdigest()
    if digest != manifest["derived_sha256"]:
        fail("derived sha was not recorded")
    saved = json.loads(build_books.PLAIN_ZH_MANIFEST.read_text())
    if saved["derived_sha256"] != digest or saved["source_sha256"] != WANT_SOURCE:
        fail("manifest sha does not match the files")
    if saved["kind"] != "unstyled-derived-test-fixture":
        fail("manifest does not label the derived fixture")
    if saved["rights"] != "Public domain in the USA.":
        fail("rights were not carried from the source")
    with zipfile.ZipFile(derived_path) as book:
        if book.namelist()[0] != "mimetype" or book.read("mimetype") != b"application/epub+zip":
            fail("not an epub zip")
        opf = book.read("OEBPS/content.opf").decode("utf-8")
        chapter = book.read("OEBPS/chapter.xhtml").decode("utf-8")
        names = set(book.namelist())
    if names != {
        "mimetype",
        "META-INF/container.xml",
        "OEBPS/content.opf",
        "OEBPS/chapter.xhtml",
    }:
        fail(f"unexpected package entries: {sorted(names)}")
    lowered = chapter.lower()
    for token in ("<style", "<link", "<script", "<iframe", ".css"):
        if token in lowered:
            fail(f"derived chapter contains {token}")
    if "朝花夕拾" in opf or "朝花夕拾" in chapter:
        fail("the derived fixture presents itself as the original book")
    for field in (
        WANT_SOURCE,
        "范愛農",
        "unstyled-derived-test-fixture",
        "Public domain in the USA.",
    ):
        if field not in opf:
            fail(f"package manifest is missing {field}")
    text = body_text(chapter)
    collapsed = stripped(text)
    if len(collapsed) < 1500:
        fail(f"opened text is {len(collapsed)} characters")
    if not stripped(text).startswith("范愛農"):
        fail("opened body does not start at 范愛農")
    if "客店" not in collapsed:
        fail("opened body does not contain 客店")
    if "后記" in collapsed or "後記" in collapsed or "我在第三篇" in collapsed:
        fail("opened body continues into the next essay")
    with zipfile.ZipFile(SOURCE) as book:
        source_html = book.read(RECORD["chapter_href"]).decode("utf-8")
    source_body = body_text(source_html)
    source_collapsed = stripped(source_body).replace("\ue4c2", "")
    for paragraph in re.findall(r"(?is)<p[^>]*>(.*?)</p>", chapter):
        piece = stripped(html_unescape(paragraph))
        if piece and piece not in source_collapsed and piece.lstrip("\u3000") not in source_collapsed:
            fail(f"paragraph is not in the source essay: {piece[:40]}")
    if source_collapsed.count("範愛農") != collapsed.count("範愛農"):
        fail("範愛農 was rewritten")
    print(f"opened derived chapter {len(collapsed)} characters")
    print(f"source sha256 {WANT_SOURCE}")
    print(f"derived sha256 {digest}")
    print("original bytes unchanged; no style, script, or iframe")


def html_unescape(text):
    import html

    return html.unescape(re.sub(r"(?is)<[^>]+>", "", text))


if __name__ == "__main__":
    main()
