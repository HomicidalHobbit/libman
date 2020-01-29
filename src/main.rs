#[cfg(target_os = "windows")]
use windows::*;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(not(target_os = "windows"))]
use unix::*;
#[cfg(not(target_os = "windows"))]
pub mod unix;

use std::env;

pub struct Member {
    name: String,
    library: usize,
    count: usize,
    is_new: bool,
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    println!("\n[Library Manager]\n-----------------\nPart of Equinox Tools\nAuthor: Richard Underhill July 2019\n");
    if args.len() < 2 {
        print_usage_and_exit();
    }

    let mut libraries: Vec<String> = Vec::new();
    let mut objs: Vec<String> = Vec::new();
    let mut target_library = String::new();
    let lib_extension: &str;
    if cfg!(target_os = "windows") {
        lib_extension = ".lib";
    } else {
        lib_extension = ".a";
    }

    let obj_extension: &str;
    if cfg!(target_os = "windows") {
        obj_extension = ".obj";
    } else {
        obj_extension = ".o";
    }

    for i in 1..args.len() {
        if args[i] == "-o" {
            if i + 1 < args.len() {
                target_library = args[i + 1].clone();
                args[i + 1].clear();
            }
        } else {
            if !args[i].is_empty() {
                if args[i].ends_with(lib_extension) {
                    libraries.push(args[i].clone());
                } else {
                    if args[i].ends_with(obj_extension) {
                        objs.push(args[i].clone());
                    } else {
                        print_usage_and_exit();
                    }
                }
            }
        }
    }

    if target_library.is_empty() {
        print_usage_and_exit();
    }

    let lib_tool = get_lib_tool();
    let mut members: Vec<Member> = Vec::new();

    let mut index = 0;
    for library in &libraries {
        scan_library(index, library, &mut members, &lib_tool);
        index += 1;
    }

    for obj in objs {
        add_member(&mut members, &obj);
    }
    resolve_duplicates(&mut members);

    if cfg!(not(target_os = "windows")) {
        make_backups(&libraries);
    }
    if !target_library.is_empty() {
        let count = create_new_library(&libraries, &members, &target_library, &lib_tool);
        println!("Created Library: {} has {} Members", target_library, count);
    }

    if cfg!(not(target_os = "windows")) {
        erase_backups(&libraries);
    }

    println!("Done!");
}

fn add_member(members: &mut Vec<Member>, name: &str) {
    println!("Adding: {}", name);
    members.push(Member {
        name: name.to_string(),
        library: 0,
        count: 0,
        is_new: true,
    });
}

fn resolve_duplicates(members: &mut Vec<Member>) {
    for i in 0..members.len() {
        // Check to see if count has already been resolved
        let mut count = 1;
        if members[i].count == 0 {
            for j in i..members.len() {
                if i != j {
                    if members[i].name == members[j].name {
                        //println!("found dup: i: {} j: {} {}", i, j, members[i].name);
                        members[i].count = 1;
                        count += 1;
                        members[j].count = count;
                    }
                }
            }
        }
    }
}
