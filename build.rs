fn main() {
    // rebuild if changed
    println!("cargo:rerun-if-changed=src/metadata/metadata_ggx_brooklyn.scale");
    println!("cargo:rerun-if-changed=src/metadata/metadata_ggx_sydney.scale");
}
