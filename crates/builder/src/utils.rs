use eframe::egui::{KeyboardShortcut, Modifiers, WidgetText};

#[derive(Copy, Clone)]
pub struct Shortcut {
    cmd_or_ctrl: bool,
    shift: bool,
    alt: bool,
    key: char,
}

impl Shortcut {
    pub const fn new(key: char) -> Self {
        Self {
            cmd_or_ctrl: false,
            shift: false,
            alt: false,
            key,
        }
    }

    pub const fn cmd_or_ctrl(mut self) -> Self {
        self.cmd_or_ctrl = true;
        self
    }

    pub const fn alt(mut self) -> Self {
        self.alt = true;
        self
    }
    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }
}

impl From<Shortcut> for String {
    fn from(shortcut: Shortcut) -> String {
        let mut str = vec![];

        if shortcut.alt {
            str.push(if cfg!(target_os = "macos") {
                "⌥"
            } else {
                "Alt"
            })
        }

        if shortcut.shift {
            str.push(if cfg!(target_os = "macos") {
                "⬆"
            } else {
                "Shift"
            })
        }

        if shortcut.cmd_or_ctrl {
            str.push(if cfg!(target_os = "macos") {
                "⌘"
            } else {
                "Ctrl"
            })
        }

        let key = shortcut.key.to_uppercase().to_string();
        str.push(key.as_str());

        str.join(" ")
    }
}

impl From<Shortcut> for WidgetText {
    fn from(shortcut: Shortcut) -> Self {
        let str: String = shortcut.into();
        str.into()
    }
}

pub fn shortcut(key: char) -> Shortcut {
    Shortcut::new(key)
}
