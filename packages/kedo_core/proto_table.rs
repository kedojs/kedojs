use std::collections::HashMap;

use rust_jsc::JSObject;

#[derive(Default)]
pub struct ProtoTable {
    pub prototypes: HashMap<String, JSObject>,
}

impl Drop for ProtoTable {
    fn drop(&mut self) {
        for prototype in self.prototypes.values() {
            prototype.unprotect();
        }
    }
}

impl ProtoTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: String, prototype: JSObject) {
        prototype.protect();
        self.prototypes.insert(name, prototype);
    }

    pub fn get(&self, name: &str) -> Option<&JSObject> {
        self.prototypes.get(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<JSObject> {
        let object = self.prototypes.remove(name);
        return if let Some(object) = object {
            object.unprotect();
            Some(object)
        } else {
            None
        };
    }

    pub fn contains(&self, name: &str) -> bool {
        self.prototypes.contains_key(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &JSObject> {
        self.prototypes.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut JSObject> {
        self.prototypes.values_mut()
    }

    pub fn clear(&mut self) {
        self.prototypes.clear();
    }

    pub fn len(&self) -> usize {
        self.prototypes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prototypes.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.prototypes.keys()
    }
}
