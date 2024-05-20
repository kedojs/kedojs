use std::collections::HashMap;

use rust_jsc::JSClass;

#[derive(Default)]
pub struct ClassManager {
    pub classes: HashMap<String, JSClass>,
}

impl ClassManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, class: JSClass) {
        self.classes.insert(class.name().to_string(), class);
    }

    pub fn get(&self, name: &str) -> Option<&JSClass> {
        self.classes.get(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<JSClass> {
        self.classes.remove(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &JSClass> {
        self.classes.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut JSClass> {
        self.classes.values_mut()
    }

    pub fn clear(&mut self) {
        self.classes.clear();
    }

    pub fn len(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.classes.keys()
    }

    pub fn register(&mut self, ctx: &rust_jsc::JSContext) {
        for class in self.classes.values() {
            class.register(ctx).unwrap();
        }
    }
}
