use std::io;

use kedo_core::asyncify;

pub struct FsDirEntry {
    pub parent_path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
}

pub struct StdFileSystem;

impl StdFileSystem {
    pub fn read_file_evt(path: &str) -> io::Result<String> {
        let contents = std::fs::read_to_string(path)?;
        Ok(contents)
    }

    pub async fn read_file_async_evt(path: &str) -> io::Result<String> {
        let path = path.to_owned();
        let contents = asyncify(move || Self::read_file_evt(&path)).await?;
        Ok(contents)
    }

    pub fn write_file_evt(path: &str, data: &str) -> io::Result<()> {
        std::fs::write(path, data)?;
        Ok(())
    }

    pub async fn write_file_async_evt(path: &str, data: &str) -> io::Result<()> {
        let path = path.to_owned();
        let data = data.to_owned();
        asyncify(move || Self::write_file_evt(&path, &data)).await?;
        Ok(())
    }

    pub fn read_dir_evt(path: &str) -> io::Result<Vec<FsDirEntry>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let is_dir = metadata.is_dir();
            let is_file = metadata.is_file();
            let is_symlink = metadata.file_type().is_symlink();

            let parent_path = entry
                .path()
                .parent()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "parent path not found")
                })?
                .to_string_lossy()
                .to_string();
            let name = entry.file_name().to_string_lossy().to_string();

            entries.push(FsDirEntry {
                name,
                parent_path,
                is_dir,
                is_file,
                is_symlink,
            });
        }

        Ok(entries)
    }

    pub async fn read_dir_async_evt(path: &str) -> io::Result<Vec<FsDirEntry>> {
        let path = path.to_owned();
        let entries = asyncify(move || Self::read_dir_evt(&path)).await?;
        Ok(entries)
    }

    /// remove file, directory, or symlink
    pub fn remove_evt(path: &str, recursive: bool) -> io::Result<()> {
        // check type of file
        let metadata = std::fs::metadata(path)?;
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            if recursive {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_dir(path)?;
            }
        } else if metadata.file_type().is_symlink() {
            // support remove of non unix-like system
            #[cfg(not(unix))]
            {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "symlink removal is not supported on this platform",
                ));
            }

            #[cfg(unix)]
            {
                std::fs::remove_file(path)?;
            }
        } else {
            std::fs::remove_file(path)?;
        }

        Ok(())
    }

    pub async fn remove_async_evt(path: &str, recursive: bool) -> io::Result<()> {
        let path = path.to_owned();
        asyncify(move || Self::remove_evt(&path, recursive)).await?;
        Ok(())
    }
}
