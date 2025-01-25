use crate::module_scanner::ModuleScanner;

pub struct ModuleManager {
    scanner: ModuleScanner,
    entries: Vec<String>,
    externals: Vec<String>,
}

impl ModuleManager {
    pub fn new(root: &str) -> Self {
        ModuleManager {
            scanner: ModuleScanner::new(root),
            entries: Vec::new(),
            externals: Vec::new(),
        }
    }

    pub fn scan(&mut self) -> std::io::Result<()> {
        self.scanner.scan()?;
        self.entries.clear();
        self.externals.clear();

        let modules = self.scanner.get_modules();
        for module in modules {
            if module.has_index {
                self.externals.push(format!("@kedo/{}", module.name));
                self.entries.push(format!("{}/index.ts", module.path));
            }
            if module.has_internals {
                self.entries.push(format!("{}/_internals.ts", module.path));
                self.externals
                    .push(format!("@kedo:int/std/{}", module.name));
            }
        }
        Ok(())
    }

    pub fn add_entry(&mut self, entry: String) {
        self.entries.push(entry);
    }

    pub fn remove_entry(&mut self, entry: &str) {
        self.entries.retain(|e| e != entry);
    }

    pub fn add_external_module(&mut self, module: String) {
        self.externals.push(module);
    }

    pub fn remove_external_module(&mut self, module: &str) {
        self.externals.retain(|m| m != module);
    }

    pub fn get_entries(&self) -> &Vec<String> {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.scanner.len()
    }

    pub fn get_externals(&self) -> &Vec<String> {
        &self.externals
    }
}
