"""Validate the public GitHub Pages files without dependencies."""

import json
import re
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urljoin, urlsplit
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "site"
BASE = "https://dev-universe.github.io/fender-studio-pro-mcp/"
REPO = "https://github.com/dev-universe/fender-studio-pro-mcp"
LANGUAGES = {"en": BASE, "ko": BASE + "ko/", "x-default": BASE}
PAGES = {"en": SITE / "index.html", "ko": SITE / "ko/index.html"}
LOCALS = {"en": "en_US", "ko": "ko_KR"}
SITEMAP = "http://www.sitemaps.org/schemas/sitemap/0.9"
XHTML = "http://www.w3.org/1999/xhtml"


class Page(HTMLParser):
    def __init__(self, path):
        super().__init__(convert_charrefs=True)
        self.tags = []
        self.schema = []
        self.reading_schema = False
        self.feed(path.read_text(encoding="utf-8"))

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        self.tags.append((tag, attrs))
        if tag == "script":
            assert attrs.get("type") == "application/ld+json"
            self.reading_schema = True

    def handle_endtag(self, tag):
        if tag == "script":
            self.reading_schema = False

    def handle_data(self, text):
        if self.reading_schema:
            self.schema.append(json.loads(text))

    def attrs(self, tag):
        return [attrs for current, attrs in self.tags if current == tag]


def verify_repo_link(url):
    assert url == REPO or url.startswith(REPO + "/") or url.startswith(REPO + "#"), url
    parsed = urlsplit(url)
    path = parsed.path.removeprefix(urlsplit(REPO).path)
    if path.startswith("/blob/main/"):
        file = ROOT / unquote(path.removeprefix("/blob/main/"))
    else:
        assert not path or path == "/", url
        file = ROOT / "README.md"
    assert file.is_file(), url
    if not parsed.fragment:
        return
    headings = re.findall(r"^#{1,6} (.+)$", file.read_text(encoding="utf-8"), re.M)
    slugs = [re.sub(r"[^\w\- ]", "", title.lower()).replace(" ", "-") for title in headings]
    assert unquote(parsed.fragment) in slugs, url


def verify_page(language, path):
    page = Page(path)
    current = LANGUAGES[language]
    alternate = "ko" if language == "en" else "en"
    assert page.attrs("html") == [{"lang": language}]
    meta = {attrs.get("name") or attrs.get("property"): attrs.get("content") for attrs in page.attrs("meta")}
    links = page.attrs("link")
    assert len(page.attrs("title")) == len(page.attrs("h1")) == 1
    assert len(page.attrs("h3")) >= 4
    assert meta["description"] and meta["robots"].startswith("index, follow")
    assert meta["og:url"] == current and meta["og:locale"] == LOCALS[language]
    assert meta["og:locale:alternate"] == LOCALS[alternate]
    assert meta["twitter:card"] == "summary_large_image"
    assert [link["href"] for link in links if link.get("rel") == "canonical"] == [current]
    assert {link["hreflang"]: link["href"] for link in links if link.get("rel") == "alternate"} == LANGUAGES
    assert len(page.schema) == 1
    schema = page.schema[0]
    assert schema["@type"] == "SoftwareSourceCode" and schema["codeRepository"] == REPO
    assert (schema["url"], schema["inLanguage"]) == (current, language)
    assert schema["license"] == REPO + "/blob/main/LICENSE"

    ids = {attrs["id"] for _, attrs in page.tags if "id" in attrs}
    anchors = page.attrs("a")
    assert [urljoin(current, a["href"]) for a in anchors if a.get("hreflang") == alternate] == [LANGUAGES[alternate]]
    assets = [meta["og:image"]]
    assets.extend(link["href"] for link in links if link.get("rel") in {"icon", "stylesheet"})
    for img in page.attrs("img"):
        assert img.get("alt") and img.get("width") and img.get("height")
        assets.append(img["src"])
    for asset in assets:
        url = urljoin(current, asset)
        assert url.startswith(BASE) and (SITE / url.removeprefix(BASE)).is_file(), url
    for a in anchors:
        target = a["href"]
        resolved = urljoin(current, target)
        if target.startswith("#"):
            assert target[1:] in ids, target
        elif not resolved.startswith(BASE):
            verify_repo_link(resolved)
        else:
            assert resolved in {BASE, BASE + "ko/"}, resolved
    print(f"Verified {language} HTML, navigation, metadata, structured data, and evidence links")


def main():
    for language, path in PAGES.items():
        verify_page(language, path)
    sitemap = ElementTree.parse(SITE / "sitemap.xml").getroot()
    records = sitemap.findall(f"{{{SITEMAP}}}url")
    assert {record.findtext(f"{{{SITEMAP}}}loc") for record in records} == {BASE, BASE + "ko/"}
    assert len(records) == len(PAGES)
    for record in records:
        assert {
            href.attrib["hreflang"]: href.attrib["href"] for href in record.findall(f"{{{XHTML}}}link")
        } == LANGUAGES
    links = re.findall(r"\]\((https://[^)]+)\)", (SITE / "llms.txt").read_text(encoding="utf-8"))
    assert set(links) >= {BASE, BASE + "ko/", REPO}
    for link in links:
        if link not in {BASE, BASE + "ko/"}:
            verify_repo_link(link)
    print("Verified both sitemap entries and the public llms.txt links")


if __name__ == "__main__":
    main()
