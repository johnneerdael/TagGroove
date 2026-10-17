"""Developer-only capture from unmodified upstream fpcalc; never run by tests.

Supply a fresh source-built x86_64 macOS/Linux native prefix. The output is a
separate forensic build with file input enabled only for the reference CLI.
"""

import argparse
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prefix", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    product = args.prefix.resolve()
    work = product.parent
    root = args.output.resolve()
    if " " in str(root):
        parser.error("FFmpeg's reference build directory must not contain spaces")
    manifest = json.loads((product / "opentag-audio-manifest.json").read_text())
    target = manifest["target"]
    if target not in {"x86_64-apple-darwin", "x86_64-unknown-linux-gnu"}:
        parser.error("This capture supports x86_64 macOS/Linux prefixes")
    recipe = Path(__file__).resolve().parents[4] / "scripts/audio-deps/build.py"
    module_spec = importlib.util.spec_from_file_location("native_audio_builder", recipe)
    builder = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(builder)
    pins = json.loads(recipe.with_name("sources.json").read_text())
    builder.validate_prefix(product, pins, manifest["recipe_sha256"],
                            builder.recipe_inputs(pins), target)
    source_info = manifest["components"]["chromaprint"]
    source = work / "sources" / source_info["directory"]
    archive = work.parent.parent / "downloads" / source_info["archive"]
    require(digest(archive) == source_info["sha256"], "Original source archive hash differs")
    source_files = 0
    with tarfile.open(archive) as handle:
        for member in handle:
            if member.isfile():
                relative = Path(member.name).relative_to(source_info["directory"])
                require((source / relative).read_bytes() == handle.extractfile(member).read(),
                        "Upstream CLI source differs: " + member.name)
                source_files += 1
    # Never overwrite or modify the shipping prefix or its build tree.
    root.mkdir(parents=True, exist_ok=False)
    prefix, build = root / "prefix", root / "ffmpeg-build"
    shutil.copytree(product, prefix, symlinks=True)
    (prefix / "opentag-audio-manifest.json").rename(prefix / "product-input-manifest.json")
    shutil.copytree(work / "ffmpeg-build", build, symlinks=True)
    log = root / "build.log"

    def run(command, cwd=root):
        with log.open("ab") as output:
            output.write(("\n" + repr([str(x) for x in command]) + "\n").encode())
            output.flush()
            subprocess.run([str(x) for x in command], cwd=cwd, stdout=output,
                           stderr=subprocess.STDOUT, check=True)

    command = next(row for row in manifest["configure_commands"]
                   if len(row) > 1 and row[1].endswith("/configure"))
    command = [value.replace(str(product), str(prefix)) for value in command]
    command.append("--enable-protocol=file")
    run(command, build)
    components = (build / "config_components.h").read_text()
    require(re.findall(r"^#define CONFIG_(\w+)_PROTOCOL 1$", components, re.M) == ["FILE"],
            "Reference input transport must enable only the file protocol")
    # Codec, transform and Chromaprint archives remain the product bytes.
    # Only libavformat's input transport changes for the independent reader.
    run(["make", "-j2", "libavformat/libavformat.a"], build)
    shutil.copyfile(build / "libavformat/libavformat.a", prefix / "lib/libavformat.a")
    names = ["chromaprint", "avformat", "avcodec", "swresample", "avutil", "z", "bz2"]
    libraries = [prefix / f"lib/lib{name}.a" for name in names]
    unchanged = {name: digest(product / f"lib/lib{name}.a") for name in names if name != "avformat"}
    for name, expected in unchanged.items():
        require(digest(prefix / f"lib/lib{name}.a") == expected,
                "Reference changed a codec/transform archive: " + name)
    cli_source, = source.rglob("fpcalc.cpp")
    apple = "apple" in target
    compiler = ["/usr/bin/clang++", "-arch", "x86_64"] if apple else ["c++"]
    system = ["-framework", "Accelerate", "-liconv", "-lm"] if apple else ["-lm", "-pthread"]
    binary = root / "fpcalc-upstream"
    run([*compiler, "-std=c++17", "-O2", "-DUSE_SWRESAMPLE", "-DCHROMAPRINT_NODLL",
         "-I" + str(source / "src"), "-I" + str(work / "chromaprint-build"),
         "-I" + str(prefix / "include"), cli_source, *libraries, *system, "-o", binary])
    fixtures = Path(__file__).resolve().parent
    cases = json.loads((fixtures / "carrier-results.json").read_text())["cases"]
    corpus = root / "corpus"
    corpus.mkdir()
    results = []
    for case in cases:
        raw = (fixtures / case["carrier"]).read_bytes()
        if case["carrier_encoding"] == "gzip":
            raw = gzip.decompress(raw)
        require(hashlib.sha256(raw).hexdigest() == case["sha256"], "Fixture input hash differs")
        path = corpus / Path(case["file"]).name
        path.write_bytes(raw)
        output = subprocess.run([str(binary), "-json", str(path)], capture_output=True,
                                text=True, check=True)
        actual = json.loads(output.stdout)
        require(path.read_bytes() == raw, "The reference program changed its input")
        results.append({"name": case["name"], "input_sha256": case["sha256"],
                        "expected": actual, "arm_reference": case["expected"],
                        "same_as_arm": actual == case["expected"], "stderr": output.stderr})
        print(case["name"], actual == case["expected"], flush=True)
    require(len(results) == 14, "The complete original corpus is required")
    require({case["name"] for case in results if not case["same_as_arm"]} == {"vorbis"},
            "Unexpected platform differences require independent investigation")
    vorbis = next(case for case in results if case["name"] == "vorbis")
    require(vorbis["expected"]["duration"] == vorbis["arm_reference"]["duration"],
            "Platform duration differs")
    proof = {"format": 1, "target_arch": "x86_64", "reference_target": target,
             "reference_program": f"Unmodified upstream Chromaprint {source_info['version']} fpcalc source",
             "source_archives": {name: {"version": item["version"], "sha256": item["sha256"]}
                                 for name, item in manifest["components"].items()},
             "chromaprint_source_files_verified": source_files,
             "cli_source": cli_source.relative_to(source).as_posix(),
             "cli_source_sha256": digest(cli_source), "cli_binary_sha256": digest(binary),
             "fft": "vdsp" if apple else "kissfft",
             "codec_transform_archives": unchanged,
             "transport_exception": "File protocol in forensic libavformat only; product has zero protocols",
             "capture_script": Path(__file__).name, "reference_cases_checked": 14,
             "other_cases_identical_to_arm": 13,
             "input_sha256": vorbis["input_sha256"], "expected": vorbis["expected"]}
    (root / "vorbis-x86_64-reference.json").write_text(json.dumps(proof, indent=2) + "\n")
    (root / "all-reference-results.json").write_text(json.dumps(results, indent=2) + "\n")
    print("Independent source CLI capture complete", flush=True)


if __name__ == "__main__":
    main()
