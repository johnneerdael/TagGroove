"""Reproduce a bounded feature inventory from the separately supplied release.

No executable code, icons, translations, or templates are copied into the app.
Use: python3 docs/evidence/inventory.py ../
"""
import hashlib
import json
import re
import struct
import sys
from pathlib import Path


def menu_items(data, pos, extended):
    result = []
    while pos < len(data):
        if extended:
            pos = (pos + 3) & ~3
            item_type, state, command, flags = struct.unpack_from("<IIIH", data, pos)
            pos += 14
            popup = bool(flags & 1)
        else:
            flags = struct.unpack_from("<H", data, pos)[0]
            pos += 2
            popup = bool(flags & 16)
            command = None
            if not popup:
                command = struct.unpack_from("<H", data, pos)[0]
                pos += 2
        end = pos
        while end + 1 < len(data) and data[end:end + 2] != b"\0\0":
            end += 2
        if end + 1 >= len(data):
            raise ValueError("Unterminated menu label")
        label = data[pos:end].decode("utf-16-le")
        pos = end + 2
        item = {"command_id": command, "label": label}
        if popup:
            if extended:
                pos = ((pos + 3) & ~3) + 4
            item["children"], pos = menu_items(data, pos, extended)
        result.append(item)
        if flags & 128:
            return result, pos
    raise ValueError("Menu ended without terminal item flag")


def build(root):
    resource = json.loads(Path(__file__).with_name("rea-resources.json").read_text())
    binary = (root / "Mp3tag.exe").read_bytes()
    digest = hashlib.sha256(binary).hexdigest()
    if digest != resource["normalized_result"]["artifact"]["sha256"]:
        raise ValueError("Target digest differs from REA resource evidence")
    menus = []
    for r in resource["normalized_result"]["resources"]:
        if r["type"].get("id") != 4:
            continue
        loc = r["payload"]["location"]
        data = binary[loc["offset"]:loc["offset"] + loc["bytes"]]
        version, offset = struct.unpack_from("<HH", data)
        items, _ = menu_items(data, 4 + offset, version == 1)
        menus.append({"id": r["name"], "file_offset": loc["offset"], "items": items})
    language = (root / "lang/English.lng").read_text(encoding="utf-8-sig")
    keys = [line.split(" ", 1)[0] for line in language.splitlines() if line]
    action_types = sorted({int(t) for p in (root / "data/actions").glob("*.mta")
                           for t in re.findall(r"^T=(\d+)", p.read_text(encoding="cp1252"), re.M)})
    script_files = {}
    for p in sorted((root / "data/sources").glob("*")):
        text = p.read_text(encoding="utf-8-sig", errors="replace")
        script_files[p.name] = {"sha256": hashlib.sha256(p.read_bytes()).hexdigest(),
                               "directives": sorted(set(re.findall(r"^([a-z_]+)\s", text, re.M)))}
    return {"target": {"name": "Mp3tag.exe", "version": "3.36.1", "sha256": digest},
            "evidence_id": resource["evidence_id"], "menus": menus,
            "language_count": len(list((root / "lang").glob("*.lng"))),
            "english_language_keys": keys, "shipped_action_types": action_types,
            "source_scripts": script_files,
            "export_templates": [p.name for p in sorted((root / "export").glob("*.mte"))],
            "limitations": ["Static menu and configuration inventory, not observed execution.",
                            "Shipped examples do not enumerate all supported action or script types."]}


if __name__ == "__main__":
    output = Path(__file__).with_name("feature-inventory.json")
    output.write_text(json.dumps(build(Path(sys.argv[1]).resolve()), indent=2) + "\n")
    print(output)
