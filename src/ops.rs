use crate::error::Error;
use crate::disk::DiskId;
use crate::manifest::{Entry, Manifest};

use std::fs::{self, File};
use std::path::Path;
use std::io::{Read, Write};
use sha2::{Digest, Sha256};

fn hash_file(path: &Path) -> Result<String, Error> {todo!()}

pub fn offload(
    source: &Path, 
    disk_id: DiskId, 
    disk_mount: &Path,
    rel_path: &Path, 
    manifest: &mut Manifest
    ) -> Result<Entry, Error> {todo!()}




