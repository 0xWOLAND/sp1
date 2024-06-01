use cfg_if::cfg_if;
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[allow(deprecated)]
use bindgen::CargoCallbacks;

/// Build the go library, generate Rust bindings for the exposed functions, and link the library.
fn main() {
    cfg_if! {
        if #[cfg(feature = "plonk")] {
            // Define the output directory
            let out_dir = env::var("OUT_DIR").unwrap();
            let dest_path = PathBuf::from(&out_dir);
            let lib_name = "sp1gnark";
            let dest = dest_path.join(format!("lib{}.a", lib_name));

            println!("Building Go library at {}", dest.display());

            // Run the go build command
            let status = Command::new("go")
                .current_dir("go")
                .env("CGO_ENABLED", "1")
                .args(["build", "-o", dest.to_str().unwrap(), "-buildmode=c-archive", "."])
                .status()
                .expect("Failed to build Go library");
            if !status.success() {
                panic!("Go build failed");
            }
            // Copy go/babybear.h to OUT_DIR/babybear.h
            let header_src = PathBuf::from("go/babybear.h");
            let header_dest = dest_path.join("babybear.h");
            std::fs::copy(header_src, header_dest).unwrap();

            // Generate bindings using bindgen
            let header_path = dest_path.join(format!("lib{}.h", lib_name));
            let bindings = bindgen::Builder
                ::default()
                .header(header_path.to_str().unwrap())
                .parse_callbacks(Box::new(CargoCallbacks::new()))
                .generate()
                .expect("Unable to generate bindings");

            bindings
                .write_to_file(dest_path.join("bindings.rs"))
                .expect("Couldn't write bindings!");

            println!("Go library built");

            // Link the Go library
            println!("cargo:rustc-link-search=native={}", dest_path.display());
            println!("cargo:rustc-link-lib=static={}", lib_name);
        } else {
            println!("cargo:rerun-if-changed=go");
            let lib_name = "sp1gnark";
            let image_name = "v1"; // TODO: Use env var for image name
            let temp_container = "temp_container";

            let out_dir = env::var("OUT_DIR").unwrap();
            let dest_path = PathBuf::from(&out_dir);
            let lib_path_a = dest_path.join(format!("lib{}.a", lib_name));
            let lib_path_h = dest_path.join(format!("lib{}.h", lib_name));

            Command::new("docker")
                .args(["create", "--name", temp_container, image_name])
                .status()
                .expect("Failed to create temp container");
            Command::new("docker")
                .args([
                    "cp",
                    &format!("{}:OUT_DIR/lib{}.h", temp_container, lib_name),
                    lib_path_h.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to copy library from container");
            Command::new("docker")
                .args([
                    "cp",
                    &format!("{}:OUT_DIR/lib{}.a", temp_container, lib_name),
                    lib_path_a.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to copy library from container");

            let header_src = PathBuf::from("go/babybear.h");
            let header_dest = dest_path.join("babybear.h");
            std::fs::copy(header_src, header_dest).unwrap();

            let header_path = dest_path.join(format!("lib{}.h", lib_name));
            let bindings = bindgen::Builder
                ::default()
                .header(header_path.to_str().unwrap())
                .parse_callbacks(Box::new(CargoCallbacks::new()))
                .generate()
                .expect("Unable to generate bindings");

            bindings
                .write_to_file(dest_path.join("bindings.rs"))
                .expect("Couldn't write bindings!");
            Command::new("docker")
                .args(["rm", temp_container])
                .status()
                .expect("Failed to remove temp container");


            println!("cargo:rustc-link-search=native={}", dest_path.display());
            println!("cargo:rustc-link-lib=static={}", lib_name);
        }
    }
}
