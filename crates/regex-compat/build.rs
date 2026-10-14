fn main() {
    println!("cargo:rerun-if-changed=src/bridge.cpp");
    println!("cargo:rerun-if-changed=vendor/boost-regex/include");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .define("BOOST_REGEX_STANDALONE", None)
        .define("BOOST_HAS_THREADS", None)
        // Use std::locale consistently. Boost otherwise selects Win32 LCID
        // traits on Windows, whose imbue() does not accept std::locale.
        .define("BOOST_REGEX_USE_CPP_LOCALE", None)
        .include("vendor/boost-regex/include")
        .file("src/bridge.cpp");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        build.flag("/EHsc").flag("/utf-8");
    }
    build.compile("opentag_regex");
}
