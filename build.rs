use std::path::Path;

fn panic_if_file_not_found(path: &str) {
    if !Path::new(path).exists() {
        panic!("File not found: {}", path);
    }
}

fn main() {
    panic_if_file_not_found("src/metadata/metadata_ggx_brooklyn.scale");
    panic_if_file_not_found("src/metadata/metadata_ggx_sydney.scale");

    // rebuild if changed
    println!("cargo:rerun-if-changed=src/metadata/metadata_ggx_brooklyn.scale");
    println!("cargo:rerun-if-changed=src/metadata/metadata_ggx_sydney.scale");
}
