/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::io::{BufRead, Read, Seek, SeekFrom, Write};

use crate::framework::itest;
use godot::builtin::GodotString;
use godot::engine::{file_access::ModeFlags, GFile};

const TEST_FULL_PATH: &str = "res://file_tests";

fn remove_test_file() {
    let test_file_path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../godot/",
        "file_tests"
    ));
    std::fs::remove_file(test_file_path).expect(&format!(
        "Couldn't remove test file: {}",
        test_file_path.display()
    ));
}

#[itest]
fn write_to_file_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE_READ).unwrap();
    let line_to_store = GodotString::from("TESTING1");
    file.write_string_line(line_to_store.clone()).unwrap();
    file.rewind().unwrap();
    let gotten_line = file.read_gstring_line().unwrap();
    assert_eq!(line_to_store, gotten_line);
    remove_test_file();
}

#[itest]
fn write_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE).unwrap();

    let integers: Vec<u8> = (0..=50).step_by(1).collect();
    file.write_all(&integers)
        .expect("Couldn't write integer vector");

    assert_eq!(file.length(), integers.len() as u64);
    remove_test_file();
}

#[itest]
fn read_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE).unwrap();

    let integers: Vec<u8> = (0..=50).step_by(1).collect();
    file.write_all(&integers)
        .expect("Couldn't write integer vector");
    drop(file);

    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::READ).unwrap();

    let mut read_integers = Vec::new();
    file.read_to_end(&mut read_integers)
        .expect("Couln't read numbers");

    assert_eq!(integers, read_integers);
    remove_test_file();
}

#[itest]
fn seek_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE_READ).unwrap();

    let integers: Vec<u8> = (0..=50).step_by(1).collect();
    file.write_all(&integers)
        .expect("Couldn't write integer vector");

    file.seek(SeekFrom::Start(10))
        .expect("Couldn't seek from start");
    let val = file.read_u8().unwrap();

    assert_eq!(integers[10], val);

    file.seek(SeekFrom::End(-1))
        .expect("Couldn't seek from end");
    let val = file.read_u8().unwrap();

    assert_eq!(integers[integers.len() - 1], val);

    file.seek(SeekFrom::Current(-10))
        .expect("Couldn't seek from current");
    file.seek(SeekFrom::Current(-10))
        .expect("Coulnd't seek from current");
    let val = file.read_u8().unwrap();

    assert_eq!(integers[integers.len() - 10 - 10], val);

    remove_test_file();
}

#[itest]
fn bufread_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE_READ).unwrap();

    let lines = String::from("First line\nSecond line\nThird line\nFourth\nLast line");

    file.write_string(&lines).expect("Couldn't write to file");
    file.rewind().unwrap();

    let mut read_lines = String::new();

    for i in 0..5 {
        file.read_line(&mut read_lines)
            .unwrap_or_else(|_| panic!("Couln't read line {}", i));
    }

    assert_eq!(lines, read_lines);
    remove_test_file();
}
