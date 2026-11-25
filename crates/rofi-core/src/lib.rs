//! Shared building blocks for the Rofi Hub provider ecosystem.

pub mod app;
pub mod category;
pub mod error;
pub mod item;
pub mod manifest;
pub mod notify;
pub mod paths;
pub mod rofi;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod theme;
pub mod ui;
pub mod which;

pub use category::Category;
pub use error::{Error, Result};
pub use item::{Item, escape_markup};
pub use manifest::{ManifestSet, ProviderManifest};
pub use rofi::{DmenuOptions, Outcome, Rofi};
pub use theme::{ChromeProfile, Mode, Theme};
pub use ui::Level;
