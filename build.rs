// exe에 제품명/버전/게시자 정보(VERSIONINFO)를 넣는다. rc.exe가 없으면 건너뛴다.
use std::{env, fs, path::PathBuf, process::Command};

fn find_rc() -> Option<PathBuf> {
    let pf = env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into());
    let bin = PathBuf::from(pf).join("Windows Kits").join("10").join("bin");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&bin).ok()?.flatten().map(|e| e.path()).collect();
    dirs.sort();
    dirs.reverse();
    dirs.into_iter().map(|d| d.join("x64").join("rc.exe")).find(|p| p.exists())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let nums: Vec<&str> = version.split('.').collect();
    let n = |i: usize| nums.get(i).copied().unwrap_or("0");
    let comma = format!("{},{},{},0", n(0), n(1), n(2));
    let rc_text = format!(
        r#"#include <winver.h>
VS_VERSION_INFO VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "trustone02\0"
      VALUE "FileDescription", "IMECaret Plus - IME state indicator\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "IMECaretPlus\0"
      VALUE "LegalCopyright", "Copyright (c) 2026 trustone02 & mattisonit. MIT License\0"
      VALUE "OriginalFilename", "IMECaretPlus.exe\0"
      VALUE "ProductName", "IMECaret Plus\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 0x04B0
  END
END
"#
    );
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc_path = out.join("version.rc");
    let res_path = out.join("version.res");
    fs::write(&rc_path, rc_text).unwrap();
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe not found; VERSIONINFO skipped");
        return;
    };
    let kit = rc.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap();
    let ver_dir = rc.parent().unwrap().parent().unwrap().file_name().unwrap().to_owned();
    let inc = kit.join("Include").join(ver_dir);
    let status = Command::new(&rc)
        .arg("/nologo")
        .arg(format!("/I{}", inc.join("um").display()))
        .arg(format!("/I{}", inc.join("shared").display()))
        .arg(format!("/fo{}", res_path.display()))
        .arg(&rc_path)
        .status()
        .unwrap();
    if status.success() {
        println!("cargo:rustc-link-arg-bins={}", res_path.display());
    } else {
        println!("cargo:warning=rc.exe failed; VERSIONINFO skipped");
    }
}
