"""Keep old guide URLs working after pages move.

`redirects.json` maps an old page (`learn/regex-from-zero.md`) to the page that
replaced it (`scripting/regex-from-zero.md`). The build fails if a target does
not exist or an old path still exists as a real page, so a redirect can never
hide content. Each old URL gets a small static page that forwards to the new
one and keeps the query string and #anchor.
"""

import html
import json
import os
import posixpath

from mkdocs.exceptions import PluginError
from mkdocs.structure.files import File

DEFAULT_MAP = "docs/hooks/redirects.json"
_moves = {}


def _map_path(config):
    base = os.path.dirname(os.path.abspath(config.config_file_path))
    return os.path.join(base, config.get("extra", {}).get("redirects_file", DEFAULT_MAP))


def on_files(files, config):
    with open(_map_path(config), encoding="utf-8") as handle:
        moves = json.load(handle)
    pages = {file.src_uri for file in files.documentation_pages()}
    for old, new in moves.items():
        if new not in pages:
            raise PluginError(f"redirect {old} -> {new}: the target is not a published page")
        if old in pages:
            raise PluginError(f"redirect {old}: a page still exists at the old path")
    _moves.clear()
    _moves.update(moves)
    return files


def _page(target, canonical):
    escaped = html.escape(target, quote=True)
    script_target = json.dumps(target)
    link = f'<link rel="canonical" href="{html.escape(canonical, quote=True)}">\n' if canonical else ""
    return (
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n"
        "<title>This page has moved</title>\n"
        f"{link}<meta name=\"robots\" content=\"noindex\">\n"
        f"<meta http-equiv=\"refresh\" content=\"0; url={escaped}\">\n"
        f"<script>location.replace({script_target} + location.search + location.hash);</script>\n"
        "</head>\n<body>\n"
        f"<p>This page has moved to <a href=\"{escaped}\">{escaped}</a>.</p>\n"
        "</body>\n</html>\n"
    )


def on_post_build(config):
    site_url = (config.get("site_url") or "").rstrip("/")
    use_dirs = config["use_directory_urls"]
    for old, new in _moves.items():
        old_file = File(old, config["docs_dir"], config["site_dir"], use_dirs)
        new_file = File(new, config["docs_dir"], config["site_dir"], use_dirs)
        destination = os.path.join(config["site_dir"], old_file.dest_uri)
        if os.path.exists(destination):
            raise PluginError(f"redirect {old}: the built site already has {old_file.dest_uri}")
        start = posixpath.dirname(old_file.dest_uri) or "."
        target = posixpath.relpath(new_file.dest_uri, start)
        if use_dirs and new_file.dest_uri.endswith("/index.html"):
            target = posixpath.relpath(posixpath.dirname(new_file.dest_uri) or ".", start) + "/"
        canonical = f"{site_url}/{new_file.url}" if site_url else ""
        os.makedirs(os.path.dirname(destination), exist_ok=True)
        with open(destination, "w", encoding="utf-8") as handle:
            handle.write(_page(target, canonical))
