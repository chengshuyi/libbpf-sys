// build.rs

use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::os::unix::prelude::*;
use std::path;
use std::process;
use std::process::Command;

fn main() {
    let src_dir = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = path::PathBuf::from(env::var_os("OUT_DIR").unwrap());

    if !cfg!(target_os = "linux") {
        panic!(
            "cannot compile libbpf-sys on {}",
            env::var("CARGO_CFG_TARGET_OS").unwrap()
        );
    }
    
    let output = Command::new("sh")
    .arg("patch.sh")
    .output()
    .expect("Failed to execute command");

    if cfg!(feature = "novendor") {
        let libbpf = pkg_config::Config::new()
            .atleast_version(&format!(
                "{}.{}.{}",
                env!("CARGO_PKG_VERSION_MAJOR"),
                env!("CARGO_PKG_VERSION_MINOR"),
                env!("CARGO_PKG_VERSION_PATCH")
            ))
            .probe("libbpf")
            .unwrap();

        cc::Build::new()
            .file("bindings.c")
            .includes(&libbpf.include_paths)
            .define("__LIBBPF_SYS_NOVENDOR", None)
            .out_dir(&out_dir)
            .compile("bindings");
    } else {
        if let Err(_) = process::Command::new("make").status() {
            panic!("make is required to compile libbpf-sys using the vendored copy of libbpf");
        }

        if let Err(_) = process::Command::new("pkg-config").status() {
            panic!(
                "pkg-config is required to compile libbpf-sys using the vendored copy of libbpf"
            );
        }

        let compiler = match cc::Build::new().try_get_compiler() {
            Ok(compiler) => compiler,
            Err(_) => panic!(
                "a C compiler is required to compile libbpf-sys using the vendored copy of libbpf"
            ),
        };

        // create obj_dir if it doesn't exist
        let obj_dir = path::PathBuf::from(&out_dir.join("obj").into_os_string());
        let _ = fs::create_dir(&obj_dir);

        let status = process::Command::new("make")
            .arg("install")
            .env("BUILD_STATIC_ONLY", "y")
            .env("PREFIX", "/")
            .env("LIBDIR", "")
            .env("OBJDIR", &obj_dir)
            .env("DESTDIR", &out_dir)
            .env("CC", compiler.path())
            .env("CFLAGS", compiler.cflags_env())
            .current_dir(&src_dir.join("libbpf/src"))
            .status()
            .expect("could not execute make");

        assert!(status.success(), "make failed");

        let status = process::Command::new("make")
            .arg("clean")
            .current_dir(&src_dir.join("libbpf/src"))
            .status()
            .expect("could not execute make");

        assert!(status.success(), "make failed");

        cc::Build::new()
            .file("bindings.c")
            .include(&src_dir.join("libbpf/include"))
            .include(&src_dir.join("libbpf/include/uapi"))
            .out_dir(&out_dir)
            .compile("bindings");

        io::stdout()
            .write_all("cargo:rustc-link-search=native=".as_bytes())
            .unwrap();
        io::stdout()
            .write_all(out_dir.as_os_str().as_bytes())
            .unwrap();
        io::stdout().write_all("\n".as_bytes()).unwrap();
        io::stdout()
            .write_all("cargo:rustc-link-lib=elf\n".as_bytes())
            .unwrap();
        io::stdout()
            .write_all("cargo:rustc-link-lib=z\n".as_bytes())
            .unwrap();
        io::stdout()
            .write_all("cargo:rustc-link-lib=static=bpf\n".as_bytes())
            .unwrap();
        io::stdout().write_all("cargo:include=".as_bytes()).unwrap();
        io::stdout()
            .write_all(out_dir.as_os_str().as_bytes())
            .unwrap();
        io::stdout().write_all("/include\n".as_bytes()).unwrap();
    }
}
