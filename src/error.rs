use std::path::PathBuf;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    // Error Variants
    #[error("Source: {0} is a symlink, file already offloaded")]
    AleardyOffloaded(PathBuf), // Source is a symlink, already offloaded
    #[error("Source: {0} is not a regular file")]
    SourceNotRegularFile(PathBuf),

    #[error("Destination {0} already exists")]
    DestinationExists(PathBuf),
    
    #[error("Disk not mounted: {0}")]
    DiskNotMounted(PathBuf),

    #[error("Hashmismath on {path} expected: {expected}, found {actual}")]
    HashMismatch{expected: String, actual: String, path: PathBuf},



};

