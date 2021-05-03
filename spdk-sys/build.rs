use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to tell rustc to link the system shared library.
    println!("cargo:rustc-link-lib=spdk");
    println!("cargo:rustc-link-lib=spdk_env_dpdk");
    println!("cargo:rustc-link-lib=rte_eal");
    println!("cargo:rustc-link-lib=rte_mempool");
    println!("cargo:rustc-link-lib=rte_ring");
    println!("cargo:rustc-link-lib=rte_mbuf");
    println!("cargo:rustc-link-lib=rte_bus_pci");
    println!("cargo:rustc-link-lib=rte_pci");
    println!("cargo:rustc-link-lib=rte_mempool_ring");
    println!("cargo:rustc-link-lib=rte_power");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
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
