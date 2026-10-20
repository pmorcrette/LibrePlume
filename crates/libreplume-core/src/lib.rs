//! LibrePlume core engine: spelling, grammar, conjugation, typography and style checks.
//!
//! ```
//! use libreplume_core::{check, CheckOptions, Language};
//!
//! let opts = CheckOptions { language: Some(Language::Fr), ..Default::default() };
//! let result = check("Il à mangé.", &opts);
//! assert_eq!(result.matches[0].replacements[0], "a");
//! ```

pub mod affix;
pub mod conjugation;
pub mod engine;
pub mod resources;
pub mod rules;
pub mod text;
pub mod types;

pub use conjugation::{Conjugation, ConjugationError};
pub use engine::{
    check, conjugate, correct, detect_language, rules as list_rules, CheckOptions, CheckResult, Correction, Match,
    Stats,
};
pub use types::*;

/// Crate version, exposed by the server.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
