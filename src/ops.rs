use crate::error::Error;
use crate::disk::DiskId;
use crate::manifest::{Entry, Manifest};

use std::fs::{self, File};
use std::path::Path;
use std::io::{Read, Write};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

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

    // Copy and hash during copying
    let mut source_dir = File::open(source)?;
    let mut dest_dir = File::create_new(&dest)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = source_dir.read(&mut buf)?;
        if n == 0 {break;}
        dest_dir.write_all(&buf[..n]);
        hasher.update(&buf[..n]);
    }
    dest_dir.sync_all()?;
    let written = format!("{:x}", hasher.finalize());

    // Verify the hashes of the destination 
    // and source are equal
    let on_disk = hash_file(&dest)?;
    if on_disk != written {
        let _ = fs::remove_dir(&dest); // Cleanup and safe exit
        return Err(Error::HashMismatch{
            expected:written, 
            actual: on_disk, 
            path: dest})
    }

    // Commiting after offloading completes
    // delete the files.
    fs::remove_file(source)?;
    std::os::unix::fs::symlink(&dest, source);

    let entry = Entry {
       original_path: source.to_path_buf(),
       rel_path: rel_path.to_path_buf(),
       disk_id,
       sha256: written,
       offloaded_at: OffsetDateTime::now_utc(),
    };
    manifest.add(entry.clone());
    manifest.save()?;

    Ok(entry)

}




