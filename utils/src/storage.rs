use std::fs::{DirBuilder, File};
use std::io::Write;
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

    fn strip_to_root(&self, path: &Path) -> PathBuf {
        path.strip_prefix(self.root.clone()).unwrap().to_path_buf()
    }

    pub fn add_folder(
        &self,
        folder_name: &str,
        path: Option<&Path>,
    ) -> Result<PathBuf, std::io::Error> {
        let ancestor_path = match path {
            Some(ap) => self.root.join(ap),
            None => self.root.clone(),
        };
        let folder_path = ancestor_path.join(folder_name);
        let new_path = self.strip_to_root(&folder_path);
        DirBuilder::new()
            .recursive(true)
            .create(folder_path)
            .map(|_| new_path)
    }

    pub fn search_item(
        &self,
        folder_path: Option<&Path>,
        item_name: &str,
    ) -> Result<Option<PathBuf>, std::io::Error> {
        let absolute_folder_path = match folder_path {
            Some(ap) => self.root.join(ap),
            None => self.root.clone(),
        };
        if absolute_folder_path.is_dir() {
            return Ok(absolute_folder_path
                .read_dir()?
                .flat_map(|res| res.map(|e| e.path()))
                .find(|el| {
                    el.file_stem()
                        .filter(|&os| os.to_str().filter(|&s| s == item_name).is_some())
                        .is_some()
                })
                .map(|path| self.strip_to_root(&path)));
        }
        Ok(None)
    }

    pub fn save_file(
        &self,
        folder_path: &Path,
        file_name: &str,
        extension: &str,
        content: &[u8],
    ) -> Result<(), std::io::Error> {
        let mut path = self.root.join(folder_path);
        path.set_file_name(file_name);
        path.set_extension(extension);
        let mut buffer = File::create(path)?;
        buffer.write_all(content)?;
        Ok(())
    }

    pub fn read_file(&self, _path: &Path) -> Result<Vec<u8>, std::io::Error> {
        todo!()
    }
}
