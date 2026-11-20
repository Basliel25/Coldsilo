use crate::error::Error; 
use crate::disk::{DiskId, mounted_disks};
use crate::manifest::{Entry, Manifest};

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::io::{Read, Write, ErrorKind};
use std::os::unix::fs::symlink;
use std::collections::HashMap;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

// Define a linkstate enum
#[derive(Debug)]
pub enum LinkOutcome {
    Created, 
    Unchanged, 
    Repointed, 
    Failed(Error)
}

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
    if dest.exists() {return Err(Error::DestinationExists(dest.to_path_buf()));}
    if let Some(parent) = dest.parent() {fs::create_dir_all(parent)?;}

    // Copy and hash during copying
    let mut source_dir = File::open(source)?;
    let mut dest_dir = File::create_new(&dest)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = source_dir.read(&mut buf)?;
        if n == 0 {break;}
        dest_dir.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
    }
    dest_dir.sync_all()?;
    let written = format!("{:x}", hasher.finalize());

    // Verify the hashes of the destination 
    // and source are equal
    let on_disk = hash_file(&dest)?;
    if on_disk != written {
        let _ = fs::remove_file(&dest); // Cleanup and safe exit
        return Err(Error::HashMismatch{
            expected:written, 
            actual: on_disk, 
            path: dest})
    }

    // Commiting after offloading completes
    // delete the files.
    fs::remove_file(source)?;
    std::os::unix::fs::symlink(&dest, source)?;

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

pub fn restore(target: &Path, manifest: &mut Manifest, disk_mount: &Path) -> Result<(), Error> {

    // purely mechanical restore, restore_path does policy declaration
    //
    // Lookup and cloning out what matches the target path
    let (rel_path, expected) = {
        let entry = manifest.entries.iter()
            .find(|e| e.original_path == target)
            .ok_or_else(|| Error::NotOffloaded(target.to_path_buf()))?;
        (entry.rel_path.clone(), entry.sha256.clone())
    };
    let blob = disk_mount.join(&rel_path);
    
    // copy blob from mounted disk to target while stream-hashing
    let parent = target.parent().expect("offload target must have a parent dir");
    let temp = parent.join(format!(
            ".coldsilo-tmp-{}",
            target.file_name().unwrap().to_string_lossy()

    ));

    // initalize src and dst
    let mut src = File::open(&blob)?;
    let mut dest = File::create_new(&temp)?;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 { break; }
        dest.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
    }

    dest.sync_all()?;
    let actual = format!("{:x}", hasher.finalize());

    // compare hash to entry.hash before renaming, if missmatch, throw HashMismatch error
    if actual != expected {
        fs::remove_file(&temp).ok();
        return Err(Error::HashMismatch {expected, actual, path: blob});
    }

    // atomically handle symlinks
    fs::rename(&temp, target)?;

    // Retain the manifest and save it
    manifest.entries.retain(|e| e.original_path != target);
    manifest.save()?;

    // delete the blob from stick
    fs::remove_file(&blob)?;

    Ok(())
}

pub fn restore_path(
    target: &Path,
    scan_roots: &[PathBuf],
    manifest: &mut Manifest) -> Result<(), Error> {

    // disk Owner of file with disk_id
    let disk_id = {
        let entry = manifest.entries.iter()
        .find(|e| e.original_path == target)
        .ok_or_else(|| Error::NotOffloaded(target.to_path_buf()))?;
        entry.disk_id
    };

    // query and map mounted disks
    let mounted = mounted_disks(scan_roots);
    // absent disks is just 'disk not present'
    let disk_mount = mounted
        .get(&disk_id)
        .ok_or_else(|| Error::DiskNotMounted(disk_id))?;

    restore(target, manifest, disk_mount)
}

// A manifest function to handle mismatch of path of middle link between disk and registered offload
// file
pub fn indirection_path(data_dir: &Path, disk_id:DiskId) -> PathBuf {
    // Takes the data directory on host disk
    // merges the available disks in the pool attaches
    // the disk_id registered by the offload function and returns
    // an absolute path
    data_dir.join("disks").join(disk_id.to_string())
}

fn link_one(data_dir: &Path, disk_id: DiskId, mount: &Path) -> Result<LinkOutcome, Error> {
    let link = indirection_path(data_dir, disk_id);
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::read_link(&link) {
        
        // If nothing is found create it
        Err(e) if e.kind() == ErrorKind::NotFound => {
            symlink(mount, &link)?;
            Ok(LinkOutcome::Created)
        }
        // Any other error will be propagated as an input mismatch error
        Err(e) => Err(e.into()),
        // Unchanged link remains the same
        Ok(current) if current.as_path() == mount => Ok(LinkOutcome::Unchanged),
        // Repoint wrong path by renaming
        Ok(_) => {
            let temp = link.with_file_name(format!("{}.tmp", disk_id));
            fs::remove_file(&temp).ok();
            symlink(mount, &temp)?;
            fs::rename(&temp, &link)?;
            Ok(LinkOutcome::Repointed)
        }
    }
}

pub fn refresh_direction(
    data_dir: &Path, 
    mounted: &HashMap<DiskId, PathBuf>, 
) -> Vec<(DiskId, LinkOutcome)> {
    mounted
        .iter()
        .map(|(disk_id, mount)| {
            let link_pass = match link_one(data_dir, *disk_id, mount){
                Ok(o)=>o,
                Err(e)=>LinkOutcome::Failed(e),
            };
            (*disk_id, link_pass)
        }
        ).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn indirection_path_test() {
        let disk_id = DiskId::new();
        let path = indirection_path(Path::new("/tmp/cs"), disk_id);

        assert_eq!(path.parent().unwrap(), Path::new("/tmp/cs/disks"));
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), disk_id.to_string());
    }

    #[test]
    fn link_one_creates_and_unchanged() {
        let data = tempdir().unwrap();
        let disk_id = DiskId::new();
        let mount = Path::new("/mnt/a");

       let first_link = link_one(data.path(), disk_id, mount).unwrap();
       assert!(matches!(first_link, LinkOutcome::Created));
       assert_eq!(fs::read_link(indirection_path(data.path(), disk_id)).unwrap(), mount);

       let second_link = link_one(data.path(), disk_id, mount).unwrap();
       assert!(matches!(second_link, LinkOutcome::Unchanged));
    }

    #[test]
    fn link_one_repoints_when_mount_moved() {
        let data = tempdir().unwrap();
        let disk_id = DiskId::new();

        link_one(data.path(), disk_id, Path::new("/mnt/a")).unwrap();
        let out = link_one(data.path(), disk_id, Path::new("/mnt/b")).unwrap();

        assert!(matches!(out, LinkOutcome::Repointed));
        assert_eq!(
            fs::read_link(indirection_path(data.path(), disk_id)).unwrap(),
            Path::new("/mnt/b")
        );

    }

    #[test]
    fn link_one_rejects_real_directory() {
        let data = tempdir().unwrap();
        let disk_id = DiskId::new();

        fs::create_dir_all(indirection_path(data.path(), disk_id)).unwrap();
        assert!(link_one(data.path(), disk_id, Path::new("/mnt/a")).is_err());
    }
    fn outcome_for(out: &[(DiskId, LinkOutcome)], id: DiskId) -> &LinkOutcome {
        &out.iter().find(|(i, _)| *i == id).unwrap().1
    }
    #[test]
    fn refresh_idempotent() {
        let data = tempdir().unwrap();
        let (disk_a, disk_b) = (DiskId::new(), DiskId::new());
        let mounted = HashMap:: from([
            (disk_a, PathBuf::from("/mnt/a")),
            (disk_b, PathBuf::from("/mnt/b"))
        ]);

        let first_disk = refresh_direction(data.path(), &mounted);
        assert!(matches!(outcome_for(&first_disk, disk_a), LinkOutcome::Created));
        assert!(matches!(outcome_for(&first_disk, disk_b), LinkOutcome::Created));

        let second_disk = refresh_direction(data.path(), &mounted);
        assert!(matches!(outcome_for(&second_disk, disk_a), LinkOutcome::Unchanged));
        assert!(matches!(outcome_for(&second_disk, disk_b), LinkOutcome::Unchanged));
    }

    #[test]
    fn refresh_handles_absent_disk() {
        let data = tempdir().unwrap();
        let (disk_a, absent_disk) = (DiskId::new(), DiskId::new());

        link_one(data.path(), absent_disk, Path::new("/mnt/gone")).unwrap();
        let mounted = HashMap::from([(disk_a, PathBuf::from("/mnt/a"))]);
        let out = refresh_direction(data.path(), &mounted);

        assert_eq!(out.len(), 1);
        assert_eq!(
            fs::read_link(indirection_path(data.path(), absent_disk)).unwrap(), 
            Path::new("/mnt/gone")
        );
    }

    #[test]
    fn refresh_one_bad_disk_dont_affect_pool() {
        let data = tempdir().unwrap();
        let (good_disk, bad_disk) = (DiskId::new(), DiskId::new());
        fs::create_dir_all(indirection_path(data.path(), bad_disk)).unwrap();

        let mounted = HashMap::from([
            (good_disk, PathBuf::from("/mnt/good")),
            (bad_disk, PathBuf::from("/mnt/bad"))
        ]);

        let out = refresh_direction(data.path(), &mounted);

        assert!(matches!(outcome_for(&out, good_disk), LinkOutcome::Created));
        assert!(matches!(outcome_for(&out, bad_disk), LinkOutcome::Failed(_)));
    }
}
