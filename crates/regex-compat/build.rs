fn main() {
    println!("cargo:rerun-if-changed=src/bridge.cpp");
    println!("cargo:rerun-if-changed=vendor/boost-regex/include");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .define("BOOST_REGEX_STANDALONE", None)
        .define("BOOST_HAS_THREADS", None)
        .include("vendor/boost-regex/include")
        .file("src/bridge.cpp")
        .compile("opentag_regex");
}
