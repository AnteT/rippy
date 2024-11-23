use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::io::{Error, Write};
use std::fs;

use rippy::args;
use rippy::tree;

/// Generate a `RippyArgs` struct from the provided arguments, which should contain the program name as the first option.
pub fn generate_args_from<S: Display>(args: impl AsRef<[S]>) -> args::RippyArgs {
    let args: Vec<String> = args.as_ref().iter().map(|s| s.to_string()).collect();
    args::parse_args(Some(args))
}

/// Simplify process of creating `TreeMap` types to use for comparison against output received versus output expected.
pub fn generate_tree_map(pairs: impl IntoIterator<Item = (String, tree::Tree)>) -> tree::TreeMap {
    let mut map = tree::TreeMap::default();
    for (key, value) in pairs {
        map.insert(key, value);
    }
    map
}

/// Custom Error type for testing
pub enum DirError {
    Io(Error),
    OverWrite(String),
    InvalidDirectory(String),
    #[allow(unused)]
    Other(String),
}
impl Display for DirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::OverWrite(err) => write!(f, "Overwrite error: {}", err),
            Self::InvalidDirectory(err) => write!(f, "Invalid directory error: {}", err),
            Self::Other(err) => write!(f, "Other error: {}", err),
        }  
    }
}
impl Debug for DirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Reuse Display impl for Debug to keep single source of error output
        write!(f, "{}", self)
    }
}
impl From<Error> for DirError {
    fn from(value: Error) -> Self {
        DirError::Io(value)
    }
}

#[derive(Debug, Clone)]
/// The root node directory to use for all subsequent directories and files.
pub struct RootDirectory(PathBuf);

/// To facilitate operations with the tuple struct and allow dereferencing.
impl Deref for RootDirectory {
    type Target = Path;
    /// Treat tuple struct as &Path when dereferenced.
    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}
impl Display for RootDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}
impl RootDirectory {
    /// Create a new root from the provided directory.
    pub fn new(path: impl AsRef<Path>) -> Self {
        RootDirectory(path.as_ref().to_path_buf())
    }
    /// Returns the root path.
    pub fn root(&self) -> &Path {
        self.0.as_path()
    }
    /// Create a single entry at the specified path, useful for creating exception directories or hidden files without extensions.
    pub fn create_file<T: Into<String>>(&self, path: impl AsRef<Path>, content: Option<T>) -> Result<(), DirError> {
        let targets = self.join(path.as_ref());
        if !targets.starts_with(self.root()) {
            // Return a custom error
            return Err(DirError::OverWrite(format!("Provided path '{}' risks overwriting existing directories outside of current test root", targets.display())));
        }
        if targets.parent().is_some() && !targets.exists() {
            // Create all intermediate directories
            let parent = targets.parent().unwrap();
            if !parent.exists() {
                // println!("Creating parent: {parent:?}");
            }
            // No need to guard this call since creates intermediates safely
            fs::create_dir_all(parent).map_err(|e| DirError::Io(e))?;
        }
        // Check if the target is a file (has a file name component)  && !targets.exists()
        if targets.file_name().is_some() && !targets.exists() {
            // Create the file (and write content if provided)
            // println!("Creating file target: {targets:?}");
            let mut file = fs::File::create(targets).map_err(|e| DirError::Io(e) )?;
            if let Some(data) = content {
                file.write_all(data.into().as_ref()).map_err(|e| DirError::Io(e))?;
            }
        }
        Ok(())            
    }
    /// Create a single directory entry at the specified path, useful for creating exception directories or empty directories.
    pub fn create_directory(&self, path: impl AsRef<Path>) -> Result<(), DirError> {
        let targets = self.join(path.as_ref());
        if !targets.starts_with(self.root()) {
            // Return a custom error
            return Err(DirError::OverWrite(format!("Provided path '{}' risks overwriting existing directories outside of current test root", targets.display())));
        }
        if targets.parent().is_some() && !targets.exists() {
            // Create all intermediate directories
            let parent = targets.parent().unwrap();
            if !parent.exists() {
                // println!("Creating parent: {parent:?}");
            }
            // No need to guard this call since creates intermediates safely
            fs::create_dir_all(parent).map_err(|e| DirError::Io(e))?;
        }
        // Check if the target is a file (has a file name component)  && !targets.exists()
        if targets.file_name().is_some() && !targets.exists() && targets.extension().is_none() {
            // Create the file (and write content if provided)
            // println!("Creating file target: {targets:?}");
            let _file = fs::create_dir(targets).map_err(|e| DirError::Io(e) )?;
        }
        Ok(())            
    }    
    /// Creates the specified path including any required intermediate directories and files if path contains valid file path.
    /// If a valid file path is specified, `contents` can be provided to populate the entry with.
    pub fn generate<T: Into<String>>(&self, path: impl AsRef<Path>, content: Option<T>) -> Result<(), DirError> {
        let targets = self.join(path.as_ref());
        if !targets.starts_with(self.root()) {
            // Return a custom error
            return Err(DirError::OverWrite(format!("Provided path '{}' risks overwriting existing directories outside of current test root", targets.display())));
        }
    
        if targets.parent().is_some() && !targets.exists() {
            // Create all intermediate directories
            let parent = targets.parent().unwrap();
            if !parent.exists() {
                // println!("Creating parent: {parent:?}");
            }
            // No need to guard this call since creates intermediates safely
            fs::create_dir_all(parent).map_err(|e| DirError::Io(e))?;
        }
        
        // Check if the target is a file (has a file name component)  && !targets.exists()
        if targets.file_name().is_some() && targets.extension().is_some() && !targets.exists() {
            // Create the file (and write content if provided)
            // println!("Creating file target: {targets:?}");
            let mut file = fs::File::create(targets).map_err(|e| DirError::Io(e) )?;
            if let Some(data) = content {
                file.write_all(data.into().as_ref()).map_err(|e| DirError::Io(e))?;
            }
        }
    
        Ok(())
    }

    /// Deletes the root directory and all its child contents to ensure zero artifacts are left once testing has finished.
    pub fn clean(&self) -> Result<(), DirError> {
        if !self.root().exists() {
            return Err(DirError::InvalidDirectory(format!("Provided path '{}' does not exist and so no deletion will be performed", self.root().display())))
        }
        fs::remove_dir_all(self.root()).map_err(|e| DirError::Io(e))?;
        Ok(())
    }
}