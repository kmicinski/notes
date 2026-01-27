//! HTML templates and styling for the notes application.
//!
//! This module contains all CSS styles, JavaScript code, and HTML
//! generation functions for the web interface.
//!
//! ## Module Structure
//!
//! - `styles` - CSS constants and theme definitions
//! - `components` - Shared HTML components (nav bar, Smart Add, base template)
//! - `editor` - Monaco-based editor with PDF viewing
//! - `viewer` - View mode template with PDF support

mod styles;
mod components;
mod editor;
mod viewer;

// Re-export public items for backward compatibility
pub use styles::STYLE;
pub use components::{nav_bar, smart_add_html, base_html};
pub use editor::render_editor;
pub use viewer::render_viewer;
