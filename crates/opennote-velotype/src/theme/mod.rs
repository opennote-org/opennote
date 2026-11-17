//! Theme configuration and global theme access.
//!
//! Themes are JSON-serializable so editor colors, spacing, and typography can
//! be swapped without changing the runtime logic.

#[expect(
    clippy::module_inception,
    reason = "Keep the existing theme implementation module and re-export paths stable."
)]
mod theme;
pub use theme::*;
