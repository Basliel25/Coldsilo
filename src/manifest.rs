use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::disk::DiskId;
use crate::error::Error;

/// An entry per offloaded file
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct Manifest {
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
    //
    // Save manifest

}
