use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::disk::DiskId;
use crate::error::Error;

/// An entry per offloaded file
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub original_path: PathBuf, // where the symlink now sits at
    pub rel_path: PathBuf, // where the data lives on the stickk
    pub disk_id: DiskId, // The diskId 
    pub sha256: String, // captured at offload
    
    #[serde(with = "time::serde::rfc3339")]
    pub offloaded_at: OffsetDateTime,
}

/// A complete manifest of every entry,
/// a flat vector that serializes to toml
#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

impl Manifest {
    pub fn add(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    // Group by disk
    pub fn groub_by_disk(&self) -> HashMap<DiskId, Vec<&Entry>> {
        let mut map: HashMap<DiskId, Vec<&Entry>> = HashMap::new();
        for e in &self.entries {
            map.entry(e.disk_id).or_default().push(e);
        }
        map
    }
    // load from an explicit path
    pub fn load(path: &Path) -> Result<Self, Error> {
       match fs::read_to_string(path) {
           Ok(s) => {
               let mut manifest:Manifest = toml::from_str(&s)?;
               manifest.path = path.to_path_buf();
               Ok(toml::from_str(&s)?)
           },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
Ok(Manifest::default())
            }
            Err(e) => Err(e.into()),
       }
    }
    // Save path
    pub fn save(&self) -> Result<(), Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, toml::to_string_pretty(&self.entries)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::DiskId;
    use std::path::PathBuf;
    use time::OffsetDateTime;

    use tempfile::tempdir;

    fn dummy_entry(disk_id: DiskId, name: &str) -> Entry{
       Entry {
            disk_id,
            original_path: PathBuf::from(format!("/home/Music/{name}")),
            rel_path: PathBuf::from(format!("Music/{name}")),
            sha256: "deadbeat".repeat(3),
            offloaded_at: OffsetDateTime::now_utc(),
        }
    } 
    #[test]
    fn manifest_identical_after_roundtrip() { 
       let root = tempdir().unwrap();
       let path = root.path().join("manifest.toml");
       let disk_id = DiskId::new();

       let entry = dummy_entry(disk_id, "yomama");
       let mut manifest = Manifest::default();

       manifest.add(entry);
       manifest.save(&path).unwrap();

       let manifest_loaded = Manifest::load(&path).unwrap();

       assert_eq!(manifest, manifest_loaded);
    }
    #[test]
    fn manifest_empty_on_load_path_nonexistsent() {
        let root = tempdir().unwrap();
        let path = root.path().join("doesnt_exist.toml");

        let manifest: Manifest = Manifest::load(&path).unwrap();

        assert!(manifest.entries.is_empty());
    } 
    fn grouping_different_diskId_correct_count() {todo!()} 

}
