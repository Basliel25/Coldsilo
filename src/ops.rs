use crate::error::Error;
use crate::disk::DiskId;
use crate::manifest::{Entry, Manifest};

use std::fs::{self, File};
use std::path::Path;
use std::io::{Read, Write};
use sha2::{Digest, Sha256};

fn hash_file(path: &Path) -> Result<String, Error> {

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();

    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn offload(
    source: &Path, 
    disk_id: DiskId, 
    disk_mount: &Path,
    rel_path: &Path, 
    manifest: &mut Manifest
    ) -> Result<Entry, Error> {

    // Symlink creation and sanity check
    let meta = fs::symlink_metadata(source)?;

    // If the file being offloaded is already
    // a symlink, file is already offloaded
    if meta.file_type().is_symlink() { 
        return Err(Error::AlreadyOffloaded(source.to_path_buf()));
    }
    // If the file is not a file type
    // Return as an error
    if !meta.is_file() {
        return Err(Error::SourceNotRegularFile(source.to_path_buf()));
    }

    // Resolving the destination
    // Checking collision 
    // Ensure parent directory exists
    // Create if it doesnt
    let dest = disk_mount.join(rel_path);
    if dest.exists() {Err(Error::DestinationExists(dest.to_path_buf()));}
    if let Some(parent) = dest.parent() {fs::create_dir_all(parent)?;}

    // Copy and hash loop
    Ok(0)

}




