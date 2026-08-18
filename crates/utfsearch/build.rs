fn main() {
    // The embedding of WizTree.exe is now handled by utfsearch-core's build.rs
    // This crate just ensures the core dependency is built first
    println!("cargo:rerun-if-changed=../utfsearch-core/build.rs");
}



