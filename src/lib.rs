#[macro_use]
extern crate log;
extern crate fs_extra;

use clap::ArgMatches;
use fs_extra::file::{move_file, CopyOptions};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

type Error = Box<dyn std::error::Error>;

#[derive(Debug, Clone)]
pub struct Args {
    skip: usize,
    move_dest: Option<String>,
    dry_run: bool,
    keep_path: Option<String>,
}

impl Args {
    pub fn new(args: ArgMatches) -> Result<Args, Error> {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Duplicates {
    file_length: u64,
    file_paths: Vec<String>,
    full_hash: Option<u128>,
    partial_hash: Option<u128>,
}

impl Duplicates {
    pub fn files(&self) -> Vec<String> {
        self.file_paths.clone()
    }
    pub fn full_hashes(&self) -> Option<u128> {
        self.full_hash
    }
    pub fn partial_hashes(&self) -> Option<u128> {
        self.partial_hash
    }
}

#[derive(Debug)]
pub struct WorkItem {
    duplicate: Duplicates,
    args: Args,
    files_to_remove: Vec<String>,
}

impl WorkItem {
    pub fn new(duplicate: Duplicates, args: Args) -> Self {
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

    pub fn moveto(&self) -> Result<(), Error> {
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

    pub fn delete(&self) -> Result<(), Error> {
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

    pub fn run(&self) -> Result<(), Error> {
        debug!("Doing the proper work on files {:?}", self.files_to_remove);
        match &self.args.move_dest {
            Some(_) => self.moveto(),
            None => self.delete(),
        }
    }

    pub fn files_remove(&self) -> &Vec<String> {
        &self.files_to_remove
    }
    pub fn dups(&self) -> &Duplicates {
        &self.duplicate
    }
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
        assert_eq!(*wi.files_remove(), expected);
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
        assert_eq!(*wi.files_remove(), expected);
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
        assert_eq!(*wi.files_remove(), expected);
    }

    #[test]
    fn test_wrong_json() {
        let test_json = r#"
        {
            "field1" : "test",
            "field2" : "/data/Photos/ny/00097.jpg",
            "field3" : 3,
        }"#;
        assert!(serde_json::from_str::<String>(&test_json).is_err());
    }
}
