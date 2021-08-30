use std::fs::DirBuilder;
use std::path::{Path, PathBuf};

pub struct FsStorageHelper {
    root: PathBuf,
}

impl FsStorageHelper {
    pub fn new(root_path: &Path) -> Result<Self, std::io::Error> {
        DirBuilder::new()
            .recursive(true)
            .create(root_path)
            .map(|_| Self {
                root: root_path.to_path_buf(),
            })
    }

    pub fn add_container(self, path: &Path) -> Result<Self, std::io::Error> {
        DirBuilder::new()
            .recursive(true)
            .create(self.root.join(path).as_path())
            .map(|_| self)
    }
}
