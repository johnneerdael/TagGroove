#!/usr/bin/env python3
"""Check the supplied brand archive and every integrated resource copy."""
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT / "assets/brand"
KIT = BRAND / "TagGroove-Brand-Kit"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify():
    provenance = json.loads((BRAND / "PROVENANCE.json").read_text())
    manifest = json.loads((KIT / "asset-manifest.json").read_text())
    for entry in provenance["sources"]:
        assert digest(ROOT / entry["preserved"]) == entry["sha256"], entry
    for entry in manifest["files"]:
        assert digest(KIT / entry["path"]) == entry["sha256"], entry["path"]
    for entry in provenance["asset_copies"]:
        source, destination = BRAND / entry["source"], ROOT / entry["destination"]
        assert digest(destination) == entry["sha256"], entry["destination"]
        assert digest(source) == entry.get("source_sha256", entry["sha256"]), entry["source"]
        if entry.get("transformation") == "retain existing Android opentag_ resource identifiers":
            assert source.read_text().replace("taggroove_", "opentag_") == destination.read_text()
        elif entry.get("transformation"):
            expected = json.loads(source.read_text())
            expected.update(start_url="/", scope="/")
            assert json.loads(destination.read_text()) == expected
        else:
            assert source.read_bytes() == destination.read_bytes()
    for resources in [ROOT / "src-tauri/icons/android", ROOT / "src-tauri/gen/android/app/src/main/res"]:
        for path in resources.glob("mipmap-anydpi-*/*.xml"):
            for element in ET.parse(path).iter():
                for value in element.attrib.values():
                    if value.startswith("@drawable/"):
                        assert (resources / "drawable" / (value.split("/", 1)[1] + ".xml")).is_file(), path
    config = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())
    assert config["productName"] == "TagGroove"
    assert config["identifier"] == "org.opentag.editor", "Keep existing application data and upgrade identity"
    assert 'pub const APPLICATION: &str = "OpenTagAudioTagEditor";' in (ROOT / "src-tauri/src/sources/freedb.rs").read_text()
    print(f"Verified {len(manifest['files'])} supplied files and {len(provenance['asset_copies'])} integrated copies; app and gnudb identities unchanged.")


if __name__ == "__main__":
    verify()
