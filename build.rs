
fn main() {
    let gmsh_lib_dir = std::env::var("GMSH_LIB_DIR").expect("GMSH_LIB_DIR is not set");
    println!("cargo:rustc-link-search=native={}", gmsh_lib_dir);
    println!("cargo:rustc-link-lib=gmsh");
}
