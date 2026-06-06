use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::Error;

//Declate marker files
const MARKER_DIR:&str = ".coldsilo";
const MARKER_FILE:&str = "disk.toml";

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
    let marker_dir = mount_path.join(MARKER_DIR);
    
    // Create file on disk if not existing
    fs::create_dir_all(&marker_dir)?;

    let marker = DiskMarker {
        diskId: DiskId::new(),
        label,
        created_at: OffsetDateTime::now_utc(),
   };

    fs::write(
        marker_dir.join(MARKER_FILE),
        toml::to_string_pretty(&marker)?,
    )?;

    Ok(marker)
}


/// Takes scan roots as a param 
/// so a test points it at a tempdir and the binary passes the
// udisks defaults. 
// No failing return since 
// A missing root or a corrupt marker is
// just means that disk isn't in the map
pub fn mounted_disks(scan_roots: &[PathBuf]) -> HashMap<DiskId, PathBuf> {
    let mut map = HashMap::new();

    for root in scan_roots {
        // read_dir errors if the root doesn't exist

        let entries = match fs::read_dir(root) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let mount = entry.path();
            let marker = mount.join(MARKER_DIR).join(MARKER_FILE);
            if !marker.exists() {
                continue;
            }
            // If corrupt or missing disk continues scanning the pool
            if let Ok(m) = read_marker(&marker) {
                map.insert(m.disk_id, mount);
            }
        }
    }
    map
}


