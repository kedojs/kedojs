use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModuleEntry {
    pub path: String,
    pub name: String,
    pub has_index: bool,
    pub has_internals: bool,
}

pub struct ModuleScanner {
    root_path: PathBuf,
    modules: Vec<ModuleEntry>,
}

impl ModuleScanner {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        ModuleScanner {
            root_path: root_path.as_ref().to_path_buf(),
            modules: Vec::new(),
        }
    }

    pub fn scan(&mut self) -> std::io::Result<()> {
        self.modules.clear();

        // Scan root directory
        if self.root_path.is_dir() {
            for entry in fs::read_dir(&self.root_path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let dir_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let has_index =
                        path.join("index.ts").exists() || path.join("index.js").exists();
                    let has_internals = path.join("_internals.ts").exists();

                    self.modules.push(ModuleEntry {
                        path: dir_name.clone(),
                        name: dir_name,
                        has_index,
                        has_internals,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn get_modules(&self) -> &Vec<ModuleEntry> {
        &self.modules
    }
}
