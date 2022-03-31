use std::env;
use std::path::PathBuf;
use std::collections::HashSet;

fn main() {

    let ignored_macros = IgnoreMacros(
        vec![
            "FP_INFINITE".into(),
            "FP_NAN".into(),
            "FP_NORMAL".into(),
            "FP_SUBNORMAL".into(),
            "FP_ZERO".into(),
            // "IPPORT_RESERVED".into(),
        ]
        .into_iter()
        .collect(),
    );

    // Tell cargo to tell rustc to link the system shared library.
    println!("cargo:rustc-link-lib=spdk_fat");
    println!("cargo:rustc-link-lib=aio");
    println!("cargo:rustc-link-lib=numa");
    println!("cargo:rustc-link-lib=uuid");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=stdc++");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .parse_callbacks(Box::new(ignored_macros))
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        // .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .blacklist_item("IPPORT_.*")
        // XXX: workaround for 'error[E0588]: packed type cannot transitively contain a `#[repr(align)]` type'
        .blacklist_type("spdk_nvme_tcp_rsp")
        .blacklist_type("spdk_nvme_tcp_cmd")
        .blacklist_type("spdk_nvmf_fabric_prop_get_rsp")
        .blacklist_type("spdk_nvmf_fabric_connect_rsp")
        .blacklist_type("spdk_nvmf_fabric_connect_cmd")
        .blacklist_type("spdk_nvmf_fabric_auth_send_cmd")
        .blacklist_type("spdk_nvmf_fabric_auth_recv_cmd")
        .blacklist_type("spdk_nvme_health_information_page")
        .blacklist_type("spdk_nvme_ctrlr_data")
        .blacklist_function("spdk_nvme_ctrlr_get_data")
        .opaque_type("spdk_nvme_sgl_descriptor")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}
