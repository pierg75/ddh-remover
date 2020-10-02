use clap::{App, Arg};
use ddh_remover::{Args, Duplicates, WorkItem};
use log::{debug, trace};
use std::{fs::File, io, io::prelude::*, path::Path, process, thread};

type Error = Box<dyn std::error::Error>;

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
                eprintln!("The destination path does not exists");
                process::exit(3);
            }
        }
    }

    let de: Vec<Duplicates> = match serde_json::from_str(&buffer) {
        Ok(de) => de,
        Err(e) => {
            eprintln!("Error decoding the json file ({})", e);
            process::exit(2);
        }
    };

    // Get the various cmdline options
    let args = Args::new(matches);
    // Go through all the json elements
    trace!("de: {:?}", de);
    for v in de.into_iter() {
        if v.files().len() > 1 && (v.full_hashes().is_some() || v.partial_hashes().is_some()) {
            trace!("Element {:#?}", v);
            for entry in &v.files() {
                debug!("{}", entry);
            }
            let args = args.clone();
            let handler: thread::JoinHandle<_> = thread::spawn(move || {
                let instance = WorkItem::new(&v, args);
                trace!("instance: {:#?}", instance);
                debug!("original files: {:#?}", instance.dups().files());
                debug!("files to remove: {:#?}", instance.files_remove());
                instance.run()
            });
            handler.join().unwrap()?;
        } else {
            trace!("This file has no duplicates");
            trace!("{:#?}", v);
        }
    }
    Ok(())
}
