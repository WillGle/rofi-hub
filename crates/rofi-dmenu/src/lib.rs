//! Shared dmenu, theme, path, and UI helpers for the rofi-apps ecosystem.
//!
//! This library is the only shared dependency between the apps. It never
//! depends on the optional menu or the installer, and it never reads
//! hub-owned environment variables.

pub mod app;
#[cfg(feature = "category")]
pub mod category;
pub mod error;
pub mod item;
#[cfg(feature = "manifest")]
pub mod manifest;
#[cfg(feature = "notify")]
pub mod notify;
pub mod paths;
pub mod rofi;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod theme;
pub mod ui;
pub mod which;

#[cfg(feature = "category")]
pub use category::Category;
pub use error::{Error, Result};
pub use item::{Item, escape_markup};
#[cfg(feature = "manifest")]
pub use manifest::{ManifestSet, ProviderManifest};
pub use rofi::{DmenuOptions, Outcome, Rofi};
pub use theme::{ChromeProfile, Mode, Theme};
pub use ui::Level;
