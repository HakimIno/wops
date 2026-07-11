use serde::{Deserialize, Serialize};

use crate::Settings;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    #[default]
    Screen,
    Window,
    Webcam,
    Image,
    Color,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    #[default]
    Starting,
    Active,
    Lost,
    Error,
    Stopped,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: u64,
    pub name: String,
    pub kind: SourceKind,
    pub status: SourceStatus,
    pub visible: bool,
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
