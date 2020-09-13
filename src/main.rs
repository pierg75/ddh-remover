#[macro_use]
extern crate log;
extern crate fs_extra;

use clap::{App, Arg, ArgMatches};
use fs_extra::file::{move_file, CopyOptions};
use serde::{Deserialize, Serialize};
use std::{fs, fs::File, io, io::prelude::*, path::Path, process, thread};

type Error = Box<dyn std::error::Error>;

#[derive(Debug, Clone)]
struct Args {
    skip: usize,
    move_dest: Option<String>,
    dry_run: bool,
    keep_path: Option<String>,
}

impl Args {
    fn new(args: ArgMatches) -> Result<Args, Error> {
        let skip: usize = args.value_of("duplicates").unwrap_or("1").parse().unwrap();
        let keep_path = match args.value_of("keep") {
            Some(keep) => Some(keep.to_owned()),
            None => None,
        };
        let move_dest = match args.value_of("dest_path") {
            Some(dest) => Some(dest.to_owned()),
            None => None,
        };
        let dry_run = args.is_present("no");
        Ok(Args {
            skip,
            move_dest,
            dry_run,
            keep_path,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Duplicates {
    file_length: u64,
    file_paths: Vec<String>,
    full_hash: Option<u128>,
    partial_hash: Option<u128>,
}

#[derive(Debug)]
struct WorkItem {
    duplicate: Duplicates,
    args: Args,
    files_to_remove: Vec<String>,
}

impl WorkItem {
    fn new(duplicate: Duplicates, args: Args) -> Self {
        // Do work here to match which files to delete/move (this will end up in the
        // "affected_files" vec
        let mut tmp_files: Vec<String> = Vec::new();
        // We are going to either skip X amount of files or have a "preferred" file to keep.
        // However there could be the possibility that there are more files in the
        // preferred path. In that case apply both (skip and select preferred).
        // Note that skip it will either always be 1 or greater (1 being the default).
        let args_tmp = args.clone();
        match args.keep_path {
            Some(path) => {
                trace!("Keep a preferred file");
                for file in &duplicate.file_paths {
                    if !file.contains(&path) {
                        tmp_files.push(file.to_owned());
                    }
                }
                if tmp_files.len() > args.skip {
                    tmp_files.resize(args.skip, "".to_owned());
                }
                trace!("tmp_files after keeping: {:?}", tmp_files);
            }
            None => {
                trace!("Keep only the first {} amount of files", args.skip);
                duplicate
                    .file_paths
                    .iter()
                    .skip(args.skip)
                    .for_each(|x| tmp_files.push(x.clone()));
                trace!("tmp_files after skipping: {:?}", tmp_files);
            }
        };
        WorkItem {
            duplicate,
            args: args_tmp,
            files_to_remove: tmp_files,
        }
    }

    fn moveto(&self) -> Result<(), Error> {
        debug!("Moving files {:?}", self.files_to_remove);
        for file in &self.files_to_remove {
            print!(
                "Moving file {} to {}...",
                file,
                self.args.move_dest.clone().unwrap_or("".to_owned())
            );
            let file_name = Path::new(file).file_name().unwrap();
            let mut dest = String::from(self.args.move_dest.clone().unwrap());
            dest.push('/');
            dest.push_str(file_name.to_str().unwrap());
            debug!("dest: {}", dest);
            let options = CopyOptions::new();
            match move_file(file, dest, &options) {
                Ok(_) => println!("Done"),
                Err(e) => println!("Error ({})", e),
            }
        }
        Ok(())
    }

    fn delete(&self) -> Result<(), Error> {
        debug!("Deleting files {:?}", self.files_to_remove);
        for file in &self.files_to_remove {
            print!("Removing file {}...", file);
            match self.args.dry_run {
                false => match fs::remove_file(file) {
                    Ok(_) => println!("Done"),
                    Err(e) => println!("Error ({})", e),
                },
                true => println!("Done (not really)"),
            }
        }
        Ok(())
    }

    fn run(&self) -> Result<(), Error> {
        debug!("Doing the proper work on files {:?}", self.files_to_remove);
        match &self.args.move_dest {
            Some(_) => self.moveto(),
            None => self.delete(),
        }
    }
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
        .arg(
            Arg::with_name("keep")
                .short("k")
                .long("keep")
                .takes_value(true)
                .help("Keep the files matching the \"keep\" string"),
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

    // Some sanity checks
    if matches.is_present("dest_path") {
        match Path::new(matches.value_of("dest_path").unwrap_or("")).exists() {
            true => {}
            false => {
                println!("The destination path does not exists");
                process::exit(3);
            }
        }
    }

    let de: Vec<Duplicates> = match serde_json::from_str(&buffer) {
        Ok(de) => de,
        Err(e) => {
            println!("Error decoding the json file ({})", e);
            process::exit(2);
        }
    };

    // Get the various cmdline options
    let args = Args::new(matches)?;
    // Go through all the json elements
    trace!("de: {:?}", de);
    for v in de.into_iter() {
        if v.file_paths.len() > 1 && (v.full_hash.is_some() || v.partial_hash.is_some()) {
            trace!("Element {:#?}", v);
            for entry in &v.file_paths {
                debug!("{}", entry);
            }
            let args = args.clone();
            let handler = thread::spawn(move || {
                let instance = WorkItem::new(v, args);
                trace!("instance: {:#?}", instance);
                debug!("original files: {:#?}", instance.duplicate.file_paths);
                debug!("files to remove: {:#?}", instance.files_to_remove);
                instance.run().unwrap();
            });
            handler.join()
        } else {
            trace!("This file has no duplicates");
            trace!("{:#?}", v);
            Ok(())
        }
        .unwrap();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_dups() {
        let test_json = r#"
        {
            "file_length" : 1318934,
            "file_paths" : [
                "/data/Photos/ny/00097.jpg",
                "/data/Photos/concerts/00097.jpg"
            ],
            "full_hash" : 306482972711412640985380379178329462852,
            "partial_hash" : 119482817874600850350240560092010233366
        }"#;
        let expected = vec!["/data/Photos/concerts/00097.jpg"];
        let deserialized: Duplicates = serde_json::from_str(&test_json).unwrap();
        let args = Args {
            skip: 1,
            move_dest: None,
            dry_run: false,
            keep_path: None,
        };
        let wi = WorkItem::new(deserialized, args);
        assert_eq!(wi.files_to_remove, expected);
    }

    #[test]
    fn test_json_dups_keep() {
        let test_json = r#"
        {
            "file_length" : 1318934,
            "file_paths" : [
                "/data/Photos/ny/00097.jpg",
                "/data/Photos/concerts/00097.jpg"
            ],
            "full_hash" : 306482972711412640985380379178329462852,
            "partial_hash" : 119482817874600850350240560092010233366
        }"#;
        let expected = vec!["/data/Photos/ny/00097.jpg"];
        let deserialized: Duplicates = serde_json::from_str(&test_json).unwrap();
        let args = Args {
            skip: 1,
            move_dest: None,
            dry_run: false,
            keep_path: Some("concerts".to_owned()),
        };
        let wi = WorkItem::new(deserialized, args);
        assert_eq!(wi.files_to_remove, expected);
    }

    #[test]
    fn test_json_more_dups() {
        let test_json = r#"
        {
            "file_length" : 1318934,
            "file_paths" : [
                "/data/Photos/ny/00097.jpg",
                "/data/Photos/concerts/00097.jpg"
            ],
            "full_hash" : 306482972711412640985380379178329462852,
            "partial_hash" : 119482817874600850350240560092010233366
        }"#;
        let expected: Vec<String> = Vec::new();
        let deserialized: Duplicates = serde_json::from_str(&test_json).unwrap();
        let args = Args {
            skip: 2,
            move_dest: None,
            dry_run: false,
            keep_path: None,
        };
        let wi = WorkItem::new(deserialized, args);
        assert_eq!(wi.files_to_remove, expected);
    }
}
