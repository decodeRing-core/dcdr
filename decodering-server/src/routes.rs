use actix_web::web::ServiceConfig;
use std::collections::HashMap;
use std::sync::Arc;

pub mod app;
pub mod config;
pub mod doc;
pub mod osl;
pub mod raft;
pub mod system;

type Configurator = Arc<dyn Fn(&mut ServiceConfig) + Send + Sync + 'static>;

#[derive(Clone, Default)]
pub struct RouteExtensions {
    root: Vec<Configurator>,
    scopes: HashMap<&'static str, Vec<Configurator>>,
}

impl RouteExtensions {
    pub fn extend_root(&mut self, f: impl Fn(&mut ServiceConfig) + Send + Sync + 'static) {
        self.root.push(Arc::new(f));
    }
    pub fn extend_scope(
        &mut self,
        scope: &'static str,
        f: impl Fn(&mut ServiceConfig) + Send + Sync + 'static,
    ) {
        self.scopes.entry(scope).or_default().push(Arc::new(f));
    }
    pub fn apply_root(&self, cfg: &mut ServiceConfig) {
        for f in &self.root {
            f(cfg);
        }
    }
    pub fn apply_scope(&self, scope: &'static str, cfg: &mut ServiceConfig) {
        if let Some(fs) = self.scopes.get(scope) {
            for f in fs {
                f(cfg);
            }
        }
    }
}
