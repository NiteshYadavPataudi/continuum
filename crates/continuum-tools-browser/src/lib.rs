//! Browser automation tool runners (Playwright and chromiumoxide).
#![warn(missing_docs)]

#[cfg(feature = "playwright")]
mod playwright;
#[cfg(not(feature = "playwright"))]
mod chromiumoxide;

#[cfg(feature = "playwright")]
pub use playwright::PlaywrightRunner;
#[cfg(not(feature = "playwright"))]
pub use chromiumoxide::ChromiumoxideRunner;
