use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::SshHost;

use super::fields::EditableField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FieldEditor {
    pub(super) field: Option<EditableField>,
    pub(super) value: String,
    pub(super) cursor: usize,
    pub(super) error: Option<String>,
}

impl FieldEditor {
    pub(super) fn new_create() -> Self {
        Self {
            field: None,
            value: String::new(),
            cursor: 0,
            error: None,
        }
    }

    pub(super) fn new_field(field: EditableField, host: &SshHost) -> Self {
        let value = field.edit_value(host);
        let cursor = value.chars().count();
        Self {
            field: Some(field),
            value,
            cursor,
            error: None,
        }
    }

    pub(super) fn title(&self) -> String {
        match self.field {
            Some(field) => format!("Edit {}", field.label()),
            None => "New host".to_string(),
        }
    }

    pub(super) fn label(&self) -> &'static str {
        self.field.map(EditableField::label).unwrap_or("Alias")
    }

    pub(super) fn example(&self) -> &'static str {
        self.field
            .map(EditableField::example)
            .unwrap_or("example: prod-api")
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> EditorAction {
        if is_save_key(key) {
            return EditorAction::Submit;
        }

        match key.code {
            KeyCode::Esc => EditorAction::Cancel,
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.insert_char(ch);
                EditorAction::Continue
            }
            KeyCode::Backspace => {
                self.backspace();
                EditorAction::Continue
            }
            KeyCode::Delete => {
                self.delete();
                EditorAction::Continue
            }
            KeyCode::Left => {
                self.move_left();
                EditorAction::Continue
            }
            KeyCode::Right => {
                self.move_right();
                EditorAction::Continue
            }
            KeyCode::Home => {
                self.cursor = 0;
                EditorAction::Continue
            }
            KeyCode::End => {
                self.cursor = self.value.chars().count();
                EditorAction::Continue
            }
            _ => EditorAction::Continue,
        }
    }

    fn insert_char(&mut self, ch: char) {
        let byte_index = byte_index_for_char(&self.value, self.cursor);
        self.value.insert(byte_index, ch);
        self.cursor += 1;
        self.error = None;
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let start = byte_index_for_char(&self.value, self.cursor - 1);
        let end = byte_index_for_char(&self.value, self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor -= 1;
        self.error = None;
    }

    fn delete(&mut self) {
        if self.cursor >= self.value.chars().count() {
            return;
        }

        let start = byte_index_for_char(&self.value, self.cursor);
        let end = byte_index_for_char(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
        self.error = None;
    }

    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.value.chars().count());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorAction {
    Continue,
    Submit,
    Cancel,
}

fn is_save_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn byte_index_for_char(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_supports_utf8_cursor_edits() {
        let mut editor = FieldEditor {
            field: None,
            value: "云枫".to_string(),
            cursor: 1,
            error: None,
        };

        editor.insert_char('晴');
        assert_eq!(editor.value, "云晴枫");
        editor.backspace();
        assert_eq!(editor.value, "云枫");
        editor.delete();
        assert_eq!(editor.value, "云");
    }

    #[test]
    fn save_key_uses_ctrl_s() {
        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(is_save_key(key));

        let plain = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert!(!is_save_key(plain));
    }
}
