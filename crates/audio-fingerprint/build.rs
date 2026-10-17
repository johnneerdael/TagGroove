use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = crate_dir.parent().unwrap().parent().unwrap();
    let target = env::var("TARGET").unwrap();
    let root = env::var_os("OPENTAG_AUDIO_BUILD_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace.join("target"))
                .join("audio-native")
        });
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-changed=../../scripts/audio-deps");
    println!("cargo:rerun-if-env-changed=OPENTAG_AUDIO_BUILD_ROOT");
    println!("cargo:rerun-if-env-changed=OPENTAG_AUDIO_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=PYTHON");
    println!("cargo:rerun-if-env-changed=OPENTAG_MSYS2");
    println!("cargo:rerun-if-env-changed=NDK_HOME");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    let python = env::var_os("PYTHON").unwrap_or_else(|| {
        if cfg!(windows) {
            "python".into()
        } else {
            "python3".into()
        }
    });
    let mut command = Command::new(python);
    command
        .arg(workspace.join("scripts/audio-deps/build.py"))
        .arg("--target")
        .arg(&target)
        .arg("--output")
        .arg(&root);
    if target.ends_with("windows-msvc") {
        // cc discovers the selected Visual Studio environment without requiring
        // a globally modified shell, including the correct linker/include/lib.
        let tool = cc::Build::new().get_compiler();
        for (key, value) in tool.get_envs() {
            command.env(key, value);
        }
        command.env("CC", tool.path());
    }
    let output = command
        .output()
        .expect("Run the pinned native audio source builder (Python 3.10+)");
    if !output.status.success() {
        panic!(
            "Native audio source build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let prefix = PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("Native prefix path is UTF-8")
            .trim(),
    );
    assert!(
        prefix.join("opentag-audio-manifest.json").is_file(),
        "Native build manifest missing"
    );
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .define("USE_SWRESAMPLE", None)
        .define("CHROMAPRINT_NODLL", None)
        .include(prefix.join("include"))
        .include("native")
        .file("native/fingerprint.cpp");
    if target.ends_with("windows-msvc") {
        build.flag("/EHsc").flag("/utf-8").flag(format!(
            "/external:I{}",
            crate_dir.join("native/vendor").display()
        ));
    } else {
        build
            .flag("-isystem")
            .flag(crate_dir.join("native/vendor").as_os_str());
    }
    build.compile("opentag_audio");
    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    for name in [
        "chromaprint",
        "avformat",
        "avcodec",
        "swresample",
        "avutil",
        "z",
        "bz2",
    ] {
        println!("cargo:rustc-link-lib=static={name}");
    }
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    } else if target.ends_with("windows-msvc") {
        println!("cargo:rustc-link-lib=bcrypt");
    } else {
        println!("cargo:rustc-link-lib=m");
    }
    println!("cargo:metadata=prefix={}", prefix.display());
}
