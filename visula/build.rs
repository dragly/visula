fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_arch != "wasm32" {
        pyo3_build_config::add_python_framework_link_args();
        pyo3_build_config::add_extension_module_link_args();
    }
}
