use crate::Member;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Write};
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::process::{exit, Command};
use std::ptr::null_mut;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::HKEY;
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::winnt::KEY_READ;
use winapi::um::winreg::RegCloseKey;
use winapi::um::winreg::RegEnumKeyExW;
use winapi::um::winreg::RegOpenKeyExW;
use winapi::um::winreg::RegQueryInfoKeyW;
use winapi::um::winreg::RegQueryValueExW;
use winapi::um::winreg::HKEY_CURRENT_USER;

pub fn print_usage_and_exit() {
    println!("Usage: libman libraryfile(.lib)/objectfile(.obj) ... -o outputfile\n");
    exit(0);
}

pub fn get_lib_tool_list() -> Vec<String> {
    let mut list: Vec<String> = Vec::new();
    try_registry(&mut list);

    let current_path = env::current_dir().unwrap();
    let mut vsw = current_path.join("vswhere.exe");
    if !vsw.is_file() {
        let path;
        match env::var("PROGRAMFILES(x86)") {
            Ok(val) => path = val,
            Err(_) => panic!("cannot locate env variable for Program Files(x86)!"),
        }

        let root = Path::new(&path);
        vsw = root
            .join("Microsoft Visual Studio")
            .join("Installer")
            .join("vswhere.exe");
    }

    let output = Command::new(&vsw)
        .args(&[
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ])
        .output()
        .expect("failed to execute process");
    let s = String::from_utf8(output.stdout).expect("Invalid Output");
    let mut entries: Vec<String> = Vec::new();
    let mut lines = s.lines();
    loop {
        if let Some(line) = &lines.next() {
            entries.push(line.to_string());
        } else {
            break;
        }
    }

    for i in entries {
        list.push(get_tool(&i));
    }
    list
}

pub fn get_tool(install_path: &str) -> String {
    let t: &[_] = &['\r', '\n'];
    let path = Path::new(install_path);
    let ver_path = path
        .join("VC")
        .join("Auxiliary")
        .join("Build")
        .join("Microsoft.VCToolsVersion.default.txt");
    let mut file = File::open(ver_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let tool_path = path
        .join("VC")
        .join("Tools")
        .join("MSVC")
        .join(contents.trim_end_matches(t))
        .join("bin")
        .join("Hostx64")
        .join("x64")
        .join("lib.exe");
    //println!("{}", tool_path.display());
    tool_path.to_str().unwrap().to_string()
}

pub fn try_registry(list: &mut Vec<String>) {
    unsafe {
        let mut hkey: HKEY = null_mut();
        let mut inner_hkey: HKEY = null_mut();

        let root_entry = String::from("Software\\Microsoft\\VisualStudio");
        let mut wide: Vec<u16> = OsStr::new(&root_entry)
            .encode_wide()
            .chain(once(0))
            .collect();
        let mut lres =
            RegOpenKeyExW(HKEY_CURRENT_USER, wide.as_ptr(), 0, KEY_READ, &mut hkey) as u32;
        if lres != ERROR_SUCCESS {
            return;
        }

        // Get the subkeycount
        let mut sc: DWORD = 0;
        let mut sz_buffer: [u16; 512] = [0; 512];
        RegQueryInfoKeyW(
            hkey,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sc,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        );
        //println!("Number of subkeys: {}", sc);

        let mut sz_buffer_u8: [u8; 1024] = [0; 1024];
        let install_wide: Vec<u16> = OsStr::new("ShellFolder")
            .encode_wide()
            .chain(once(0))
            .collect();

        for i in 0..sc as usize {
            let mut sz: DWORD = 1024;
            RegEnumKeyExW(
                hkey,
                i as u32,
                sz_buffer.as_mut_ptr(),
                &mut sz,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            );
            let mut key = String::from_utf16(&sz_buffer).unwrap();
            key.truncate(sz as usize);
            //print!("{}",key);
            if key.contains("_Config") {
                let mut reg_key = root_entry.clone();
                reg_key.push('\\');
                reg_key.push_str(&key);
                wide = OsStr::new(&reg_key).encode_wide().chain(once(0)).collect();
                lres = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    wide.as_ptr(),
                    0,
                    KEY_READ,
                    &mut inner_hkey,
                ) as u32;
                if lres != ERROR_SUCCESS {
                    return;
                }

                let mut sz: DWORD = 1024;
                lres = RegQueryValueExW(
                    inner_hkey,
                    install_wide.as_ptr(),
                    null_mut(),
                    null_mut(),
                    sz_buffer_u8.as_mut_ptr(),
                    &mut sz,
                ) as u32;
                RegCloseKey(inner_hkey);

                if lres != ERROR_SUCCESS {
                    return;
                }

                let mut path = String::from(
                    get_string_from_u8(&sz_buffer_u8, sz as usize).trim_end_matches('\0'),
                );
                path.push_str("VC\\bin\\amd64\\lib.exe");
                list.push(path);
            }
        }
        RegCloseKey(hkey);
    }
}

pub fn get_string_from_u8(buffer: &[u8; 1024], len: usize) -> String {
    let mut sz_buffer: [u8; 512] = [0; 512];
    for i in 0..len {
        sz_buffer[i] = buffer[i * 2];
    }
    String::from_utf8(sz_buffer.to_vec()).unwrap()
}

pub fn scan_library(_index: usize, library: &String, members: &mut Vec<Member>, libtool: &str) {
    let output = Command::new(libtool)
        .args(&["/LIST", &library])
        .output()
        .expect("failed to execute process");
    let s = String::from_utf8(output.stdout).expect("Invalid Output");
    let mut lines = s.lines();
    let mut done_banner = false;
    let mut cline;
    let mut last_length = 0;
    loop {
        if let Some(line) = lines.next() {
            if done_banner {
                cline = format!("{}: extracting {}", library, line);
                let length = cline.len();
                if length < last_length {
                    for _ in 0..(last_length - length) {
                        cline.push(' ')
                    }
                } else {
                    last_length = length;
                }
                print!("{}\r", cline);
                io::stdout().flush().unwrap();
                extract_obj(&library, line, members, libtool);
            } else {
                if line.is_empty() {
                    done_banner = true;
                }
            }
        } else {
            cline = format!("{} - Done.", library);
            let length = cline.len();
            if length < last_length {
                for _ in 0..(last_length - length) {
                    cline.push(' ');
                }
            }
            println!("{}", cline);
            break;
        }
    }

    /*
    let mut mcount = 0;
    for m in members {
        println!("[{}]\t{}", mcount, m.name);
        mcount += 1;
    }
    */
}

pub fn extract_obj(lib: &str, obj: &str, members: &mut Vec<Member>, libtool: &str) {
    let mut extract = String::from("/EXTRACT:");
    extract.push_str(obj);
    let mut os = String::from("/OUT:eq-libman-");
    os.push_str(&members.len().to_string());
    os.push_str(".obj");
    Command::new(libtool)
        .args(&[extract, String::from(lib), os])
        .output()
        .expect("failed to execute process");

    let obj_name = String::from(obj);
    let nm: Vec<&str> = obj_name.rsplit('\\').collect();
    let final_nm: Vec<&str> = nm[0].rsplit('/').collect();
    let t: &[_] = &['.', 'o', 'b', 'j'];
    members.push(Member {
        name: String::from(final_nm[0].trim_end_matches(t)),
        library: 0,
        count: 0,
        is_new: false,
    });
}

pub fn create_new_library(
    _libraries: &Vec<String>,
    members: &Vec<Member>,
    new_library: &str,
    libtool: &str,
) -> usize {
    println!("Creating New Library '{}'", new_library);
    Command::new("cmd")
        .args(&["/C", "erase", new_library])
        .output()
        .expect("failed to execute process");

    let mut count = 0;
    let member_count = members.len();
    for member in members {
        print!("{}%\r", (count as f32 / member_count as f32 * 100.0) as u32);
        io::stdout().flush().unwrap();
        let mut output_path = String::from("/OUT:");
        output_path.push_str(new_library);

        let mut os: String;
        if member.is_new {
            os = member.name.clone();
        } else {
            os = String::from("eq-libman-");
            os.push_str(&count.to_string());
            os.push_str(".obj");
        }
        let mut is = member.name.clone();
        if member.count > 0 {
            is.push_str(&member.count.to_string());
        }
        is.push_str(".obj");

        Command::new("cmd")
            .args(&["/C", "copy", &os, &is])
            .output()
            .expect("failed to execute process");

        if count > 0 {
            Command::new(libtool)
                .args(&[output_path, String::from(new_library), is.clone()])
                .output()
                .expect("failed to execute process");
        } else {
            Command::new(libtool)
                .args(&[output_path, is.clone()])
                .output()
                .expect("failed to execute process");
        }

        if !member.is_new {
            Command::new("cmd")
                .args(&["/C", "erase", &os])
                .output()
                .expect("failed to execute process");
        }

        Command::new("cmd")
            .args(&["/C", "erase", &is])
            .output()
            .expect("failed to execute process");
        count += 1;
    }
    count + 1
}

pub fn get_lib_tool() -> String {
    let list: Vec<String> = get_lib_tool_list();
    let length = list.len();
    if length == 0 {
        panic!("Error cannot locate lib.exe. Please install command line tools for Visual Studio");
    }
    list[length - 1].clone()
}

pub fn make_backups(_libraries: &Vec<String>) {
    unimplemented!()
}

pub fn erase_backups(_libraries: &Vec<String>) {
    unimplemented!()
}
