use std::{
  fs::File,
  io::{prelude::*, Seek, Write},
  iter::Iterator,
  path::Path,
};

use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};
use zip::{result::ZipError, write::FileOptions};

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub struct DebugInput {
  /// Collect various log files and package into a zip archive
  #[arg(long)]
  pub create_log_archive: bool,
}

impl DebugInput {
  pub async fn debug(&self) -> Result<()> {
    if self.create_log_archive {
      collect_logs(&["/var/log"], "/tmp/eksnode-logs.zip")?;
    }

    Ok(())
  }
}

fn collect_logs(src_dirs: &[&str], dst_file: &str) -> zip::result::ZipResult<()> {
  let path = Path::new(dst_file);
  let file = File::create(path).unwrap();

  for src_dir in src_dirs {
    if !Path::new(src_dir).is_dir() {
      return Err(ZipError::FileNotFound);
    }

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, &file)?;
  }

  Ok(())
}

fn zip_dir<T>(it: &mut dyn Iterator<Item = DirEntry>, prefix: &str, writer: T) -> zip::result::ZipResult<()>
where
  T: Write + Seek,
{
  let mut zip = zip::ZipWriter::new(writer);
  let options = FileOptions::default()
    .compression_method(zip::CompressionMethod::BZIP2)
    .unix_permissions(0o755);

  let mut buffer = Vec::new();
  for entry in it {
    let path = entry.path();
    let name = path.strip_prefix(Path::new(prefix)).unwrap();

    // Write file or directory explicitly
    // Some unzip tools unzip files with directory paths correctly, some do not!
    if path.is_file() {
      println!("adding file {path:?} as {name:?} ...");
      #[allow(deprecated)]
      zip.start_file_from_path(name, options)?;
      let mut f = File::open(path)?;

      f.read_to_end(&mut buffer)?;
      zip.write_all(&buffer)?;
      buffer.clear();
    } else if !name.as_os_str().is_empty() {
      // Only if not root! Avoids path spec / warning
      // and mapname conversion failed error on unzip
      println!("adding dir {path:?} as {name:?} ...");
      #[allow(deprecated)]
      zip.add_directory_from_path(name, options)?;
    }
  }
  zip.finish()?;
  Result::Ok(())
}
