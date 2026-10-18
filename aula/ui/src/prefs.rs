//! Font size (zoom for projectors) and light/dark theme, saved in
//! `~/.config/expressa-aula/prefs`.

use std::path::{Path, PathBuf};

pub const DEFAULT_FONT: u32 = 11;
const MIN_FONT: u32 = 7;
const MAX_FONT: u32 = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefs {
    pub font_size: u32,
    pub dark: bool,
    /// Last local project folder. The chooser starts there.
    pub pasta: String,
    /// Last remote server URL.
    pub servidor: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            font_size: DEFAULT_FONT,
            dark: false,
            pasta: String::new(),
            servidor: "http://127.0.0.1:50051".to_string(),
        }
    }
}

fn path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("expressa-aula")
        .join("prefs")
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
                "pasta" => p.pasta = v.trim().to_string(),
                "servidor" => p.servidor = v.trim().to_string(),
                _ => {}
            }
        }
        p
    }

    pub fn to_text(&self) -> String {
        format!(
            "fonte={}\ntema={}\npasta={}\nservidor={}\n",
            self.font_size,
            if self.dark { "escuro" } else { "claro" },
            self.pasta,
            self.servidor,
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

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// `~` and `~/…` become the user home directory. `~nome` is left as-is.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from("~"));
    }
    let rest = path
        .strip_prefix("~/")
        .or_else(|| path.strip_prefix("~\\"));
    if let Some(rest) = rest {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// `/home/ana/proj` → `~/proj` when the prefix is this user’s home.
pub fn path_for_display(path: &Path) -> String {
    let shown = path.display().to_string();
    let Some(home) = home_dir() else {
        return shown;
    };
    let home = home.canonicalize().unwrap_or(home);
    let home_s = home.display().to_string();
    if shown == home_s {
        return "~".to_string();
    }
    let sep = std::path::MAIN_SEPARATOR;
    let prefix = format!("{home_s}{sep}");
    if let Some(rest) = shown.strip_prefix(&prefix) {
        return format!("~{sep}{rest}");
    }
    if let Ok(canon) = path.canonicalize() {
        let cs = canon.display().to_string();
        if cs == home_s {
            return "~".to_string();
        }
        if let Some(rest) = cs.strip_prefix(&prefix) {
            return format!("~{sep}{rest}");
        }
    }
    shown
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

/// App-wide light/dark chrome. Does not touch `gtk-theme-name` or
/// `gtk-application-prefer-dark-theme`: on Yaru those reload a missing
/// `gtk-dark.css` gresource and leave the window unstyled, while only the
/// GtkSourceView scheme would change.
pub struct ThemeCss(gtk::CssProvider);

impl ThemeCss {
    pub fn install() -> Self {
        let css = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        Self(css)
    }

    pub fn apply(&self, dark: bool) {
        self.0.load_from_data(if dark { DARK_CSS } else { LIGHT_CSS });
    }
}

const LIGHT_CSS: &str = r#"
@define-color aula_bg #f6f5f4;
@define-color aula_fg #3d3846;
@define-color aula_view #fcfcfc;
@define-color aula_view_fg #3d3846;
@define-color aula_btn #e9e7e4;
@define-color aula_hover #ddd9d4;
@define-color aula_sel #c5e1a5;
@define-color aula_border #d3cfc8;
@define-color aula_accent #0e8420;
@define-color window_bg_color @aula_bg;
@define-color window_fg_color @aula_fg;
@define-color view_bg_color @aula_view;
@define-color view_fg_color @aula_view_fg;
@define-color headerbar_bg_color @aula_bg;
@define-color headerbar_fg_color @aula_fg;
@define-color popover_bg_color @aula_view;
@define-color popover_fg_color @aula_fg;
@define-color sidebar_bg_color @aula_bg;
@define-color sidebar_fg_color @aula_fg;
@define-color card_bg_color @aula_view;
@define-color card_fg_color @aula_fg;
@define-color dialog_bg_color @aula_bg;
@define-color dialog_fg_color @aula_fg;
@define-color theme_bg_color @aula_bg;
@define-color theme_fg_color @aula_fg;
@define-color theme_base_color @aula_view;
@define-color theme_text_color @aula_view_fg;
@define-color theme_selected_bg_color #c5e1a5;
@define-color theme_selected_fg_color #1a1a1a;
window, dialog, messagedialog, .background {
  background-color: @aula_bg;
  color: @aula_fg;
}
box, paned, notebook, stack, scrolledwindow, viewport, frame, grid, revealer {
  color: inherit;
}
textview, textview > text, treeview, treeview.view, listview, list {
  background-color: @aula_view;
  color: @aula_view_fg;
}
/* Application priority beats the theme's :selected rule, so the row
   highlight has to be set here or a click looks like it did nothing. */
treeview.view:selected,
treeview.view:selected:focus,
treeview.view:selected:hover {
  background-color: @aula_sel;
  color: #1a1a1a;
}
entry, spinbutton {
  background-color: @aula_view;
  color: @aula_view_fg;
  caret-color: @aula_view_fg;
}
button {
  background-image: none;
  background-color: @aula_btn;
  color: @aula_fg;
  border-color: @aula_border;
}
button:hover { background-color: @aula_hover; }
button:checked, togglebutton:checked { background-color: @aula_sel; }
menubar {
  background-color: @aula_bg;
  color: @aula_fg;
  box-shadow: none;
  border: none;
}
menubar button {
  background-image: none;
  background-color: transparent;
  border: none;
  box-shadow: none;
  color: @aula_fg;
}
menubar button:hover, menubar button:checked { background-color: @aula_hover; }
popover.background {
  background-color: transparent;
  background-image: none;
  border: none;
  box-shadow: none;
  padding: 0;
  min-width: 0;
}
popover > contents {
  background-color: @aula_view;
  color: @aula_fg;
  border: 1px solid @aula_border;
  border-radius: 12px;
  box-shadow: 0 2px 8px alpha(black, 0.16);
  padding: 4px;
  min-width: 0;
}
popover > arrow {
  background-color: transparent;
  border: none;
}
modelbutton {
  background: none;
  border: none;
  box-shadow: none;
  color: @aula_fg;
  min-width: 0;
  padding: 4px 8px;
}
modelbutton:hover, modelbutton:selected { background-color: @aula_hover; }
modelbutton > box { min-width: 0; }
modelbutton image { min-width: 0; padding: 0; }
popover.menu scrolledwindow { min-width: 0; min-height: 0; }
tooltip {
  background-color: @aula_view;
  color: @aula_fg;
}
notebook > header { background-color: @aula_bg; }
notebook > header tabs tab {
  background-color: @aula_btn;
  color: @aula_fg;
}
notebook > header tabs tab:checked { background-color: @aula_view; }
scrollbar slider { background-color: @aula_border; }
label { color: inherit; }
selection { background-color: @aula_sel; color: #1a1a1a; }
"#;

const DARK_CSS: &str = r#"
@define-color aula_bg #24242c;
@define-color aula_fg #e4e2ea;
@define-color aula_view #1e1e24;
@define-color aula_view_fg #e4e2ea;
@define-color aula_btn #2e2e38;
@define-color aula_hover #3a3a46;
@define-color aula_sel #3d4f74;
@define-color aula_border #3a3a44;
@define-color aula_accent #7ee08a;
@define-color window_bg_color @aula_bg;
@define-color window_fg_color @aula_fg;
@define-color view_bg_color @aula_view;
@define-color view_fg_color @aula_view_fg;
@define-color headerbar_bg_color @aula_bg;
@define-color headerbar_fg_color @aula_fg;
@define-color popover_bg_color #2a2a32;
@define-color popover_fg_color @aula_fg;
@define-color sidebar_bg_color @aula_bg;
@define-color sidebar_fg_color @aula_fg;
@define-color card_bg_color #2a2a32;
@define-color card_fg_color @aula_fg;
@define-color dialog_bg_color @aula_bg;
@define-color dialog_fg_color @aula_fg;
@define-color theme_bg_color @aula_bg;
@define-color theme_fg_color @aula_fg;
@define-color theme_base_color @aula_view;
@define-color theme_text_color @aula_view_fg;
@define-color theme_selected_bg_color #3d4f74;
@define-color theme_selected_fg_color #ffffff;
window, dialog, messagedialog, .background {
  background-color: @aula_bg;
  color: @aula_fg;
}
box, paned, notebook, stack, scrolledwindow, viewport, frame, grid, revealer {
  color: inherit;
}
textview, textview > text, treeview, treeview.view, listview, list {
  background-color: @aula_view;
  color: @aula_view_fg;
}
/* Application priority beats the theme's :selected rule, so the row
   highlight has to be set here or a click looks like it did nothing. */
treeview.view:selected,
treeview.view:selected:focus,
treeview.view:selected:hover {
  background-color: @aula_sel;
  color: #ffffff;
}
entry, spinbutton {
  background-color: @aula_view;
  color: @aula_view_fg;
  caret-color: @aula_view_fg;
}
button {
  background-image: none;
  background-color: @aula_btn;
  color: @aula_fg;
  border-color: @aula_border;
}
button:hover { background-color: @aula_hover; }
button:checked, togglebutton:checked { background-color: @aula_sel; }
menubar {
  background-color: @aula_bg;
  color: @aula_fg;
  box-shadow: none;
  border: none;
}
menubar button {
  background-image: none;
  background-color: transparent;
  border: none;
  box-shadow: none;
  color: @aula_fg;
}
menubar button:hover, menubar button:checked { background-color: @aula_hover; }
popover.background {
  background-color: transparent;
  background-image: none;
  border: none;
  box-shadow: none;
  padding: 0;
  min-width: 0;
}
popover > contents {
  background-color: #2a2a32;
  color: @aula_fg;
  border: 1px solid @aula_border;
  border-radius: 12px;
  box-shadow: 0 2px 8px alpha(black, 0.35);
  padding: 4px;
  min-width: 0;
}
popover > arrow {
  background-color: transparent;
  border: none;
}
modelbutton {
  background: none;
  border: none;
  box-shadow: none;
  color: @aula_fg;
  min-width: 0;
  padding: 4px 8px;
}
modelbutton:hover, modelbutton:selected { background-color: @aula_hover; }
modelbutton > box { min-width: 0; }
modelbutton image { min-width: 0; padding: 0; }
popover.menu scrolledwindow { min-width: 0; min-height: 0; }
tooltip {
  background-color: #2a2a32;
  color: @aula_fg;
}
notebook > header { background-color: @aula_bg; }
notebook > header tabs tab {
  background-color: @aula_btn;
  color: @aula_fg;
}
notebook > header tabs tab:checked { background-color: @aula_view; }
scrollbar slider { background-color: @aula_border; }
label { color: inherit; }
selection { background-color: @aula_sel; color: #ffffff; }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_print() {
        let p = Prefs::parse("fonte=18\ntema=escuro\n");
        let mut expect = Prefs::default();
        expect.font_size = 18;
        expect.dark = true;
        assert_eq!(p, expect);
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

    #[test]
    fn palettes_cover_window_and_editor() {
        assert!(LIGHT_CSS.contains("#fcfcfc"));
        assert!(DARK_CSS.contains("#1e1e24"));
        assert!(LIGHT_CSS.contains("window, dialog"));
        assert!(DARK_CSS.contains("window, dialog"));
    }

    #[test]
    fn tilde_becomes_home() {
        let home = std::env::temp_dir().join("expressa-aula-tilde-home");
        unsafe { std::env::set_var("HOME", &home) };
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/proj/a"), home.join("proj/a"));
        assert_eq!(expand_tilde("/abs"), PathBuf::from("/abs"));
        assert_eq!(expand_tilde("~nao"), PathBuf::from("~nao"));
        let proj = home.join("proj");
        assert_eq!(path_for_display(&proj), format!("~/proj"));
    }
}
