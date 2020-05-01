use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::App;
use clap::Arg;
use memmap::Mmap;
use std::fs::copy;
use std::fs::remove_file;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<()> {
    const DIRECTORY_NAME: &'static str = "DIRECTORY";

    let matches = App::new("vm-test")
        .about(
            "This program helps test Linux mmap behaviour. The program starts by creating\n\
            number_of_files files each with pages_per_file * page_size bytes in the directory\n\
            DIRECTORY. Each file created is mmap into the process address. After mmapping all\n\
            of the files and sleeping for sleep seconds, it deletes all of the files created.",
        )
        .arg(
            Arg::with_name("number_of_files")
                .long("number-of-files")
                .takes_value(true)
                .help("The number of files to create and mmap."),
        )
        .arg(
            Arg::with_name("pages_per_file")
                .long("pages-per-file")
                .takes_value(true)
                .help("The number of pages per file."),
        )
        .arg(
            Arg::with_name("page_size")
                .long("page-size")
                .takes_value(true)
                .help("The number of bytes in a page"),
        )
        .arg(
            Arg::with_name("sleep")
                .long("sleep")
                .takes_value(true)
                .help("How should the process sleep after mmapping all of the files deleting them"),
        )
        .arg(Arg::with_name(DIRECTORY_NAME).required(true).index(1))
        .get_matches();

    let number_of_files = matches
        .value_of("number_of_files")
        .unwrap_or("10")
        .parse()?;

    let pages_per_file = matches.value_of("pages_per_file").unwrap_or("10").parse()?;

    let page_size = matches.value_of("page_size").unwrap_or("1024").parse()?;

    let sleep_time = Duration::from_secs(matches.value_of("sleep").unwrap_or("10").parse()?);

    let directory = Path::new(matches.value_of(DIRECTORY_NAME).unwrap());

    if directory
        .read_dir()
        .with_context(|| format!("Failed to read directory: {:?}", directory))?
        .count()
        != 0
    {
        Err(Error::msg(format!(
            "The directory in {} is not empty",
            DIRECTORY_NAME
        )))?;
    }

    let page_content = vec![b'a'; page_size];

    let mut mmaps = Vec::with_capacity(number_of_files);

    let mut path = directory.to_path_buf();
    for i in 0..number_of_files {
        path.push(file_name(i));
        let file = if i == 0 {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)?;

            for _ in 0..pages_per_file {
                file.write_all(&page_content)?;
            }

            file
        } else {
            let mut src = directory.to_path_buf();
            src.push(file_name(0));

            copy(src, &path)?;

            OpenOptions::new().read(true).write(true).open(&path)?
        };

        let mmap = unsafe { Mmap::map(&file) }
            .with_context(|| format!("Failed to mmap file: {:?}", file))?;
        mmaps.push(mmap);

        path.pop();
    }

    sleep(sleep_time);

    for (i, mmap) in mmaps.into_iter().enumerate() {
        path.push(file_name(i));
        drop(mmap);
        remove_file(&path)?;
        path.pop();
    }

    Ok(())
}

fn file_name(id: usize) -> String {
    format!("file_{}", id)
}
