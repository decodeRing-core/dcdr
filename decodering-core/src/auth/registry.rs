use std::collections::HashMap;

use crate::auth::method::AuthMethod;

#[derive(Default)]
pub struct AuthRegistry {
    methods: HashMap<String, Box<dyn AuthMethod>>,
}

impl AuthRegistry {
    pub fn register(&mut self, method: Box<dyn AuthMethod>) {
        self.methods.insert(method.kind(), method);
    }
    pub fn get(&self, kind: &str) -> Option<&dyn AuthMethod> {
        self.methods.get(kind).map(AsRef::as_ref)
    }
}
