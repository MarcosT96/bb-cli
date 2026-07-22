fn main() {
    // Cargo sets TARGET for build scripts to the target triple being built.
    // Re-export it as a compile-time env var so the binary knows which release
    // asset (`bb-<triple>`) corresponds to itself, for `bb upgrade`.
    let target = std::env::var("TARGET").expect("TARGET is always set by Cargo for build scripts");
    println!("cargo:rustc-env=TARGET={target}");
}
