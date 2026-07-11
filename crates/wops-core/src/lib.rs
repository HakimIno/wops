//! Application state and UI-to-core protocol for WOPS.

mod config;
mod protocol;
mod state;

pub use config::{ConfigError, Settings, ThemePreference};
pub use protocol::{Command, CoreChannels, Event, FrameStats, core_channels};
pub use state::{AppState, Scene, Source, SourceKind, SourceStatus};
