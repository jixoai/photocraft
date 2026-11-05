#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") || std::env::var_os("CARGO_FEATURE_THREADS").is_none() {
        return;
    }
    for argument in ["--shared-memory", "--import-memory", "--export=__wasm_init_tls", "--export=__tls_size", "--export=__tls_align", "--export=__tls_base"] {
        println!("cargo:rustc-link-arg-bin=photocraft-web={argument}");
    }
}
