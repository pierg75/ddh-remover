#[macro_use]
extern crate log;
extern crate fs_extra;

use clap::ArgMatches;
use clap::{App, Arg};
use fs_extra::file::{move_file, CopyOptions};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process;
use std::thread;

type Error = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize)]
struct Duplicates {
    file_length: u64,
    file_paths: Vec<String>,
    full_hash: Option<u128>,
    partial_hash: Option<u128>,
}

fn rem_file(
    paths: Vec<String>,
    skip: usize,
    no: bool,
    matches: ArgMatches<'static>,
) -> thread::Result<()> {
    let handler = thread::spawn(move || {
        paths.iter().skip(skip).for_each(|x| {
            trace!("Thread file: {}", x);
            match matches.value_of("dest_path").or(None) {
                None => {
                    print!("Deleting duplicate {}...", x);
                    if !no {
                        match fs::remove_file(x) {
                            Ok(_) => println!("Done"),
                            Err(e) => println!("Error ({})", e),
                        }
                    } else {
                        println!("Done (not really)");
                    }
                }
                Some(dest_path) => {
                    print!("Moving duplicate {} to {}...", x, dest_path);
                    if !no {
                        let file_name = Path::new(x).file_name().unwrap();
                        let mut dest = String::from(dest_path);
                        dest.push('/');
                        dest.push_str(file_name.to_str().unwrap());
                        debug!("dest: {}", dest);
                        let options = CopyOptions::new();
                        match move_file(x, dest, &options) {
                            Ok(_) => println!("Done"),
                            Err(e) => println!("Error ({})", e),
                        }
                    } else {
                        println!("Done (not really)");
                    }
                }
            };
        });
    });
    handler.join()
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let matches = App::new("ddh-remover")
        .version("0.1")
        .author("Pierguido L.")
        .long_about("It removes files found by the ddh utility.\nddh has to be used with the json output to be parsed by ddh-remover.\nThis can be saved in a file or read from stdin with a pipe a pipe")
        .arg(
            Arg::with_name("no")
                .short("n")
                .help("It doesn't do anything, no file removal"),
        )
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .takes_value(true)
                .help("Read the json input from a file"),
        )
        .arg(
            Arg::with_name("duplicates")
                .short("d")
                .long("duplicates")
                .takes_value(true)
                .default_value("1")
                .help("How many duplicates to keep. Defaults to 1 (only one file, no duplicates)"),
        )
        .arg(
            Arg::with_name("dest_path")
                .short("m")
                .long("move")
                .takes_value(true)
                .help("Move the files to [dest_path] instead of deleting them"),
        )
        .get_matches();

    let mut buffer = String::new();
    if matches.is_present("file") {
        // Read from file
        let mut jsonf = File::open(matches.value_of("file").unwrap())?;
        jsonf.read_to_string(&mut buffer)?;
    } else {
        // Read from stdin
        let mut stdin = io::stdin();
        stdin.read_to_string(&mut buffer)?;
        trace!("stdin {}", buffer);
    }

    let mut de: Vec<Duplicates> = match serde_json::from_str(&buffer) {
        Ok(de) => de,
        Err(e) => {
            println!("Error decoding the json file ({})", e);
            process::exit(2);
        }
    };
    let iter_de = de.iter_mut();
    trace!("iter_de: {:?}", iter_de);
    for (i, v) in iter_de.enumerate() {
        if v.file_paths.len() > 1 && (v.full_hash.is_some() || v.partial_hash.is_some()) {
            debug!("Index {}", i);
            trace!("Element {:#?}", v);
            for entry in &v.file_paths {
                debug!("{}", entry);
            }
            let skip: usize = matches
                .value_of("duplicates")
                .unwrap_or("1")
                .parse()
                .unwrap();
            let cloned_paths = v.file_paths.clone();
            let no = matches.is_present("no");
            rem_file(cloned_paths, skip, no, matches.clone()).unwrap();
        } else {
            trace!("This file has no duplicates");
            trace!("{:#?}", v);
        }
    }
    Ok(())
}