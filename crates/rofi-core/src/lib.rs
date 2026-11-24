//! Shared building blocks for the Rofi Hub provider ecosystem.

pub mod app;
pub mod error;
pub mod item;
pub mod manifest;
pub mod notify;
pub mod paths;
pub mod rofi;
pub mod theme;
pub mod which;

pub use error::{Error, Result};
pub use item::{Item, escape_markup};
pub use manifest::{ManifestSet, ProviderManifest};
pub use rofi::{DmenuOptions, Outcome, Rofi};
pub use theme::{Mode, Theme};
