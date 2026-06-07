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
    #[serde(with = "time::serde::rfc3339")]
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
                map.insert(m.diskId, mount);
            }
        }
    }
    map
}

fn read_marker(path: &Path) -> Result<DiskMarker, Error> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

#[cfg(test)]
mod tests {

    use super::*;
    use tempfile::tempdir;
    
    /// Three tests:
    /// init writes a marker that reads back identically
    #[test]
    fn init_reads_identicall_roundtrip() {
        let stick = tempdir().unwrap();
        let written = init(stick.path(), "notHomework-1".into()).unwrap();

        let marker_path = stick.path().join(MARKER_DIR).join(MARKER_FILE);
        let read_back = read_marker(&marker_path).unwrap();

        assert_eq!(written.diskId, read_back.diskId);
        assert_eq!(read_back.label, "notHomework-1");

    }
    /// a planted stik keyed by disk_id with mount path
    #[test]
    fn planted_stick_id_and_mount_path(){

        let root = tempdir().unwrap();
        let mount = root.path().join("notHomework-1");

        std::fs::create_dir(&mount).unwrap();
        let marker = init(&mount, "notHomework-1".into()).unwrap();

        let found = mounted_disks(&[root.path().to_path_buf()]);

        assert_eq!(found.len(), 1);
        assert_eq!(found.get(&marker.diskId), Some(&mount));
    }
    /// different stick tests like :
    ///         - good
    ///         - corrupt
    ///         - nonexistent
    #[test]
    fn three_stick_pool_test() {

        let root = tempdir().unwrap();

        // good stick
        let good = root.path().join("good");
        std::fs::create_dir(&good).unwrap();
        let good_marker = init(&good, "good".into()).unwrap();

        // one corrupt stick
        let corrupt = root.path().join("corrupt");
        std::fs::create_dir(corrupt.join(MARKER_DIR).as_path()).unwrap_or_default();
        std::fs::create_dir_all(corrupt.join(MARKER_DIR)).unwrap();
        std::fs::write(corrupt.join(MARKER_DIR).join(MARKER_FILE), "not toml {{{").unwrap();

        // absent root
        let absent = root.path().join("doesnt exist");

        let found = mounted_disks(&[
            root.path().to_path_buf(),
            absent,
        ]);

        // just the good disk
        assert_eq!(found.len(), 1);
        assert!(found.contains_key(&good_marker.diskId));
    }

}
