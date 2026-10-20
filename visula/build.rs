fn main() {
    #[cfg(feature = "python")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap() != "wasm32" {
        pyo3_build_config::add_python_framework_link_args();
        pyo3_build_config::add_extension_module_link_args();
    }
}
