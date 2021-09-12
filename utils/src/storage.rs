use std::fs::{DirBuilder, File, ReadDir};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use bincode;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
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

    fn get_root_path(&self, path: Option<&Path>) -> PathBuf {
        match path {
            Some(p) => p.to_path_buf(),
            None => self.root.clone(),
        }
    }

    pub fn add_folder(
        &self,
        folder_name: &str,
        path: Option<&Path>,
    ) -> Result<PathBuf, std::io::Error> {
        let path = self.get_root_path(path);
        let folder_path = path.join(folder_name);
        DirBuilder::new()
            .recursive(true)
            .create(folder_path.clone())
            .map(|_| folder_path)
    }

    pub fn iterate_folder(
        &self,
        folder_name: &str,
        path: Option<&Path>,
    ) -> Result<ReadDir, std::io::Error> {
        let path = self.get_root_path(path);
        let folder_path = path.join(folder_name);
        folder_path.read_dir()
    }

    pub fn search_item(
        &self,
        path: Option<&Path>,
        item_name: &str,
        extension: Option<&str>,
    ) -> Result<Option<PathBuf>, std::io::Error> {
        let path = self.get_root_path(path);
        if path.is_dir() {
            return Ok(path
                .read_dir()?
                .flat_map(|res| res.map(|e| e.path()))
                .find(|el| {
                    let ok_name = el
                        .file_stem()
                        .and_then(|os| os.to_str())
                        .filter(|&name| name == item_name)
                        .is_some();
                    let ok_ext = match extension {
                        Some(ext) => el
                            .extension()
                            .and_then(|os| os.to_str())
                            .filter(|&extension| extension == ext)
                            .is_some(),
                        None => true,
                    };
                    ok_name && ok_ext
                }));
        }
        Ok(None)
    }

    pub fn save_file(
        &self,
        path: Option<&Path>,
        file_name: &str,
        extension: &str,
        content: &[u8],
    ) -> Result<PathBuf, std::io::Error> {
        let mut path = self.get_root_path(path).join(file_name);
        path.set_extension(extension);
        File::create(path.clone())
            .and_then(|mut file| file.write_all(content))
            .map(|_| path)
    }

    pub fn save_file_object<T: Serialize>(
        &self,
        path: Option<&Path>,
        file_name: &str,
        extension: &str,
        content: T,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let serialized = bincode::serialize(&content)?;
        Ok(self.save_file(path, file_name, extension, &serialized)?)
    }

    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>, std::io::Error> {
        File::open(path).and_then(|mut file| {
            let mut buffer = vec![];
            file.read_to_end(&mut buffer)?;
            Ok(buffer)
        })
    }

    pub fn read_file_object<T>(&self, path: &Path) -> Result<T, Box<dyn std::error::Error>>
    where
        for<'de> T: Deserialize<'de>,
    {
        let file = File::open(path)?;
        let deserialized = bincode::deserialize_from(file)?;
        Ok(deserialized)
    }

    pub fn delete_item(&self, path: &Path) -> Result<(), std::io::Error> {
        if path.is_file() {
            return std::fs::remove_file(path);
        } else if path.is_dir() {
            return std::fs::remove_dir_all(path);
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Item not found",
        ))
    }
}
