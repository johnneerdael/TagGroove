"""Publish reviewed static guide bytes; never execute the downloaded artifact."""
import argparse
import hashlib
import html
import http.client
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import stat
import subprocess
import tempfile
import time
from urllib.parse import urlsplit

from review_gate import GitHub, check_release


CANONICAL = "https://johnneerdael.github.io/audio-tag-guide/"
SOURCE_PROJECT = "johnneerdael/opentag"
SOURCE_REPOSITORY = f"https://github.com/{SOURCE_PROJECT}"
TARGET = "git@github.com:johnneerdael/audio-tag-guide.git"
MARKER = "publication-source.json"
SIGNATURE = MARKER + ".sig"
SIGNERS = Path(__file__).resolve().parents[1] / "guide-pages-allowed-signers"
PRINCIPAL = "opentag-guide-publisher"
NAMESPACE = "opentag-guide-publication-v1"
KNOWN_HOSTS = Path(__file__).resolve().parents[1] / "guide-pages-known-hosts"
TARGET_TEMPLATE = Path(__file__).resolve().parents[1] / "guide-pages-target.yml"
TARGET_WORKFLOW = ".github/workflows/static-pages.yml"


def identity(source, head, pr):
    if not all(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{40}", value) for value in (source, head)):
        raise ValueError("Publication requires full source and reviewed-head SHAs")
    if not isinstance(pr, int) or isinstance(pr, bool) or pr <= 0:
        raise ValueError("Publication requires a positive reviewed PR number")


def relative_path(name):
    if name.casefold() == "cname":
        raise ValueError("The artifact cannot change the fixed Pages domain")
    if (not re.fullmatch(r"[A-Za-z0-9_.\-/]+", name)
            or name.startswith("/") or any(part in ("", ".", "..") for part in name.split("/"))):
        raise ValueError(f"Unsafe static path: {name}")
    parts = name.split("/")
    if any(part.startswith(".") or part in ("_meta", "_snippets") for part in parts):
        if name != ".nojekyll":
            raise ValueError(f"Nonpublic or repository-control path: {name}")


def files(site):
    if site.is_symlink() or not site.is_dir():
        raise ValueError("Static artifact must be a real directory")
    result = []
    for directory, dirs, names in os.walk(site, followlinks=False):
        for name in dirs + names:
            path = Path(directory) / name
            relative_path(path.relative_to(site).as_posix())
            info = path.lstat()
            if stat.S_ISLNK(info.st_mode):
                raise ValueError("Static artifacts cannot contain symlinks")
            if stat.S_ISDIR(info.st_mode):
                continue
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                raise ValueError("Static artifacts must contain only independent regular files")
            result.append(path)
    return sorted(result)


def tree_digest(site):
    rows = [f"{path.relative_to(site).as_posix()} {hashlib.sha256(path.read_bytes()).hexdigest()}"
            for path in files(site) if path.relative_to(site).as_posix() not in (MARKER, SIGNATURE)]
    return hashlib.sha256("\n".join(rows).encode()).hexdigest()


def routes(site):
    for name in ("index.html", "404.html", "search/search_index.json"):
        if not (site / name).is_file():
            raise ValueError(f"Static artifact is missing {name}")
    index = json.loads((site / "search/search_index.json").read_text())
    if not isinstance(index, dict) or not isinstance(index.get("docs"), list) or not index["docs"]:
        raise ValueError("Static search index has no document list")
    result = set()
    for document in index["docs"]:
        if not isinstance(document, dict) or not isinstance(document.get("location"), str):
            raise ValueError("Invalid static search document")
        location = urlsplit(document["location"])
        if location.scheme or location.netloc or location.query:
            raise ValueError("Search routes must remain inside the static guide")
        route = location.path
        if route:
            relative_path(route.rstrip("/"))
        target = route + "index.html" if not route or route.endswith("/") else route
        if not (site / target).is_file() or not target.endswith(".html"):
            raise ValueError(f"Search route has no HTML page: {route}")
        result.add(route)
    if "" not in result:
        raise ValueError("Static search index is missing the homepage")
    return result


def redirect_page(route):
    fallback = CANONICAL + route
    # Fixed host/base; pathname is data assigned below that base, never a URL input.
    script = """const target = new URL("https://johnneerdael.github.io/audio-tag-guide/");
const prefix = "/opentag/";
const route = location.pathname.startsWith(prefix) ? location.pathname.slice(prefix.length) : "";
target.pathname += route;
target.search = location.search;
target.hash = location.hash;
location.replace(target.href);"""
    return ("<!doctype html>\n<html lang=\"en\"><meta charset=\"utf-8\">\n"
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n"
            "<title>OpenTag guide has moved</title>\n"
            f"<link rel=\"canonical\" href=\"{html.escape(fallback, quote=True)}\">\n"
            "<h1>The OpenTag guide has moved</h1>\n"
            f"<p><a href=\"{html.escape(fallback, quote=True)}\">Continue to the user guide</a>.</p>\n"
            "<noscript><p>Follow the link to continue without JavaScript.</p></noscript>\n"
            f"<script>{script}</script></html>\n")


def prepare(site, redirects, source, head, pr):
    identity(source, head, pr)
    files(site)
    public_routes = routes(site)
    if redirects.exists():
        raise ValueError("Redirect output must be a new directory")
    if site.resolve() in redirects.resolve().parents or redirects.resolve() in site.resolve().parents:
        raise ValueError("Static and redirect outputs must be separate")
    (site / ".nojekyll").write_text("")
    marker = dict(schema=1, source_repository=SOURCE_REPOSITORY, source_commit=source,
                  reviewed_head=head, reviewed_pr=pr, canonical_url=CANONICAL,
                  completed_pages=len(public_routes), static_tree_sha256=tree_digest(site),
                  target_workflow_sha256=hashlib.sha256(TARGET_TEMPLATE.read_bytes()).hexdigest())
    (site / MARKER).write_text(json.dumps(marker, indent=2) + "\n")
    redirects.mkdir(parents=True)
    for path in files(site):
        if path.suffix != ".html":
            continue
        relative = path.relative_to(site)
        route = relative.as_posix()
        if route.endswith("index.html"):
            route = route[:-len("index.html")]
        if route == "404.html":
            route = ""
        output = redirects / relative
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(redirect_page(route))
    (redirects / ".nojekyll").write_text("")
    validate_site(site, source, head, pr)
    return marker


def validate_site(site, source, head, pr):
    identity(source, head, pr)
    files(site)
    public_routes = routes(site)
    marker = json.loads((site / MARKER).read_text())
    expected = dict(schema=1, source_repository=SOURCE_REPOSITORY, source_commit=source,
                    reviewed_head=head, reviewed_pr=pr, canonical_url=CANONICAL,
                    completed_pages=len(public_routes), static_tree_sha256=tree_digest(site),
                    target_workflow_sha256=hashlib.sha256(TARGET_TEMPLATE.read_bytes()).hexdigest())
    if marker != expected or not (site / ".nojekyll").is_file() or (site / ".nojekyll").read_bytes():
        raise ValueError("Static artifact provenance or content digest does not match the approved source")
    return expected


def git(directory, *args, env=None):
    return subprocess.run([
        "git", "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false",
        "-c", "user.name=OpenTag guide publisher", "-c", "user.email=docs@users.noreply.github.com",
        "-C", str(directory), *args], env=env, check=True, capture_output=True, text=True).stdout.strip()


def target_control(checkout):
    controls = files(checkout / ".github")
    expected = checkout / TARGET_WORKFLOW
    if controls != [expected] or expected.read_bytes() != TARGET_TEMPLATE.read_bytes():
        raise ValueError("Target deployment control must match the reviewed source template")


def authorize_source(source, head, pr, api=None):
    """Only SOURCE's authenticated release job can approve a signature/push."""
    api = api or GitHub(SOURCE_PROJECT)
    if api.repo != SOURCE_PROJECT or api.get("git/ref/heads/main")["object"]["sha"] != source:
        raise RuntimeError("Only current source main can authorize neutral publication")
    check_release(api, pr, head, source)
    if api.get("git/ref/heads/main")["object"]["sha"] != source:
        raise RuntimeError("Source main changed during publication authorization")


def sign_publication(site, private):
    result = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", str(private), "-n", NAMESPACE],
                            input=(site / MARKER).read_bytes(), capture_output=True, check=True,
                            env={name: value for name, value in os.environ.items()
                                 if name not in ("DOCS_PAGES_DEPLOY_KEY", "GH_TOKEN", "GITHUB_TOKEN")})
    (site / SIGNATURE).write_bytes(result.stdout)


def current_source():
    result = subprocess.run(["git", "-c", "core.hooksPath=/dev/null", "ls-remote",
                             SOURCE_REPOSITORY + ".git", "refs/heads/main"],
                            env={**{name: value for name, value in os.environ.items()
                                    if name not in ("DOCS_PAGES_DEPLOY_KEY", "GH_TOKEN", "GITHUB_TOKEN")},
                                 "GIT_TERMINAL_PROMPT": "0"},
                            capture_output=True, text=True, check=True, timeout=30)
    match = re.fullmatch(r"([0-9a-f]{40})\trefs/heads/main\n?", result.stdout)
    if not match:
        raise RuntimeError("Cannot verify current source main")
    return match[1]


def authorize_target(site, checkout):
    """Verify SOURCE's signed approval receipt; never access its scoped token."""
    target_control(checkout)
    if site.resolve() != (checkout / "site").resolve():
        raise ValueError("Only the target's site directory may be packaged")
    files(site)
    marker = json.loads((site / MARKER).read_text())
    if not isinstance(marker, dict):
        raise ValueError("Publication provenance must be an object")
    source, head, pr = (marker.get(name) for name in ("source_commit", "reviewed_head", "reviewed_pr"))
    validate_site(site, source, head, pr)
    signature = site / SIGNATURE
    if not signature.is_file() or signature.stat().st_size > 16384:
        raise ValueError("A bounded source publication signature is required")
    subprocess.run(["ssh-keygen", "-Y", "verify", "-f", str(SIGNERS), "-I", PRINCIPAL,
                    "-n", NAMESPACE, "-s", str(signature)],
                   input=(site / MARKER).read_bytes(), capture_output=True, check=True)
    if current_source() != source:
        raise RuntimeError("Signed publication is no longer current source main")
    return marker


def publish_checkout(site, checkout, source, head, pr, env=None, before_push=None):
    validate_site(site, source, head, pr)
    if (checkout / ".git").is_symlink() or not (checkout / ".git").is_dir():
        raise ValueError("Publisher requires its own ordinary Git clone")
    target_control(checkout)
    published = checkout / "site"
    if published.is_symlink():
        raise ValueError("Target site directory cannot be a symlink")
    if published.exists():
        if not published.is_dir():
            raise ValueError("Target site must be a directory")
        shutil.rmtree(published)
    published.mkdir()
    for path in files(site):
        output = published / path.relative_to(site)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, output)
    git(checkout, "add", "--all", "--", "site", env=env)
    changes = git(checkout, "diff", "--cached", "--name-only", env=env)
    if changes:
        git(checkout, "commit", "-m", f"Publish user guide from {source}", env=env)
        if before_push is not None:
            before_push()
        # No force option: a concurrent target update must fail instead of being erased.
        git(checkout, "push", "origin", "HEAD:main", env=env)
    return git(checkout, "rev-parse", "HEAD", env=env)


def publish(site, source, head, pr):
    validate_site(site, source, head, pr)
    key = os.environ.get("DOCS_PAGES_DEPLOY_KEY", "")
    if not key:
        raise ValueError("The github-pages environment deployment key is required")
    with tempfile.TemporaryDirectory(prefix="guide-publisher-") as directory:
        root = Path(directory)
        private = root / "deploy-key"
        private.write_text(key + "\n")
        private.chmod(0o600)
        hosts = root / "known-hosts"
        hosts.write_bytes(KNOWN_HOSTS.read_bytes())
        env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_SSH_COMMAND": shlex.join([
            "ssh", "-i", str(private), "-o", "IdentitiesOnly=yes", "-o", "StrictHostKeyChecking=yes",
            "-o", "HostKeyAlgorithms=ssh-ed25519", "-o", f"UserKnownHostsFile={hosts}"])}
        # Keep the secret out of the Git/SSH child environment and all command arguments.
        for name in ("DOCS_PAGES_DEPLOY_KEY", "GH_TOKEN", "GITHUB_TOKEN"):
            env.pop(name, None)
        checkout = root / "target"
        subprocess.run(["git", "-c", "core.hooksPath=/dev/null", "clone", "--depth=1",
                        "--single-branch", "--branch=main", TARGET, str(checkout)],
                       env=env, check=True, capture_output=True, text=True)
        target_control(checkout)
        authorize_source(source, head, pr)
        sign_publication(site, private)
        return publish_checkout(site, checkout, source, head, pr, env=env,
                                before_push=lambda: authorize_source(source, head, pr))


def fetch_marker(source):
    connection = http.client.HTTPSConnection("johnneerdael.github.io", timeout=10)
    try:
        connection.request("GET", f"/audio-tag-guide/{MARKER}?source={source}",
                           headers={"Cache-Control": "no-cache", "User-Agent": "reviewed-guide-publisher"})
        response = connection.getresponse()
        if response.status != 200:
            raise OSError(f"Neutral publication marker returned HTTP {response.status}")
        body = response.read(65537)
        if len(body) > 65536:
            raise ValueError("Publication marker exceeds its size limit")
        return body
    finally:
        connection.close()


def wait_for_publication(expected, fetch=None, timeout=660):
    fetch = fetch or (lambda: fetch_marker(expected["source_commit"]))
    deadline = time.monotonic() + timeout
    while True:
        try:
            if json.loads(fetch()) == expected:
                return
        except (OSError, ValueError, http.client.HTTPException):
            pass
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        time.sleep(min(8, remaining))
    raise TimeoutError("Neutral Pages has not published the expected source/tree; legacy deployment remains blocked")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["prepare", "publish", "wait", "verify-target"])
    parser.add_argument("--site", type=Path, required=True)
    parser.add_argument("--redirects", type=Path)
    parser.add_argument("--checkout", type=Path)
    parser.add_argument("--source")
    parser.add_argument("--head")
    parser.add_argument("--pr", type=int)
    args = parser.parse_args()
    if args.command == "verify-target":
        if args.checkout is None:
            parser.error("verify-target requires --checkout")
        marker = authorize_target(args.site, args.checkout)
        print(f"Source review and current main authorize {marker['source_commit']}")
        return
    if args.source is None or args.head is None or args.pr is None:
        parser.error("prepare, publish and wait require --source, --head and --pr")
    if args.command == "prepare":
        if args.redirects is None:
            parser.error("prepare requires --redirects")
        prepare(args.site, args.redirects, args.source, args.head, args.pr)
    elif args.command == "publish":
        print(f"Static publication commit: {publish(args.site, args.source, args.head, args.pr)}")
    else:
        marker = validate_site(args.site, args.source, args.head, args.pr)
        wait_for_publication(marker)
        print(f"Neutral Pages serves reviewed source {args.source}")


if __name__ == "__main__":
    main()
