extern crate walkdir;
use std::fs::{self, OpenOptions};
use std::io;
use std::io::Write;
use std::path::Path;
use walkdir::WalkDir;

#[cfg(windows)]
fn windows_drive_roots() -> Vec<String> {
    let mut drives = Vec::new();

    for letter in 'A'..='Z' {
        let drive = format!("{}:\\", letter);
        if Path::new(&drive).exists() {
            drives.push(drive);
        }
    }

    drives
}

fn ensure_parent_dir(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    Ok(())
}

pub fn run_filesystem_service(output_path: &str) -> io::Result<usize> {
    ensure_parent_dir(output_path)?;

    let mut output_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(output_path)?;

    let mut files_scanned = 0usize;

    #[cfg(windows)]
    {
        for drive in windows_drive_roots() {
            for entry in WalkDir::new(&drive).into_iter().filter_map(Result::ok) {
                if entry.file_type().is_file() {
                    writeln!(output_file, "{}", entry.path().display())?;
                    files_scanned += 1;
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        for entry in WalkDir::new("/").into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                writeln!(output_file, "{}", entry.path().display())?;
                files_scanned += 1;
            }
        }
    }

    output_file.flush()?;
    Ok(files_scanned)
}
