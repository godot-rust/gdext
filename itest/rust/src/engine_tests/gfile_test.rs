/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};

use godot::builtin::GString;
use godot::classes::file_access::ModeFlags;
use godot::tools::GFile;

use crate::framework::itest;

const TEST_FULL_PATH: &str = "res://file_tests";

fn remove_test_file() {
    let test_file_path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../godot/",
        "file_tests"
    ));
    std::fs::remove_file(test_file_path)
        .unwrap_or_else(|_| panic!("couldn't remove test file: {}", test_file_path.display()));
}

#[itest]
fn basic_read_write_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE).unwrap();
    let line_to_store = GString::from("TESTING1");
    file.write_gstring_line(&line_to_store).unwrap();
    drop(file);

    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::READ).unwrap();
    let gotten_line = file.read_gstring_line().unwrap();
    assert_eq!(line_to_store, gotten_line);
    drop(file);

    remove_test_file();
}

#[itest]
fn write_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE).unwrap();

    let integers: Vec<u8> = (0..=50).collect();
    file.write_all(&integers)
        .expect("couldn't write integer vector");

    assert_eq!(file.length(), integers.len() as u64);
    drop(file);
    remove_test_file();
}

#[itest]
fn read_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE).unwrap();

    let integers: Vec<u8> = (0..=50).collect();
    file.write_all(&integers)
        .expect("couldn't write integer vector");
    drop(file);

    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::READ).unwrap();

    let mut read_integers = Vec::new();
    file.read_to_end(&mut read_integers)
        .expect("couldn't read numbers");

    assert_eq!(integers, read_integers);
    drop(file);
    remove_test_file();
}

#[itest]
fn bufwriter_works() {
    let file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE).unwrap();
    let mut bufwriter = BufWriter::new(file);

    let integers: Vec<u8> = (0..=255).collect();
    bufwriter
        .write_all(&integers)
        .expect("couldn't write integer vector");

    drop(bufwriter);

    remove_test_file();
}

#[itest]
fn bufreader_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE).unwrap();

    let integers: Vec<u8> = (0..=50).collect();
    file.write_all(&integers)
        .expect("couldn't write integer vector");
    drop(file);

    let file = GFile::open(TEST_FULL_PATH, ModeFlags::READ).unwrap();
    let mut bufreader = BufReader::new(file);

    let mut read_integers = Vec::new();
    bufreader
        .read_to_end(&mut read_integers)
        .expect("couldn't read numbers");

    assert_eq!(integers, read_integers);
    drop(bufreader);
    remove_test_file();
}

#[itest]
fn seek_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE_READ).unwrap();

    let integers: Vec<u8> = (0..=50).step_by(1).collect();
    file.write_all(&integers)
        .expect("couldn't write integer vector");

    file.seek(SeekFrom::Start(10))
        .expect("couldn't seek from start");
    let val = file.read_u8().unwrap();

    assert_eq!(integers[10], val);

    file.seek(SeekFrom::End(-1))
        .expect("couldn't seek from end");
    let val = file.read_u8().unwrap();

    assert_eq!(integers[integers.len() - 1], val);

    file.seek(SeekFrom::Current(-10))
        .expect("couldn't seek from current");
    file.seek(SeekFrom::Current(-10))
        .expect("couldn't seek from current");
    let val = file.read_u8().unwrap();

    assert_eq!(integers[integers.len() - 10 - 10], val);
    drop(file);
    remove_test_file();
}

#[itest]
fn bufread_trait_works() {
    let mut file = GFile::open(TEST_FULL_PATH, ModeFlags::WRITE_READ).unwrap();

    let lines = String::from("First line\nSecond line\nThird line\nFourth\nLast line");

    file.write_gstring(&lines).expect("couldn't write to file");
    file.rewind().unwrap();

    let mut read_lines = String::new();

    for i in 0..5 {
        file.read_line(&mut read_lines)
            .unwrap_or_else(|_| panic!("couldn't read line {i}"));
    }

    assert_eq!(lines, read_lines);
    drop(file);
    remove_test_file();
}
