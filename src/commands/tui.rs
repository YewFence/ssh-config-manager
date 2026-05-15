use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::config::{self, SshConfig, SshHost};

use super::host_builder::{HostFlags, prompt_host};

pub fn run(config_path: &Path) -> Result<()> {
    let mut app = TuiApp::load(config_path.to_path_buf())?;
    let mut terminal = ratatui::try_init()?;
    let result = run_app(&mut terminal, &mut app);
    ratatui::try_restore()?;
    result
}

fn run_app(terminal: &mut DefaultTerminal, app: &mut TuiApp) -> Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.delete_pending.is_some() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete()?,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete(),
                _ => {}
            }
            continue;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
            KeyCode::Char('r') => app.reload()?,
            KeyCode::Char('n') => run_prompt_action(terminal, app, PromptAction::Create)?,
            KeyCode::Char('e') => run_prompt_action(terminal, app, PromptAction::Edit)?,
            KeyCode::Char('d') => app.request_delete(),
            _ => {}
        }
    }

    Ok(())
}

fn run_prompt_action(
    terminal: &mut DefaultTerminal,
    app: &mut TuiApp,
    action: PromptAction,
) -> Result<()> {
    ratatui::try_restore()?;
    let result = match action {
        PromptAction::Create => app.create_host(),
        PromptAction::Edit => app.edit_selected_host(),
    };
    *terminal = ratatui::try_init()?;
    result
}

#[derive(Clone, Copy)]
enum PromptAction {
    Create,
    Edit,
}

#[derive(Debug)]
struct TuiApp {
    config_path: PathBuf,
    config: SshConfig,
    selected: usize,
    status: String,
    delete_pending: Option<String>,
}

impl TuiApp {
    fn load(config_path: PathBuf) -> Result<Self> {
        let config = config::load_config(&config_path)?;
        Ok(Self {
            config_path,
            config,
            selected: 0,
            status: "Ready".to_string(),
            delete_pending: None,
        })
    }

    fn selected_host(&self) -> Option<&SshHost> {
        self.config.hosts.get(self.selected)
    }

    fn select_next(&mut self) {
        if self.config.hosts.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected + 1).min(self.config.hosts.len() - 1);
    }

    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn reload(&mut self) -> Result<()> {
        let selected_alias = self.selected_host().map(|host| host.alias.clone());
        self.config = config::load_config(&self.config_path)?;
        self.selected = selected_alias
            .and_then(|alias| {
                self.config
                    .hosts
                    .iter()
                    .position(|host| host.alias == alias)
            })
            .unwrap_or(0);
        self.clamp_selection();
        self.delete_pending = None;
        self.status = "Reloaded ~/.ssh/config".to_string();
        Ok(())
    }

    fn create_host(&mut self) -> Result<()> {
        let host = prompt_host(None, empty_flags(), None, true)?;

        if self.config.contains(&host.alias) {
            anyhow::bail!(
                "Host '{}' already exists. Use `sshm edit {}` to modify it.",
                host.alias,
                host.alias
            );
        }

        let alias = host.alias.clone();
        self.config.hosts.push(host);
        config::save_config(&self.config, &self.config_path)?;
        self.selected = self.config.hosts.len().saturating_sub(1);
        self.status = format!("Host '{}' added.", alias);
        self.delete_pending = None;
        Ok(())
    }

    fn edit_selected_host(&mut self) -> Result<()> {
        let Some(original) = self.selected_host().cloned() else {
            self.status = "No host selected.".to_string();
            return Ok(());
        };

        let updated = prompt_host(
            Some(original.alias.clone()),
            empty_flags(),
            Some(&original),
            true,
        )?;
        let alias = updated.alias.clone();

        if alias != original.alias && self.config.contains(&alias) {
            anyhow::bail!(
                "Host '{}' already exists. Choose another alias or edit that host instead.",
                alias
            );
        }

        if let Some(host) = self.config.find_mut(&original.alias) {
            *host = updated;
        }

        config::save_config(&self.config, &self.config_path)?;
        if let Some(index) = self
            .config
            .hosts
            .iter()
            .position(|host| host.alias == alias)
        {
            self.selected = index;
        }
        self.status = format!("Host '{}' updated.", alias);
        self.delete_pending = None;
        Ok(())
    }

    fn request_delete(&mut self) {
        let Some(alias) = self.selected_host().map(|host| host.alias.clone()) else {
            self.status = "No host selected.".to_string();
            return;
        };

        self.delete_pending = Some(alias.clone());
        self.status = format!("Delete host '{}'? Press y to confirm, n to cancel.", alias);
    }

    fn confirm_delete(&mut self) -> Result<()> {
        let Some(alias) = self.delete_pending.take() else {
            return Ok(());
        };

        if let Some(index) = self
            .config
            .hosts
            .iter()
            .position(|host| host.alias == alias)
        {
            self.config.hosts.remove(index);
            config::save_config(&self.config, &self.config_path)?;
            self.selected = index;
            self.clamp_selection();
            self.status = format!("Host '{}' deleted.", alias);
        } else {
            self.status = format!("Host '{}' no longer exists.", alias);
            self.clamp_selection();
        }

        Ok(())
    }

    fn cancel_delete(&mut self) {
        self.delete_pending = None;
        self.status = "Delete cancelled.".to_string();
    }

    fn clamp_selection(&mut self) {
        if self.config.hosts.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.config.hosts.len() {
            self.selected = self.config.hosts.len() - 1;
        }
    }
}

fn empty_flags() -> HostFlags {
    HostFlags {
        hostname: None,
        user: None,
        port: None,
        identity_file: None,
        proxy_jump: None,
        description: None,
    }
}

fn render(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let [body, footer] = Layout::vertical([Constraint::Min(8), Constraint::Length(2)]).areas(area);
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(36), Constraint::Percentage(64)]).areas(body);

    render_host_list(frame, app, list_area);
    render_details(frame, app, detail_area);
    render_footer(frame, app, footer);
}

fn render_host_list(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let items = if app.config.hosts.is_empty() {
        vec![ListItem::new(Line::from("No hosts configured"))]
    } else {
        app.config
            .hosts
            .iter()
            .map(|host| {
                let hostname = host.hostname.as_deref().unwrap_or("-");
                ListItem::new(vec![
                    Line::from(Span::styled(
                        host.alias.as_str(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(hostname, Style::default().fg(Color::DarkGray))),
                ])
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::new()
                .title(" Hosts ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ");

    let mut state = ListState::default();
    if !app.config.hosts.is_empty() {
        state.select(Some(app.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let content = match app.selected_host() {
        Some(host) => detail_lines(host),
        None => vec![
            Line::from("No host selected."),
            Line::from("Press n to create a host."),
        ],
    };

    let title = app
        .selected_host()
        .map(|host| format!(" {} ", host.alias))
        .unwrap_or_else(|| " Details ".to_string());

    let details = Paragraph::new(content)
        .block(
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(details, area);
}

fn render_footer(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let [help_area, status_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    let help = Paragraph::new("q quit  ↑/k up  ↓/j down  n new  e edit  d delete  r reload")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    let status_style = if app.delete_pending.is_some() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };
    let status = Paragraph::new(app.status.as_str())
        .style(status_style)
        .alignment(Alignment::Center);

    frame.render_widget(help, help_area);
    frame.render_widget(status, status_area);
}

fn detail_lines(host: &SshHost) -> Vec<Line<'static>> {
    let mut lines = vec![
        field_line("Alias", &host.alias),
        field_line("Description", host.description.as_deref().unwrap_or("-")),
        field_line("HostName", host.hostname.as_deref().unwrap_or("-")),
        field_line("User", host.user.as_deref().unwrap_or("-")),
        field_line(
            "Port",
            &host
                .port
                .map(|port| port.to_string())
                .unwrap_or_else(|| "22".to_string()),
        ),
        field_line("IdentityFile", host.identity_file.as_deref().unwrap_or("-")),
        field_line("ProxyJump", host.proxy_jump.as_deref().unwrap_or("-")),
        field_line("ForwardAgent", host.forward_agent.as_deref().unwrap_or("-")),
    ];

    push_list_lines(&mut lines, "LocalForward", &host.local_forwards);
    push_list_lines(&mut lines, "RemoteForward", &host.remote_forwards);
    push_list_lines(&mut lines, "SetEnv", &host.set_env);
    push_list_lines(&mut lines, "SendEnv", &host.send_env);
    lines.push(field_line(
        "Extra directives",
        &host.extra.len().to_string(),
    ));
    lines
}

fn field_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<17}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_string()),
    ])
}

fn push_list_lines(lines: &mut Vec<Line<'static>>, label: &str, values: &[String]) {
    if values.is_empty() {
        lines.push(field_line(label, "-"));
        return;
    }

    for (index, value) in values.iter().enumerate() {
        let item_label = if index == 0 { label } else { "" };
        lines.push(field_line(item_label, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(alias: &str) -> SshHost {
        SshHost {
            alias: alias.to_string(),
            hostname: Some(format!("{alias}.example.com")),
            ..Default::default()
        }
    }

    #[test]
    fn selection_stays_in_bounds_for_empty_and_non_empty_hosts() {
        let mut app = TuiApp {
            config_path: PathBuf::from("config"),
            config: SshConfig::default(),
            selected: 0,
            status: String::new(),
            delete_pending: None,
        };

        app.select_next();
        app.select_previous();
        assert_eq!(app.selected, 0);

        app.config.hosts = vec![host("first"), host("second")];
        app.select_next();
        app.select_next();
        assert_eq!(app.selected, 1);
        app.select_previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn clamp_selection_moves_to_last_valid_host() {
        let mut app = TuiApp {
            config_path: PathBuf::from("config"),
            config: SshConfig {
                hosts: vec![host("only")],
                header_comments: vec![],
            },
            selected: 3,
            status: String::new(),
            delete_pending: None,
        };

        app.clamp_selection();
        assert_eq!(app.selected, 0);

        app.config.hosts.clear();
        app.clamp_selection();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn detail_lines_show_defaults_and_collection_fields() {
        let host = SshHost {
            alias: "demo".to_string(),
            local_forwards: vec!["8080:localhost:80".to_string()],
            set_env: vec!["APP_ENV=prod".to_string()],
            extra: vec![("StrictHostKeyChecking".to_string(), "no".to_string())],
            ..Default::default()
        };

        let rendered = detail_lines(&host)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Port             22"));
        assert!(rendered.contains("HostName         -"));
        assert!(rendered.contains("LocalForward     8080:localhost:80"));
        assert!(rendered.contains("SetEnv           APP_ENV=prod"));
        assert!(rendered.contains("Extra directives 1"));
    }
}
