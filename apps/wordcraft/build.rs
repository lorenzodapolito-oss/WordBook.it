//! Windows only: embed the app icon and version info (VERSIONINFO) into `WordBook.it.exe`, so it
//! shows in Explorer, the taskbar, the Start menu and Alt-Tab.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `WORDCRAFT_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).
//!
//! With the GNU toolchain the resource compiler is `windres`. It is taken from `PATH`; when it is
//! not there, the usual MSYS2 and MinGW folders are tried (`WINDRES` and `AR` override both). The
//! script reruns when `PATH` changes, so putting MSYS2 on it and building again is enough.

use std::path::{Path, PathBuf};

/// Folders MSYS2 and MinGW-w64 put `windres.exe` and `ar.exe` in, when `PATH` doesn't list them.
fn gnu_tool_dirs() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(r) = std::env::var_os("MSYS2_ROOT") {
        roots.push(PathBuf::from(r));
    }
    roots.extend(["C:\\msys64", "C:\\msys32"].map(PathBuf::from));
    let mut dirs: Vec<PathBuf> = roots.iter().flat_map(|r| ["ucrt64", "mingw64", "clang64"].map(|s| r.join(s).join("bin"))).collect();
    dirs.extend(
        ["C:\\mingw64\\bin", "C:\\ProgramData\\mingw64\\mingw64\\bin", "C:\\ProgramData\\chocolatey\\lib\\mingw\\tools\\install\\mingw64\\bin"]
            .map(PathBuf::from),
    );
    dirs
}

/// Whether `name.exe` is in one of the folders of `PATH`.
fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(format!("{name}.exe")).is_file()))
}

/// Points the resource compiler at MSYS2 or MinGW when `windres` isn't on `PATH`.
fn find_gnu_tools(res: &mut winresource::WindowsResource) {
    if std::env::var_os("WINDRES").is_some() || on_path("windres") {
        return;
    }
    let found = gnu_tool_dirs().into_iter().find(|d| d.join("windres.exe").is_file());
    if let Some(dir) = found {
        let tool = |name: &str| -> String { Path::new(&dir).join(format!("{name}.exe")).to_string_lossy().into_owned() };
        println!("cargo:warning=windres is not on PATH, using {}", tool("windres"));
        res.set_windres_path(&tool("windres"));
        if std::env::var_os("AR").is_none() && !on_path("ar") {
            res.set_ar_path(&tool("ar"));
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/wordbook.ico");
    println!("cargo:rerun-if-env-changed=WORDCRAFT_REQUIRE_WINRES");
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=WINDRES");
    println!("cargo:rerun-if-env-changed=AR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        find_gnu_tools(&mut res);
    }
    res.set_icon("../../assets/app-icon/wordbook.ico")
        .set("ProductName", "WordBook.it")
        .set("FileDescription", "WordBook.it word processor")
        .set("CompanyName", "Learning Machines LLC")
        .set("LegalCopyright", "Copyright (c) the WordCraft authors. MIT OR Apache-2.0.")
        .set("OriginalFilename", "WordBook.it.exe")
        .set("InternalName", "wordcraft");
    if let Err(e) = res.compile() {
        if std::env::var_os("WORDCRAFT_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!(
            "cargo:warning=WordBook.it.exe built without icon/version resources: {e}. Put the MSYS2 folder (C:\\msys64\\ucrt64\\bin) on PATH and build again."
        );
    }
}
