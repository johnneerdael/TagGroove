"""Use Ubuntu's geographic mirrors with canonical fallback on ephemeral runners."""
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

ARCHIVE = "http://archive.ubuntu.com/ubuntu/"


def configure(root: Path, listing: str, architecture: str = "amd64") -> None:
    config = root / "apt.conf.d"
    config.mkdir(exist_ok=True)
    (config / "80taggroove-network").write_text(
        'Acquire::Retries "2";\nAcquire::http::Timeout "20";\nAcquire::https::Timeout "20";\n')
    # Geographic archive mirrors need not carry Ubuntu ports architectures.
    if architecture != "amd64":
        print(f"Kept {architecture} repositories; configured network timeouts")
        return
    mirrors = []
    for line in listing.splitlines():
        uri = line.strip()
        try:
            parsed = urlsplit(uri)
        except ValueError:
            continue
        if (parsed.scheme not in {"http", "https"} or not parsed.hostname
                or parsed.username or parsed.password or parsed.query or parsed.fragment
                or any(character.isspace() for character in uri)):
            continue
        uri = uri.rstrip("/") + "/"
        if uri != ARCHIVE and uri not in mirrors:
            mirrors.append(uri)
    mirrorlist = root / "taggroove-mirrors.txt"
    mirrorlist.write_text("".join(uri + "\tpriority:1\n" for uri in mirrors)
                          + ARCHIVE + "\tpriority:10\n")
    mirrorlist.chmod(0o644)
    sources = [root / "sources.list"]
    directory = root / "sources.list.d"
    if directory.exists():
        sources.extend(directory.glob("*.list"))
        sources.extend(directory.glob("*.sources"))
    # Preserve Signed-By, security updates, ARM ports and third-party sources.
    pattern = (r"(?:https?://(?:azure\.)?archive\.ubuntu\.com/ubuntu/?"
               r"|mirror\+file:/etc/apt/apt-mirrors\.txt)(?=\s|$)")
    for source in sources:
        if source.is_file():
            original = source.read_text()
            updated = re.sub(pattern, lambda _: "mirror+file:" + str(mirrorlist), original)
            if updated != original:
                source.write_text(updated)
    print(f"Configured {len(mirrors)} geographic Ubuntu mirrors and the main archive fallback")


if __name__ == "__main__":
    architecture = subprocess.check_output(["dpkg", "--print-architecture"], text=True).strip()
    configure(Path("/etc/apt"), Path(sys.argv[1]).read_text(), architecture)
