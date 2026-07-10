use serde::{Deserialize, Serialize};

use crate::Settings;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    pub scenes: Vec<Scene>,
    pub sources: Vec<Source>,
    pub settings: Settings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scenes: vec![Scene {
                name: "Scene".to_owned(),
            }],
            sources: Vec::new(),
            settings: Settings::default(),
        }
    }
}
