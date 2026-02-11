use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use std::env;

#[allow(non_camel_case_types)]
enum Arch {
    Linux_X86_32,
    Linux_X86_64,
    Linux_Aarch_64,
    Linux_Ppcle_64,
    Macos_Aarch_64,
    Macos_x86_64,
    Win32,
}

/// Error returned when a binary is not available.
#[derive(Debug)]
pub struct Error {
    os: &'static str,
    arch: &'static str,
}

impl Arch {
    fn detect() -> Result<Arch, Error> {
        Ok(match (env::consts::OS, env::consts::ARCH) {
            ("linux", "x86") => Arch::Linux_X86_32,
            ("linux", "x86_64") => Arch::Linux_X86_64,
            ("linux", "aarch64") => Arch::Linux_Aarch_64,
            ("linux", "powerpc64") => Arch::Linux_Ppcle_64,
            ("macos", "x86_64") => Arch::Macos_x86_64,
            ("macos", "aarch64") => Arch::Macos_Aarch_64,
            ("windows", _) => Arch::Win32,
            (os, arch) => return Err(Error { os, arch }),
        })
    }
}

/// Return a path to `protoc` binary.
///
/// This function returns an error when binary is not available for
/// the current operating system and architecture.
pub fn protoc_bin_path() -> Result<PathBuf, Error> {
    Ok(match Arch::detect()? {
        Arch::Linux_X86_32 => PathBuf::from("vendored_protoc/linux-x86_32/protoc"),
        Arch::Linux_X86_64 => PathBuf::from("vendored_protoc/linux-x86_64/protoc"),
        Arch::Linux_Aarch_64 => PathBuf::from("vendored_protoc/linux-aarch_64/protoc"),
        Arch::Linux_Ppcle_64 => PathBuf::from("vendored_protoc/linux-ppcle_64/protoc"),
        Arch::Macos_x86_64 => PathBuf::from("vendored_protoc/macos-x86_64/protoc"),
        Arch::Macos_Aarch_64 => PathBuf::from("vendored_protoc/macos-aarch_64/protoc"),
        Arch::Win32 => PathBuf::from("vendored_protoc/win32/protoc.exe"),
    })
}

fn main() {
    // The proto files defining the message types we want to support.
    let roots = ["protos/perfetto/trace/trace.proto"];
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set by cargo"));
    let depfile_path = out_dir.join("perfetto_protos.deps");
    let protoc = match protoc_bin_path() {
        Ok(path) => match path.to_str() {
            Some(s) => s.to_owned(),
            None => {
                eprintln!("Error: protoc path '{}' is not valid UTF-8", path.display());
                std::process::exit(1);
            }
        },
        Err(err) => {
            eprintln!(
                "Error: failed to locate vendored protoc for OS '{}' and ARCH '{}'",
                err.os, err.arch
            );
            std::process::exit(1);
        }
    };

    // Find the transitive deps of `roots`.
    let dependency_out_arg = format!("--dependency_out={}", depfile_path.display());
    let child = Command::new(protoc.clone())
        .arg(dependency_out_arg)
        .arg("--descriptor_set_out=/dev/null")
        .args(roots)
        .spawn()
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    let output = std::fs::read_to_string(&depfile_path).unwrap();
    let output = output.replace("\\\n", " ");
    let output = output.replace("/dev/null: ", "");
    let files: Vec<&str> = output.split_ascii_whitespace().collect();

    // Generate Rust code from protos.
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(Path::new(&protoc))
        .include(".")
        .inputs(&files)
        .cargo_out_dir("protos")
        .run_from_script();
}
