use std::error::Error;
use clap::Parser;
use data_encoding::HEXUPPER;
use multimap::MultiMap;
use ring::digest::{Context, Digest, SHA256};
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "findclone", about = "A tool to find duplicate files")]
struct Args {
    /// Path to root
    path: String,
}


fn compare_files(path_1: &Path, path_2: &Path) -> Result<bool, std::io::Error> {
    let file_1 = File::open(path_1)?;
    let file_2 = File::open(path_2)?;

    let mut reader_1 = BufReader::new(file_1);
    let mut reader_2 = BufReader::new(file_2);

    let mut buffer_1 = [0; 8192];
    let mut buffer_2 = [0; 8192];

    loop {
        let bytes_read_1 = reader_1.read(&mut buffer_1)?;
        let bytes_read_2 = reader_2.read(&mut buffer_2)?;

        if bytes_read_1 != bytes_read_2 {
            return Ok(false);
        }

        if bytes_read_1 == 0 {
            break;
        }

        if buffer_1[..bytes_read_1] != buffer_2[..bytes_read_1] {
            return Ok(false);
        }
    }
    Ok(true)
}

// From rust cookbook
fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest, std::io::Error> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 8192];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}


fn main() -> Result<(), Box<dyn Error>>{
    let mut filemap_size = MultiMap::new();
    let mut filemap_hash = MultiMap::new();

    
    let cli = Args::parse();
    let root_path = cli.path ;

    for entry in WalkDir::new(root_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            if e.file_type().is_file() {
                Some(e)
            } else {
                None
            }
        })
    {
        match fs::metadata(entry.path()) {
            Ok(i) => {
                filemap_size.insert(i.len(), entry);
            }
            Err(e) => {
                println!("Error in {}: {}", entry.path().display(), e);
            }
        }
    }
    for (_key, value) in filemap_size.iter_all() {
        if value.len() > 1 {
            for item in value {
                let input = File::open(item.path())?;
                let reader = BufReader::new(input);
                let digest = sha256_digest(reader)?;

                filemap_hash.insert(HEXUPPER.encode(digest.as_ref()), item)
            }
        }
    }

    for (_key, value) in filemap_hash.iter_all() {
        if value.len() > 1 {
            for (index, item) in value.iter().enumerate() {
                for item_2 in value.iter().skip(index + 1) {
                    if compare_files(item.path(), item_2.path())? {
                        println!(
                            "{} and {} are the same",
                            item.path().display(),
                            item_2.path().display()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
