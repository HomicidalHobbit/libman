use std::process::{exit, Command};

use crate::Member;

pub fn print_usage_and_exit() {
    println!("Usage: libman libraryfile(.a)/objectfile(.o) ... -o outputfile\n");
    exit(0);
}

pub fn scan_library(index: usize, library: &String, members: &mut Vec<Member>, _libtool: &str) {
    print!("Scanning Library: {} Member Count: ", library);
    let output = Command::new("ar")
        .arg("t")
        .arg(&library)
        .output()
        .expect("Can't spawn ''ar''");
    let s = String::from_utf8(output.stdout).expect("Invalid Output");
    let mut lines = s.lines();
    let mut member_count = 0;
    loop {
        if let Some(line) = lines.next() {
            members.push(Member {
                name: String::from(line),
                library: index,
                count: 0,
                is_new: false,
            });
            member_count += 1;
        //println!("{}", line);
        } else {
            break;
        }
    }
    println!("{}", member_count);
}

pub fn get_lib_tool() -> String {
    unimplemented!()
}

pub fn create_new_library(
    libraries: &Vec<String>,
    members: &Vec<Member>,
    new_library: &str,
    _libtool: &str,
) -> usize {
    println!("Creating New Library '{}'", new_library);
    let mut count = 0;
    for member in members {
        count += 1;
        if member.is_new {
            let mut new_file = member.name.clone();
            if member.count != 0 {
                new_file.push_str(&member.count.to_string());
            }
            println!("Inserted: {}", &new_file);
            insert_file(&new_file, new_library);
        } else {
            /*
            println!(
                "Extracting: {} from {}",
                member.name, &libraries[member.library]
            );
            */

            // We're going to actually operate on the backup files not the originals
            let matches: &[_] = &['.', 'a'];
            let mut backup_library = libraries[member.library]
                .clone()
                .trim_end_matches(matches)
                .to_string();
            backup_library.push_str(".bak");

            Command::new("ar")
                .arg("x")
                .arg(&backup_library)
                .arg(&member.name)
                .output()
                .expect("Can't spawn 'ar");

            // If this member is duplicated then we also remove from library, so the next member is picked up
            if member.count > 0 {
                println!(
                    "Resolving Duplicate: {} from {}",
                    &member.name, &libraries[member.library]
                );
                Command::new("ar")
                    .arg("d")
                    .arg(&backup_library)
                    .arg(&member.name)
                    .output()
                    .expect("Can't spawn 'ar");

                // We also need to make sure that the file is unique, so let's rename it with the count appended
                let mut new_name = member.name.clone();
                new_name.push_str(&member.count.to_string());
                Command::new("mv")
                    .arg("-f")
                    .arg(&member.name)
                    .arg(&new_name)
                    .output()
                    .expect("Can't rename file");
                insert_file(&new_name, new_library);
                delete_file(&new_name);
            } else {
                insert_file(&member.name, new_library);
                delete_file(&member.name);
            }
        }
    }
    count
}

pub fn insert_file(name: &str, new_library: &str) {
    Command::new("ar")
        .arg("rs")
        .arg(&new_library)
        .arg(&name)
        .output()
        .expect("Can't spawn 'ar");
}

pub fn delete_file(name: &str) {
    Command::new("rm")
        .arg(&name)
        .output()
        .expect("Can't delete file");
}

pub fn make_backups(libraries: &Vec<String>) {
    for i in 0..libraries.len() {
        let library = libraries[i].replace(".a", ".bak");
        Command::new("cp")
            .arg("-f")
            .arg(&libraries[i])
            .arg(&library)
            .output()
            .expect("Can't backup");
    }
}

pub fn erase_backups(libraries: &Vec<String>) {
    for i in 0..libraries.len() {
        let library = libraries[i].replace(".a", ".bak");
        Command::new("rm")
            .arg("-f")
            .arg(&library)
            .output()
            .expect("Can't erase backup");
    }
}
