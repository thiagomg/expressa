//! Font size (zoom for projectors) and light/dark theme, saved in
//! `~/.config/expressa-aula/prefs`.

use std::path::PathBuf;

pub const DEFAULT_FONT: u32 = 11;
const MIN_FONT: u32 = 7;
const MAX_FONT: u32 = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefs {
    pub font_size: u32,
    pub dark: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            font_size: DEFAULT_FONT,
            dark: false,
        }
    }
}

fn path() -> PathBuf {
    gtk::glib::user_config_dir().join("expressa-aula").join("prefs")
}

impl Prefs {
    pub fn parse(text: &str) -> Self {
        let mut p = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "fonte" => {
                    if let Ok(n) = v.trim().parse::<u32>() {
                        p.font_size = n.clamp(MIN_FONT, MAX_FONT);
                    }
                }
                "tema" => p.dark = v.trim() == "escuro",
                _ => {}
            }
        }
        p
    }

    pub fn to_text(&self) -> String {
        format!(
            "fonte={}\ntema={}\n",
            self.font_size,
            if self.dark { "escuro" } else { "claro" }
        )
    }

    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let p = path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, self.to_text());
    }

    /// Ctrl+= / Ctrl+- ; 0 resets.
    pub fn zoom(&mut self, step: i32) {
        self.font_size = if step == 0 {
            DEFAULT_FONT
        } else {
            (self.font_size as i32 + step).clamp(MIN_FONT as i32, MAX_FONT as i32) as u32
        };
    }
}

/// Applies the font size to every widget with the `aula-code` class (the
/// editors and the output pane).
pub struct FontCss(gtk::CssProvider);

impl FontCss {
    pub fn install() -> Self {
        let css = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
        Self(css)
    }

    pub fn set_size(&self, pt: u32) {
        self.0
            .load_from_data(&format!("textview.aula-code {{ font-size: {pt}pt; }}"));
    }
}

/// Dark or light GTK widgets.
pub fn apply_widget_theme(dark: bool) {
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(dark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_print() {
        let p = Prefs::parse("fonte=18\ntema=escuro\n");
        assert_eq!(p, Prefs { font_size: 18, dark: true });
        assert_eq!(Prefs::parse(&p.to_text()), p);
        assert_eq!(Prefs::parse("fonte=999\nlixo\n").font_size, MAX_FONT);
        assert_eq!(Prefs::parse(""), Prefs::default());
    }

    #[test]
    fn zoom_steps_and_limits() {
        let mut p = Prefs::default();
        p.zoom(2);
        assert_eq!(p.font_size, DEFAULT_FONT + 2);
        p.zoom(0);
        assert_eq!(p.font_size, DEFAULT_FONT);
        for _ in 0..50 {
            p.zoom(-1);
        }
        assert_eq!(p.font_size, MIN_FONT);
    }
}
