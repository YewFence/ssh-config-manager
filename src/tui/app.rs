use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::core::{
    config::{self, SshConfig, SshHost},
    hosts,
};

use super::{
    editor::{EditorAction, FieldEditor, TextAreaEditor},
    fields::{EDITABLE_FIELDS, EditableField},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AppSignal {
    Continue,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FocusPane {
    Hosts,
    Fields,
    TextEditor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Dialog {
    Edit(FieldEditor),
    Create(FieldEditor),
    ConfirmDelete(String),
}

#[derive(Debug)]
pub(super) struct TuiApp {
    pub(super) config_path: PathBuf,
    pub(super) config: SshConfig,
    pub(super) selected_host: usize,
    pub(super) selected_field: usize,
    pub(super) focus: FocusPane,
    pub(super) dialog: Option<Dialog>,
    pub(super) text_editor: Option<TextAreaEditor>,
    pub(super) status: String,
}

impl TuiApp {
    pub(super) fn load(config_path: PathBuf) -> Result<Self> {
        let config = config::load_config(&config_path)?;
        Ok(Self {
            config_path,
            config,
            selected_host: 0,
            selected_field: 0,
            focus: FocusPane::Hosts,
            dialog: None,
            text_editor: None,
            status: "Ready".to_string(),
        })
    }

    pub(super) fn selected_host(&self) -> Option<&SshHost> {
        self.config.hosts.get(self.selected_host)
    }

    pub(super) fn selected_field(&self) -> EditableField {
        EDITABLE_FIELDS[self.selected_field.min(EDITABLE_FIELDS.len() - 1)]
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Result<AppSignal> {
        if let Some(dialog) = self.dialog.take() {
            return self.handle_dialog_key(dialog, key);
        }

        if let Some(editor) = self.text_editor.take() {
            return self.handle_text_editor_key(editor, key);
        }

        self.handle_normal_key(key)
    }

    fn handle_dialog_key(&mut self, dialog: Dialog, key: KeyEvent) -> Result<AppSignal> {
        match dialog {
            Dialog::Edit(mut editor) => match editor.handle_key(key) {
                EditorAction::Submit => self.save_field_editor(editor)?,
                EditorAction::Cancel => {
                    self.status = "Edit cancelled.".to_string();
                }
                EditorAction::Continue => {
                    self.dialog = Some(Dialog::Edit(editor));
                }
            },
            Dialog::Create(mut editor) => match editor.handle_key(key) {
                EditorAction::Submit => self.save_new_host(editor)?,
                EditorAction::Cancel => {
                    self.status = "Create cancelled.".to_string();
                }
                EditorAction::Continue => {
                    self.dialog = Some(Dialog::Create(editor));
                }
            },
            Dialog::ConfirmDelete(alias) => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_delete(alias)?,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.status = "Delete cancelled.".to_string();
                }
                _ => {
                    self.dialog = Some(Dialog::ConfirmDelete(alias));
                }
            },
        }

        Ok(AppSignal::Continue)
    }

    fn handle_text_editor_key(
        &mut self,
        mut editor: TextAreaEditor,
        key: KeyEvent,
    ) -> Result<AppSignal> {
        match editor.handle_key(key) {
            EditorAction::Submit => self.save_text_editor(editor)?,
            EditorAction::Cancel => {
                self.focus = FocusPane::Fields;
                self.status = "Edit cancelled.".to_string();
            }
            EditorAction::Continue => {
                self.text_editor = Some(editor);
            }
        }

        Ok(AppSignal::Continue)
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<AppSignal> {
        match key.code {
            KeyCode::Char('q') => return Ok(AppSignal::Quit),
            KeyCode::Esc if self.focus == FocusPane::Fields => {
                self.focus = FocusPane::Hosts;
                self.status = "Host list focused.".to_string();
            }
            KeyCode::Esc => return Ok(AppSignal::Quit),
            KeyCode::Tab | KeyCode::BackTab => self.toggle_focus(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Enter if self.focus == FocusPane::Hosts => self.focus_fields(),
            KeyCode::Enter | KeyCode::Char('e') if self.focus == FocusPane::Fields => {
                self.edit_selected_field()
            }
            KeyCode::Char('e') => {
                self.focus_fields();
                if self.focus == FocusPane::Fields {
                    self.edit_selected_field();
                }
            }
            KeyCode::Char('n') => self.start_create(),
            KeyCode::Char('d') => self.request_delete(),
            KeyCode::Char('r') => self.reload()?,
            _ => {}
        }

        Ok(AppSignal::Continue)
    }

    fn select_next(&mut self) {
        match self.focus {
            FocusPane::Hosts => self.select_next_host(),
            FocusPane::Fields => self.select_next_field(),
            FocusPane::TextEditor => {}
        }
    }

    fn select_previous(&mut self) {
        match self.focus {
            FocusPane::Hosts => self.select_previous_host(),
            FocusPane::Fields => self.select_previous_field(),
            FocusPane::TextEditor => {}
        }
    }

    fn select_next_host(&mut self) {
        if self.config.hosts.is_empty() {
            self.selected_host = 0;
            self.focus = FocusPane::Hosts;
            return;
        }

        self.selected_host = (self.selected_host + 1).min(self.config.hosts.len() - 1);
    }

    fn select_previous_host(&mut self) {
        self.selected_host = self.selected_host.saturating_sub(1);
    }

    fn select_next_field(&mut self) {
        if self.selected_host().is_none() {
            self.focus = FocusPane::Hosts;
            return;
        }

        self.selected_field = (self.selected_field + 1).min(EDITABLE_FIELDS.len() - 1);
    }

    fn select_previous_field(&mut self) {
        self.selected_field = self.selected_field.saturating_sub(1);
    }

    fn focus_fields(&mut self) {
        if self.selected_host().is_none() {
            self.focus = FocusPane::Hosts;
            self.status = "No host selected. Press n to create one.".to_string();
            return;
        }

        self.focus = FocusPane::Fields;
        self.status = "Field list focused. Press e to edit.".to_string();
    }

    fn toggle_focus(&mut self) {
        match self.focus {
            FocusPane::Hosts => self.focus_fields(),
            FocusPane::Fields => {
                self.focus = FocusPane::Hosts;
                self.status = "Host list focused.".to_string();
            }
            FocusPane::TextEditor => {}
        }
    }

    fn start_create(&mut self) {
        self.dialog = Some(Dialog::Create(FieldEditor::new_create()));
        self.status = "New host alias.".to_string();
    }

    fn edit_selected_field(&mut self) {
        let Some(host) = self.selected_host() else {
            self.status = "No host selected. Press n to create one.".to_string();
            return;
        };

        let field = self.selected_field();
        if field.is_multivalue() {
            self.text_editor = Some(TextAreaEditor::new(field, host));
            self.focus = FocusPane::TextEditor;
            self.status = format!("Editing {}. One entry per line.", field.label());
            return;
        }

        self.dialog = Some(Dialog::Edit(FieldEditor::new_field(field, host)));
        self.status = format!("Editing {}.", field.label());
    }

    fn save_new_host(&mut self, mut editor: FieldEditor) -> Result<()> {
        let mut next_config = self.config.clone();
        let added_index = match hosts::add_empty_host(&mut next_config, &editor.value) {
            Ok(index) => index,
            Err(err) => {
                editor.error = Some(err.to_string());
                self.dialog = Some(Dialog::Create(editor));
                return Ok(());
            }
        };

        let alias = next_config.hosts[added_index].alias.clone();
        config::save_config(&next_config, &self.config_path)?;

        self.config = next_config;
        self.selected_host = added_index;
        self.selected_field = EditableField::HostName.index();
        self.focus = FocusPane::Fields;
        self.status = format!(
            "Host '{}' added. Edit fields and press Ctrl-S to save.",
            alias
        );
        Ok(())
    }

    fn save_field_editor(&mut self, mut editor: FieldEditor) -> Result<()> {
        let Some(host_index) = self.selected_host_index() else {
            self.status = "No host selected.".to_string();
            return Ok(());
        };

        let Some(field) = editor.field else {
            return Ok(());
        };

        let mut next_config = self.config.clone();
        if let Err(err) = field.apply(&mut next_config, host_index, &editor.value) {
            editor.error = Some(err.to_string());
            self.dialog = Some(Dialog::Edit(editor));
            return Ok(());
        }

        config::save_config(&next_config, &self.config_path)?;

        self.config = next_config;
        self.selected_host = host_index;
        self.selected_field = field.index();
        self.clamp_selection();

        let alias = self
            .selected_host()
            .map(|host| host.alias.clone())
            .unwrap_or_else(|| "host".to_string());
        self.status = format!("Saved {} for '{}'.", field.label(), alias);
        Ok(())
    }

    fn save_text_editor(&mut self, mut editor: TextAreaEditor) -> Result<()> {
        let Some(host_index) = self.selected_host_index() else {
            self.focus = FocusPane::Hosts;
            self.status = "No host selected.".to_string();
            return Ok(());
        };

        let mut next_config = self.config.clone();
        if let Err(err) = editor
            .field
            .apply(&mut next_config, host_index, &editor.value)
        {
            editor.error = Some(err.to_string());
            self.text_editor = Some(editor);
            return Ok(());
        }

        config::save_config(&next_config, &self.config_path)?;

        self.config = next_config;
        self.selected_host = host_index;
        self.selected_field = editor.field.index();
        self.focus = FocusPane::Fields;
        self.clamp_selection();

        let alias = self
            .selected_host()
            .map(|host| host.alias.clone())
            .unwrap_or_else(|| "host".to_string());
        self.status = format!("Saved {} for '{}'.", editor.field.label(), alias);
        Ok(())
    }

    fn request_delete(&mut self) {
        let Some(alias) = self.selected_host().map(|host| host.alias.clone()) else {
            self.status = "No host selected.".to_string();
            return;
        };

        self.dialog = Some(Dialog::ConfirmDelete(alias.clone()));
        self.status = format!("Delete host '{}'? Press y to confirm.", alias);
    }

    fn confirm_delete(&mut self, alias: String) -> Result<()> {
        let mut next_config = self.config.clone();
        let index = match hosts::find_host_index(&next_config, &alias) {
            Some(index) => index,
            None => {
                self.status = format!("Host '{}' no longer exists.", alias);
                self.clamp_selection();
                return Ok(());
            }
        };

        hosts::delete_host(&mut next_config, &alias)?;
        config::save_config(&next_config, &self.config_path)?;

        self.config = next_config;
        self.selected_host = index;
        self.clamp_selection();
        self.status = format!("Host '{}' deleted.", alias);
        Ok(())
    }

    fn reload(&mut self) -> Result<()> {
        let selected_alias = self.selected_host().map(|host| host.alias.clone());
        self.config = config::load_config(&self.config_path)?;
        self.selected_host = selected_alias
            .and_then(|alias| {
                self.config
                    .hosts
                    .iter()
                    .position(|host| host.alias == alias)
            })
            .unwrap_or(0);
        self.clamp_selection();
        self.dialog = None;
        self.text_editor = None;
        self.status = "Reloaded ~/.ssh/config".to_string();
        Ok(())
    }

    fn selected_host_index(&self) -> Option<usize> {
        (self.selected_host < self.config.hosts.len()).then_some(self.selected_host)
    }

    fn clamp_selection(&mut self) {
        if self.config.hosts.is_empty() {
            self.selected_host = 0;
            self.focus = FocusPane::Hosts;
        } else if self.selected_host >= self.config.hosts.len() {
            self.selected_host = self.config.hosts.len() - 1;
        }

        self.selected_field = self.selected_field.min(EDITABLE_FIELDS.len() - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_hosts(names: &[&str]) -> TuiApp {
        TuiApp {
            config_path: PathBuf::from("config"),
            config: SshConfig {
                hosts: names
                    .iter()
                    .map(|name| SshHost {
                        alias: (*name).to_string(),
                        ..Default::default()
                    })
                    .collect(),
                header_comments: vec![],
            },
            selected_host: 0,
            selected_field: 0,
            focus: FocusPane::Hosts,
            dialog: None,
            text_editor: None,
            status: String::new(),
        }
    }

    #[test]
    fn selection_stays_in_bounds_for_empty_and_non_empty_hosts() {
        let mut app = app_with_hosts(&[]);

        app.select_next_host();
        app.select_previous_host();
        assert_eq!(app.selected_host, 0);

        app.config.hosts = vec![
            SshHost::new("first".to_string()),
            SshHost::new("second".to_string()),
        ];
        app.select_next_host();
        app.select_next_host();
        assert_eq!(app.selected_host, 1);
        app.select_previous_host();
        assert_eq!(app.selected_host, 0);
    }

    #[test]
    fn clamp_selection_moves_to_last_valid_host() {
        let mut app = app_with_hosts(&["only"]);
        app.selected_host = 3;

        app.clamp_selection();
        assert_eq!(app.selected_host, 0);

        app.config.hosts.clear();
        app.clamp_selection();
        assert_eq!(app.selected_host, 0);
        assert_eq!(app.focus, FocusPane::Hosts);
    }

    #[test]
    fn save_new_host_writes_config_and_focuses_fields() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");
        let mut app = TuiApp::load(config_path.clone()).unwrap();
        let mut editor = FieldEditor::new_create();
        editor.value = "demo".to_string();
        editor.cursor = 4;

        app.save_new_host(editor).unwrap();

        let config = config::load_config(&config_path).unwrap();
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].alias, "demo");
        assert_eq!(app.focus, FocusPane::Fields);
        assert_eq!(app.selected_field, EditableField::HostName.index());
    }

    #[test]
    fn tab_switches_between_host_and_field_panes() {
        let mut app = app_with_hosts(&["demo"]);

        app.handle_normal_key(KeyEvent::new(
            KeyCode::Tab,
            crossterm::event::KeyModifiers::NONE,
        ))
        .unwrap();
        assert_eq!(app.focus, FocusPane::Fields);

        app.handle_normal_key(KeyEvent::new(
            KeyCode::BackTab,
            crossterm::event::KeyModifiers::SHIFT,
        ))
        .unwrap();
        assert_eq!(app.focus, FocusPane::Hosts);
    }

    #[test]
    fn save_field_editor_writes_config_immediately() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");
        config::save_config(
            &SshConfig {
                hosts: vec![SshHost::new("demo".to_string())],
                header_comments: vec![],
            },
            &config_path,
        )
        .unwrap();
        let mut app = TuiApp::load(config_path.clone()).unwrap();
        let mut editor =
            FieldEditor::new_field(EditableField::HostName, app.selected_host().unwrap());
        editor.value = "demo.example.com".to_string();
        editor.cursor = editor.value.chars().count();

        app.save_field_editor(editor).unwrap();

        let config = config::load_config(&config_path).unwrap();
        assert_eq!(
            config.hosts[0].hostname.as_deref(),
            Some("demo.example.com")
        );
    }

    #[test]
    fn edit_selected_multi_value_field_uses_left_text_editor() {
        let mut app = app_with_hosts(&["demo"]);
        app.focus = FocusPane::Fields;
        app.selected_field = EditableField::SendEnv.index();

        app.edit_selected_field();

        assert!(app.dialog.is_none());
        assert!(app.text_editor.is_some());
        assert_eq!(app.focus, FocusPane::TextEditor);
    }

    #[test]
    fn save_text_editor_writes_multi_value_field() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");
        config::save_config(
            &SshConfig {
                hosts: vec![SshHost::new("demo".to_string())],
                header_comments: vec![],
            },
            &config_path,
        )
        .unwrap();
        let mut app = TuiApp::load(config_path.clone()).unwrap();
        let mut editor = TextAreaEditor::new(EditableField::SendEnv, app.selected_host().unwrap());
        editor.value = "LANG LC_*\nTERM".to_string();
        editor.cursor = editor.value.chars().count();

        app.save_text_editor(editor).unwrap();

        let config = config::load_config(&config_path).unwrap();
        assert_eq!(config.hosts[0].send_env, vec!["LANG LC_*", "TERM"]);
        assert_eq!(app.focus, FocusPane::Fields);
    }

    #[test]
    fn confirm_delete_uses_core_delete_and_clamps_selection() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");
        config::save_config(
            &SshConfig {
                hosts: vec![
                    SshHost::new("first".to_string()),
                    SshHost::new("second".to_string()),
                ],
                header_comments: vec![],
            },
            &config_path,
        )
        .unwrap();
        let mut app = TuiApp::load(config_path.clone()).unwrap();
        app.selected_host = 1;

        app.confirm_delete("second".to_string()).unwrap();

        let config = config::load_config(&config_path).unwrap();
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].alias, "first");
        assert_eq!(app.selected_host, 0);
    }
}
