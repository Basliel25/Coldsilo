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
    ) -> Result<Entry, Error> {todo!()}




