#![forbid(unsafe_code)]

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        assert_eq!(
            std::env::var("CARGO_CFG_TARGET_ENV").as_deref(),
            Ok("msvc"),
            "Windows builds require the MSVC toolchain for the embedded manifest"
        );
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("app.manifest");
        println!("cargo:rustc-link-arg-bin=TabGlide=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=TabGlide=/MANIFESTINPUT:{}",
            manifest.display()
        );
        println!("cargo:rustc-link-arg-bin=TabGlide=/MANIFESTUAC:NO");
    }
}
