///! 
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::Error;

//Declate marker files
//MARKER_DIR = .coldsilo
//MARKER_FILE = disk.toml

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiskId(Uuid); 

impl DiskId {
   pub fn new() -> Self {
        DiskId(Uuid::new_v4())
   }
}

// A marker that lives on the disk
// Serializes to <mount>/.coldsilo/disk.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMarker {
    pub diskId: DiskId,
    pub label: String,
    // Maybe will have to select a time format for serde
    pub created_at: OffsetDateTime,
}

pub fn init(mount_path: &Path, label: String) -> Result<DiskMarker, Error> {
    // Declare Disk dir
    // Marker_dir = mount_path.join(MARKER_DIR)
    // fs::create_dir_all(&Marker_dir)?

    let marker = DiskMarker {
        diskId: DiskId::new(),
        label,
        created_at: OffsetDateTime::now_utc(),
   };

    fs::write(
        //marker_dir joined with MARKER_FILE
        //toml::to string pretty(&marker)?
        )

    Ok(marker)

}
