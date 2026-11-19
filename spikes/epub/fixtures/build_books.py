#!/usr/bin/env python3
"""Build the small EPUBs the reading-view checks open."""

import hashlib
import html
import json
import re
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
FONT = HERE / "Literata.ttf"
OUT = Path("/tmp/superreader-fixtures")
PRIDE = OUT / "pride-se.epub"
ZHAOHUA = OUT / "zhaohua.epub"
ADMISSION = HERE / "chinese-admission.json"
PLAIN_ZH = OUT / "plain-zh.epub"
PLAIN_ZH_MANIFEST = HERE / "plain-zh-manifest.json"
MIN_PLAIN_ZH = 1500
# Gutenberg's line-break marker. It is not a character of the essay.
PARA_MARK = "\ue4c2"


def blocks_from(epub, needle, fallback):
    """Pull real paragraphs from an existing book. `fallback` is not used for 朝花夕拾.

    Stop at the first document that yields the needle. A later file must not be
    appended, and a short result is not a chapter.
    """
    blocks = []
    if epub.exists():
        with zipfile.ZipFile(epub) as book:
            for name in book.namelist():
                if not name.lower().endswith((".xhtml", ".html", ".htm")):
                    continue
                text = book.read(name).decode("utf-8", "replace")
                if needle not in text:
                    continue
                body = re.search(r"(?is)<body[^>]*>(.*)</body>", text)
                inner = body.group(1) if body else text
                for match in re.finditer(r"(?is)<(h[1-6]|p)\b[^>]*>(.*?)</\1>", inner):
                    tag = match.group(1).lower()
                    piece = re.sub(r"(?is)<[^>]+>", "", match.group(2))
                    piece = html.unescape(piece)
                    piece = " ".join(piece.split()).replace(PARA_MARK, "").replace("\ue000", "")
                    if not piece or "Gutenberg" in piece or "LICENSE" in piece:
                        continue
                    kind = tag if tag.startswith("h") else "p"
                    block = (kind, piece)
                    if blocks and blocks[-1] == block:
                        continue
                    blocks.append(block)
                if sum(len(piece) for _, piece in blocks) > 1500:
                    return blocks
                return blocks
    if not blocks and fallback:
        blocks = [("p", fallback)]
    return blocks


def _join_wrap(lines):
    """Join a soft-wrapped line. Do not invent a space inside Chinese text."""
    text = lines[0]
    for line in lines[1:]:
        prev = text[-1] if text else ""
        nxt = line[0] if line else ""
        if prev.isascii() and nxt.isascii() and prev.isalnum() and nxt.isalnum():
            text += " "
        text += line
    return text


def literary_paragraphs(inner):
    """One literary paragraph per edition break. Glyphs and punctuation stay."""
    text = inner.replace("\r\n", "\n").replace("\r", "\n").replace(PARA_MARK, "\n")
    paragraphs = []
    current = []
    for raw in text.split("\n"):
        line = raw.strip(" \t")
        if not line or line.strip("\u3000") == "":
            continue
        if line.startswith("\u3000") and current:
            paragraphs.append(_join_wrap(current))
            current = [line]
        else:
            current.append(line)
    if current:
        paragraphs.append(_join_wrap(current))
    return [paragraph for paragraph in paragraphs if paragraph.strip("\u3000")]


def essay_paragraphs(chapter_html):
    """范愛農 title, its body, and the closing dateline. Not the next essay."""
    body = re.search(r"(?is)<body[^>]*>(.*)</body>", chapter_html)
    inner = body.group(1) if body else chapter_html
    found = re.findall(r"(?is)<p\b[^>]*>(.*?)</p>", inner)
    title = None
    essay = None
    closing = None
    for piece in found:
        plain = re.sub(r"(?is)<[^>]+>", "", piece)
        plain = html.unescape(plain).replace(PARA_MARK, "")
        compact = "".join(plain.split())
        if compact == "范愛農":
            title = "范愛農"
            continue
        if title and essay is None and "客店" in plain and "后記" not in plain and "後記" not in plain:
            essay = literary_paragraphs(piece)
            continue
        if essay is not None and "十一月十八日" in plain:
            for line in plain.replace("\r\n", "\n").replace("\r", "\n").split("\n"):
                line = line.strip(" \t")
                if "十一月十八日" in line and "后記" not in line and "後記" not in line:
                    closing = line.strip().strip("\u3000")
                    break
            break
    if title is None or not essay:
        raise ValueError("范愛農 body was not found; refusing to write a short fixture")
    paragraphs = [title, *essay]
    if closing:
        paragraphs.append(closing)
    stripped = re.sub(r"\s+", "", "".join(paragraphs))
    if len(stripped) < MIN_PLAIN_ZH or "范愛農" not in stripped or "客店" not in stripped:
        raise ValueError(
            f"extracted {len(stripped)} characters; refusing to write a short fixture"
        )
    if "后記" in stripped or "後記" in stripped or "我在第三篇" in stripped:
        raise ValueError("extraction continued into the next essay")
    return paragraphs


def _source_record():
    record = json.loads(ADMISSION.read_text())
    if record.get("sha256") != "91e59f1acb65d7bc77fa2d52fca8b223b893ad837f55c62de3e85fa848e0f293":
        raise ValueError("admission sha is not the original 朝花夕拾 mirror")
    return record


def _remove_short_plain_zh():
    if not PLAIN_ZH.is_file():
        return
    try:
        with zipfile.ZipFile(PLAIN_ZH) as book:
            text = "\n".join(
                book.read(name).decode("utf-8", "replace")
                for name in book.namelist()
                if name.lower().endswith((".xhtml", ".html", ".htm"))
            )
    except zipfile.BadZipFile:
        PLAIN_ZH.unlink()
        return
    stripped = re.sub(r"(?is)<[^>]+>", "", text)
    stripped = re.sub(r"\s+", "", stripped)
    if len(stripped) < MIN_PLAIN_ZH:
        PLAIN_ZH.unlink()


def write_plain_zh(source=ZHAOHUA, dest=PLAIN_ZH):
    """Unstyled derived fixture. Does not modify the original EPUB."""
    record = _source_record()
    if not source.is_file():
        _remove_short_plain_zh()
        raise ValueError(f"missing admitted source {source}")
    raw = source.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != record["sha256"] or len(raw) != record["bytes"]:
        _remove_short_plain_zh()
        raise ValueError("source bytes are not the admitted 朝花夕拾; not writing plain-zh")
    with zipfile.ZipFile(source) as book:
        chapter_name = record["chapter_href"]
        chapter_html = book.read(chapter_name).decode("utf-8")
    before = source.read_bytes()
    paragraphs = essay_paragraphs(chapter_html)
    if source.read_bytes() != before:
        raise ValueError("original 朝花夕拾 bytes changed")
    rule = (
        "From the admitted chapter only: p#id00021 范愛農, then the literary "
        "paragraphs of the following essay body. U+E4C2 is the edition's paragraph "
        "break and is dropped. A line that starts with U+3000 starts a paragraph. "
        "Other line wraps are joined without an added space. The closing dateline "
        "十一月十八日 shares a p with the next essay title; only that dateline line "
        "is kept. No traditional/simplified replacement and no added prose. "
        "This file is an unstyled derived test fixture, not the original EPUB."
    )
    body = "\n".join(f"<p>{html.escape(text)}</p>" for text in paragraphs)
    title = "无样式派生测试夹具"
    head = ""
    chapter = xhtml("zh-CN", title, body, head)
    if any(token in chapter.lower() for token in ("<style", "<link", "<script", "<iframe")):
        raise ValueError("derived chapter would carry style, script, or an iframe")
    items = [("chapter", "chapter.xhtml", "application/xhtml+xml")]
    package = opf("zh-CN", title, items, None, record, rule)
    container = """<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>
"""
    files = {
        "META-INF/container.xml": container.encode(),
        "OEBPS/content.opf": package.encode(),
        "OEBPS/chapter.xhtml": chapter.encode(),
    }
    dest.parent.mkdir(parents=True, exist_ok=True)
    temporary = dest.with_suffix(".epub.partial")
    write_epub(temporary, files)
    derived = hashlib.sha256(temporary.read_bytes()).hexdigest()
    temporary.replace(dest)
    manifest = {
        "kind": "unstyled-derived-test-fixture",
        "not": "the original EPUB",
        "purpose": "unstyled Chinese default rendering",
        "source_url": record["url"],
        "source_sha256": digest,
        "source_chapter": record["chapter"],
        "source_href": chapter_name,
        "source_anchor": record["chapter_anchor"],
        "extraction": rule,
        "rights": record["rights"],
        "derived_sha256": derived,
        "derived_path": str(dest),
        "stripped_chars": len(re.sub(r"\s+", "", "".join(paragraphs))),
    }
    PLAIN_ZH_MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    print(dest)
    return manifest


def render(blocks):
    return "\n".join(f"<{tag}>{html.escape(text)}</{tag}>" for tag, text in blocks)


def xhtml(lang, title, body, extra_head=""):
    return f"""<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="{lang}" lang="{lang}">
<head>
<title>{html.escape(title)}</title>
{extra_head}
</head>
<body>
{body}
</body>
</html>
"""


def opf(lang, title, items, viewport=None, source=None, extraction=""):
    metas = ""
    if viewport:
        width, height = viewport
        metas = (
            '<meta property="rendition:layout">pre-paginated</meta>'
            f'<meta property="rendition:viewport">width={width}, height={height}</meta>'
        )
    if source:
        metas += (
            '<meta name="slowpage:kind" content="unstyled-derived-test-fixture"/>'
            f'<meta name="slowpage:source-sha256" content="{source["sha256"]}"/>'
            f'<meta name="slowpage:source-chapter" content="{html.escape(source["chapter"])}"/>'
            f'<meta name="slowpage:source-href" content="{html.escape(source["chapter_href"])}"/>'
            f'<meta name="slowpage:source-anchor" content="{html.escape(source["chapter_anchor"])}"/>'
            f'<meta name="slowpage:extraction" content="{html.escape(extraction)}"/>'
            f'<dc:rights>{html.escape(source["rights"])}</dc:rights>'
            f'<dc:source>{html.escape(source["url"])}</dc:source>'
            "<dc:description>Unstyled derived test fixture. Not the original EPUB.</dc:description>"
        )
    manifest = "\n".join(
        f'<item id="{item_id}" href="{href}" media-type="{mime}"/>'
        for item_id, href, mime in items
    )
    spine = '<itemref idref="chapter"/>'
    identifier = "superreader-plain-zh-derived" if source else "superreader-fixture"
    return f"""<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:identifier id="id">{identifier}</dc:identifier>
<dc:title>{html.escape(title)}</dc:title>
<dc:language>{lang}</dc:language>
{metas}
</metadata>
<manifest>
{manifest}
</manifest>
<spine>{spine}</spine>
</package>
"""


def write_epub(path, files):
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w") as book:
        info = zipfile.ZipInfo("mimetype")
        info.compress_type = zipfile.ZIP_STORED
        book.writestr(info, "application/epub+zip")
        for name, data in files.items():
            book.writestr(name, data)


def build():
    pride = blocks_from(
        PRIDE,
        "truth universally acknowledged",
        "It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.",
    )
    container = """<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>
"""
    styled_css = """
@font-face { font-family: "Literata"; src: url("Literata.ttf") format("truetype"); }
p { font-family: "Literata", serif; font-size: 19px; line-height: 1.65; text-indent: 1.5em; text-align: left; margin: 0 0 0.8em 0; }
h1, h2, h3 { font-family: "Literata", serif; font-size: 19px; text-align: center; text-indent: 0; font-weight: 600; margin: 0 0 1em 0; }
"""
    publisher_css = styled_css.replace("text-align: left", "text-align: justify")
    books = {
        "plain-en.epub": (
            "en",
            "Plain English",
            render(pride),
            "",
            None,
            [],
        ),
        "styled.epub": (
            "en",
            "Styled Literata",
            render(pride),
            '<link rel="stylesheet" href="style.css"/>',
            None,
            [("style", "style.css", "text/css"), ("font", "Literata.ttf", "font/ttf")],
        ),
        "publisher.epub": (
            "en",
            "Publisher Justified",
            render(pride),
            '<link rel="stylesheet" href="style.css"/>',
            None,
            [("style", "style.css", "text/css"), ("font", "Literata.ttf", "font/ttf")],
        ),
        "fixed.epub": (
            "en",
            "Fixed Square",
            render(pride),
            '<meta name="viewport" content="width=1000, height=1000"/>',
            (1000, 1000),
            [],
        ),
    }
    for name, spec in books.items():
        if spec is None:
            print(f"not written {name}: source under 1500 characters")
            continue
        lang, title, body, head, viewport, extra = spec
        items = [("chapter", "chapter.xhtml", "application/xhtml+xml"), *extra]
        files = {
            "META-INF/container.xml": container.encode(),
            "OEBPS/content.opf": opf(lang, title, items, viewport).encode(),
            "OEBPS/chapter.xhtml": xhtml(lang, title, body, head).encode(),
        }
        if name == "styled.epub":
            files["OEBPS/style.css"] = styled_css.encode()
            files["OEBPS/Literata.ttf"] = FONT.read_bytes()
        if name == "publisher.epub":
            files["OEBPS/style.css"] = publisher_css.encode()
            files["OEBPS/Literata.ttf"] = FONT.read_bytes()
        write_epub(OUT / name, files)
        print(OUT / name)
    try:
        write_plain_zh()
    except ValueError as exc:
        _remove_short_plain_zh()
        print(f"not written plain-zh.epub: {exc}")


if __name__ == "__main__":
    build()
