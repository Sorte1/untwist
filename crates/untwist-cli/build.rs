fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if target == "x86_64-unknown-linux-gnu" && profile == "release" {
        println!("cargo:rustc-link-arg=-Wl,-z,pack-relative-relocs");
    }
}
