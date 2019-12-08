use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

fn get_toolchain_paths() -> io::Result<Vec<PathBuf>> {
    let toolchain_dir = home::rustup_home()?.join("toolchains");
    let mut paths = Vec::new();
    for entry in fs::read_dir(toolchain_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn get_rust_so_libs(libdir: &Path) -> io::Result<Vec<PathBuf>> {
    use std::env::consts::DLL_EXTENSION;
    let mut libs = Vec::new();
    for entry in fs::read_dir(libdir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() && path.extension().map_or(false, |e| e == DLL_EXTENSION) {
            libs.push(path);
        }
    }
    Ok(libs)
}

fn parse_rust_host(stdout: &str) -> Option<&str> {
    const PAT: &str = "host: ";
    for line in stdout.lines() {
        if line.starts_with(PAT) {
            let host = unsafe { line.get_unchecked(PAT.len()..) };
            return Some(host);
        }
    }
    None
}

fn link_duplicated_so_files() -> io::Result<()> {
    for tc_dir in get_toolchain_paths()? {
        let rustc = tc_dir.join("bin").join("rustc");
        let output = Command::new(rustc).args(&["-Vv"]).output()?;
        let stdout = unsafe { String::from_utf8_unchecked(output.stdout) };
        if let Some(host) = parse_rust_host(&stdout) {
            let libdir = tc_dir.join("lib");
            for lib in get_rust_so_libs(&libdir)? {
                if let Some(libname) = lib.file_name() {
                    let src = Path::new("rustlib").join(host).join("lib").join(libname);
                    let abs_src = lib.join(&src);
                    if abs_src.exists() {
                        fs::remove_file(&lib)?;
                        symlink(&src, &lib)?;
                    } else {
                        eprintln!("info: skip non existence file: {}", abs_src.display());
                    }
                } else {
                    eprintln!("warn: cannot get filename of {}", lib.display());
                }
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(err) = link_duplicated_so_files() {
        panic!("{:?}", err);
    }
}
