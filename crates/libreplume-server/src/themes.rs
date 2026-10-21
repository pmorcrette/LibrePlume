//! Interface themes: a theme is a set of CSS custom properties.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Theme {
    pub id: String,
    pub name: String,
    /// Whether the theme has a dark background (used for the browser colour scheme).
    #[serde(default)]
    pub dark: bool,
    #[serde(default)]
    pub builtin: bool,
    /// CSS custom properties without the leading dashes, e.g. `"accent": "#5b4bdb"`.
    pub variables: BTreeMap<String, String>,
}

/// Every variable a theme can set, with a description for editors and AI agents.
pub const THEME_VARIABLES: &[(&str, &str)] = &[
    ("bg", "Page background"),
    ("surface", "Cards and editor background"),
    ("surface-2", "Secondary surfaces (toolbar, inputs)"),
    ("border", "Borders and separators"),
    ("text", "Main text colour"),
    ("text-muted", "Secondary text colour"),
    ("accent", "Accent colour (buttons, links, focus)"),
    ("accent-contrast", "Text colour on top of the accent"),
    ("spelling", "Underline colour for spelling issues"),
    ("grammar", "Underline colour for grammar issues"),
    ("conjugation", "Underline colour for conjugation issues"),
    ("typography", "Underline colour for typography issues"),
    ("punctuation", "Underline colour for punctuation issues"),
    ("style", "Underline colour for style suggestions"),
    ("font-ui", "Font stack of the interface"),
    ("font-text", "Font stack of the edited text"),
    ("text-size", "Font size of the edited text, e.g. 1.125rem"),
    ("radius", "Corner radius, e.g. 14px"),
];

fn theme(id: &str, name: &str, dark: bool, vars: &[(&str, &str)]) -> Theme {
    let mut variables: BTreeMap<String, String> = BTreeMap::new();
    let defaults: &[(&str, &str)] = &[
        ("font-ui", "\"Inter\", \"Segoe UI\", system-ui, -apple-system, sans-serif"),
        ("font-text", "\"Literata\", \"Iowan Old Style\", Georgia, serif"),
        ("text-size", "1.125rem"),
        ("radius", "14px"),
    ];
    for (k, v) in defaults.iter().chain(vars) {
        variables.insert((*k).into(), (*v).into());
    }
    Theme { id: id.into(), name: name.into(), dark, builtin: true, variables }
}

pub fn builtin_themes() -> Vec<Theme> {
    vec![
        theme(
            "plume",
            "Plume",
            false,
            &[
                ("bg", "#f6f3ee"),
                ("surface", "#fffdf9"),
                ("surface-2", "#efeae2"),
                ("border", "#e2dbd0"),
                ("text", "#24211d"),
                ("text-muted", "#6f675c"),
                ("accent", "#4f46e5"),
                ("accent-contrast", "#ffffff"),
                ("spelling", "#e5484d"),
                ("grammar", "#f08c00"),
                ("conjugation", "#d6409f"),
                ("typography", "#0091ff"),
                ("punctuation", "#12a594"),
                ("style", "#8e4ec6"),
            ],
        ),
        theme(
            "encre",
            "Encre",
            true,
            &[
                ("bg", "#14151a"),
                ("surface", "#1c1d24"),
                ("surface-2", "#25262f"),
                ("border", "#30323d"),
                ("text", "#e9e7e2"),
                ("text-muted", "#9a978f"),
                ("accent", "#e2b553"),
                ("accent-contrast", "#1b1708"),
                ("spelling", "#ff6369"),
                ("grammar", "#ffa94d"),
                ("conjugation", "#f76fc0"),
                ("typography", "#52a9ff"),
                ("punctuation", "#3dd6c0"),
                ("style", "#b18aff"),
            ],
        ),
        theme(
            "sepia",
            "Sépia",
            false,
            &[
                ("bg", "#efe4d2"),
                ("surface", "#f8f0e2"),
                ("surface-2", "#e8dbc5"),
                ("border", "#d9c8ab"),
                ("text", "#3b2f22"),
                ("text-muted", "#7a6650"),
                ("accent", "#9c4f2c"),
                ("accent-contrast", "#fff8ef"),
                ("spelling", "#c2362f"),
                ("grammar", "#c76b00"),
                ("conjugation", "#a1307a"),
                ("typography", "#2d6aa3"),
                ("punctuation", "#2e7d6b"),
                ("style", "#6f4fa0"),
            ],
        ),
        theme(
            "nord",
            "Nord",
            true,
            &[
                ("bg", "#242933"),
                ("surface", "#2e3440"),
                ("surface-2", "#3b4252"),
                ("border", "#434c5e"),
                ("text", "#eceff4"),
                ("text-muted", "#a3acbd"),
                ("accent", "#88c0d0"),
                ("accent-contrast", "#1f242d"),
                ("spelling", "#bf616a"),
                ("grammar", "#d08770"),
                ("conjugation", "#b48ead"),
                ("typography", "#81a1c1"),
                ("punctuation", "#8fbcbb"),
                ("style", "#a3be8c"),
                ("font-ui", "\"IBM Plex Sans\", system-ui, sans-serif"),
            ],
        ),
        theme(
            "foret",
            "Forêt",
            true,
            &[
                ("bg", "#101814"),
                ("surface", "#16211b"),
                ("surface-2", "#1d2b23"),
                ("border", "#2a3b31"),
                ("text", "#e3efe6"),
                ("text-muted", "#93a99a"),
                ("accent", "#5cc98b"),
                ("accent-contrast", "#06140c"),
                ("spelling", "#ff6b6b"),
                ("grammar", "#f4b350"),
                ("conjugation", "#ef7fc0"),
                ("typography", "#6cb6ff"),
                ("punctuation", "#4fd1c5"),
                ("style", "#c3a6ff"),
            ],
        ),
        theme(
            "contraste",
            "Contraste élevé",
            false,
            &[
                ("bg", "#ffffff"),
                ("surface", "#ffffff"),
                ("surface-2", "#f0f0f0"),
                ("border", "#000000"),
                ("text", "#000000"),
                ("text-muted", "#333333"),
                ("accent", "#0040c1"),
                ("accent-contrast", "#ffffff"),
                ("spelling", "#c00000"),
                ("grammar", "#a04000"),
                ("conjugation", "#900070"),
                ("typography", "#0040c1"),
                ("punctuation", "#006050"),
                ("style", "#5a2a9a"),
                ("font-text", "\"Atkinson Hyperlegible\", Verdana, sans-serif"),
                ("text-size", "1.25rem"),
                ("radius", "6px"),
            ],
        ),
    ]
}

impl Theme {
    /// Rejects ids and values that could break out of a CSS declaration.
    pub fn validate(&self) -> Result<(), String> {
        let id_ok = !self.id.is_empty()
            && self.id.len() <= 48
            && self.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        if !id_ok {
            return Err("theme id must be 1–48 characters among a-z, 0-9, - and _".into());
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 60 {
            return Err("theme name must be 1–60 characters".into());
        }
        for (k, v) in &self.variables {
            if !THEME_VARIABLES.iter().any(|(name, _)| name == k) {
                return Err(format!("unknown theme variable « {k} »"));
            }
            let forbidden = v.chars().any(|c| matches!(c, ';' | '{' | '}' | '<' | '>' | '\\' | '\n' | '\r'))
                || v.to_ascii_lowercase().contains("url(")
                || v.to_ascii_lowercase().contains("expression(");
            if v.is_empty() || v.len() > 200 || forbidden {
                return Err(format!("invalid value for « {k} »"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_themes_are_complete_and_valid() {
        for t in builtin_themes() {
            t.validate().unwrap();
            for (name, _) in THEME_VARIABLES {
                assert!(t.variables.contains_key(*name), "{} misses {name}", t.id);
            }
        }
    }

    #[test]
    fn rejects_css_injection() {
        let mut t = builtin_themes().remove(0);
        t.id = "mine".into();
        t.variables.insert("bg".into(), "red; } body { display:none".into());
        assert!(t.validate().is_err());
        t.variables.insert("bg".into(), "url(https://evil)".into());
        assert!(t.validate().is_err());
    }
}
