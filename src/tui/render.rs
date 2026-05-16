use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::{
    app::{Dialog, FocusPane, TuiApp},
    editor::{FieldEditor, TextAreaEditor},
    fields::{self, DetailRow},
};

pub(super) fn render(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let [body, footer] = Layout::vertical([Constraint::Min(8), Constraint::Length(2)]).areas(area);
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(36), Constraint::Percentage(64)]).areas(body);

    render_left_pane(frame, app, list_area);
    render_details(frame, app, detail_area);
    render_footer(frame, app, footer);
    render_dialog(frame, app, area);
}

fn render_left_pane(frame: &mut Frame, app: &TuiApp, area: Rect) {
    if let Some(editor) = app.text_editor.as_ref() {
        render_text_editor(frame, editor, area);
    } else {
        render_host_list(frame, app, area);
    }
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

    let block = Block::new()
        .title(" Hosts ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(focus_style(app.focus == FocusPane::Hosts));
    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ");

    let mut state = ListState::default();
    if !app.config.hosts.is_empty() {
        state.select(Some(app.selected_host));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let Some(host) = app.selected_host() else {
        let details = Paragraph::new(vec![
            Line::from("No host selected."),
            Line::from("Press n to create a host."),
        ])
        .block(
            Block::new()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(focus_style(
                    app.focus == FocusPane::Fields || app.text_editor.is_some(),
                )),
        );
        frame.render_widget(details, area);
        return;
    };

    let rows = fields::detail_rows(host);
    let selected_field = app.selected_field();
    let selected_row = rows
        .iter()
        .position(|row| row.field == Some(selected_field));
    let items = rows.iter().map(detail_item).collect::<Vec<_>>();

    let title = format!(" {} ", host.alias);
    let details = List::new(items)
        .block(
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(focus_style(
                    app.focus == FocusPane::Fields || app.text_editor.is_some(),
                )),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ");

    let mut state = ListState::default();
    if (app.focus == FocusPane::Fields || app.text_editor.is_some()) && app.dialog.is_none() {
        state.select(selected_row);
    }
    frame.render_stateful_widget(details, area, &mut state);
}

fn detail_item(row: &DetailRow) -> ListItem<'static> {
    let label_style = if row.field.is_some() {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let value_style = if row.field.is_some() {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    ListItem::new(Line::from(vec![
        Span::styled(format!("{:<22}", row.label), label_style),
        Span::styled(row.value.clone(), value_style),
    ]))
}

fn render_footer(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let [help_area, status_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    let help = Paragraph::new(help_text(app))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    let status_style = if matches!(app.dialog.as_ref(), Some(Dialog::ConfirmDelete(_))) {
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

fn help_text(app: &TuiApp) -> &'static str {
    match app.dialog.as_ref() {
        Some(Dialog::Edit(_)) | Some(Dialog::Create(_)) => {
            "Ctrl-S save  Esc cancel  Left/Right move  Home/End jump  Backspace/Delete edit"
        }
        Some(Dialog::ConfirmDelete(_)) => "y confirm  n/Esc cancel",
        None if app.text_editor.is_some() => {
            "Ctrl-S save  Esc cancel  Enter newline  Arrows move  Backspace/Delete edit"
        }
        None if app.focus == FocusPane::Fields => {
            "Tab/Esc hosts  Up/k Down/j field  e/Enter edit  n new  d delete  r reload  q quit"
        }
        None => "Up/k Down/j host  Tab/Enter fields  n new  d delete  r reload  q/Esc quit",
    }
}

fn render_dialog(frame: &mut Frame, app: &TuiApp, area: Rect) {
    match app.dialog.as_ref() {
        Some(Dialog::Edit(editor)) | Some(Dialog::Create(editor)) => {
            render_editor_dialog(frame, editor, area);
        }
        Some(Dialog::ConfirmDelete(alias)) => {
            render_delete_dialog(frame, alias, area);
        }
        None => {}
    }
}

fn render_editor_dialog(frame: &mut Frame, editor: &FieldEditor, area: Rect) {
    let popup = centered_rect(70, 9, area);
    let block = Block::new()
        .title(format!(" {} ", editor.title()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);

    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);

    let [label_area, input_area, example_area, error_area, help_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    frame.render_widget(
        Paragraph::new(editor.label()).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        label_area,
    );

    render_input(frame, editor, input_area);

    frame.render_widget(
        Paragraph::new(editor.example()).style(Style::default().fg(Color::DarkGray)),
        example_area,
    );

    let error = editor.error.as_deref().unwrap_or("");
    frame.render_widget(
        Paragraph::new(error).style(Style::default().fg(Color::Red)),
        error_area,
    );

    frame.render_widget(
        Paragraph::new("Ctrl-S save  Esc cancel").style(Style::default().fg(Color::DarkGray)),
        help_area,
    );
}

fn render_input(frame: &mut Frame, editor: &FieldEditor, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width.saturating_sub(1) as usize;
    let (visible, cursor_col) = visible_input(&editor.value, editor.cursor, width);
    frame.render_widget(Paragraph::new(visible).wrap(Wrap { trim: false }), inner);

    if inner.width > 0 {
        frame.set_cursor_position((inner.x + cursor_col, inner.y));
    }
}

fn render_text_editor(frame: &mut Frame, editor: &TextAreaEditor, area: Rect) {
    let block = Block::new()
        .title(format!(" {} ", editor.title()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(focus_style(true));
    let inner = block.inner(area);

    frame.render_widget(block, area);

    let [input_area, hint_area, example_area, error_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let width = input_area.width.saturating_sub(1) as usize;
    let height = input_area.height as usize;
    let (visible, cursor_col, cursor_row) =
        visible_text_area(&editor.value, editor.cursor, width, height);

    frame.render_widget(
        Paragraph::new(visible).wrap(Wrap { trim: false }),
        input_area,
    );
    frame.render_widget(
        Paragraph::new("One entry per line").style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
    frame.render_widget(
        Paragraph::new(editor.example()).style(Style::default().fg(Color::DarkGray)),
        example_area,
    );
    let error = editor.error.as_deref().unwrap_or("");
    frame.render_widget(
        Paragraph::new(error).style(Style::default().fg(Color::Red)),
        error_area,
    );

    if input_area.width > 0 && input_area.height > 0 {
        frame.set_cursor_position((input_area.x + cursor_col, input_area.y + cursor_row));
    }
}

fn render_delete_dialog(frame: &mut Frame, alias: &str, area: Rect) {
    let popup = centered_rect(54, 5, area);
    let block = Block::new()
        .title(" Delete host ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);

    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Delete host '{}'?", alias)),
            Line::from("Press y to confirm, n or Esc to cancel."),
        ])
        .alignment(Alignment::Center),
        inner,
    );
}

fn visible_input(value: &str, cursor: usize, width: usize) -> (String, u16) {
    if width == 0 {
        return (String::new(), 0);
    }

    let chars = value.chars().collect::<Vec<_>>();
    let cursor = cursor.min(chars.len());
    let start = if cursor >= width {
        cursor + 1 - width
    } else {
        0
    };
    let end = (start + width).min(chars.len());
    let visible = chars[start..end].iter().collect::<String>();
    let cursor_col = (cursor - start).min(width.saturating_sub(1)) as u16;
    (visible, cursor_col)
}

fn visible_text_area(
    value: &str,
    cursor: usize,
    width: usize,
    height: usize,
) -> (String, u16, u16) {
    if width == 0 || height == 0 {
        return (String::new(), 0, 0);
    }

    let lines = split_lines_for_editor(value);
    let (cursor_line, cursor_column) = cursor_position(value, cursor);
    let start_line = if cursor_line >= height {
        cursor_line + 1 - height
    } else {
        0
    };
    let end_line = (start_line + height).min(lines.len());
    let column_start = if cursor_column >= width {
        cursor_column + 1 - width
    } else {
        0
    };

    let visible = lines[start_line..end_line]
        .iter()
        .map(|line| visible_line(line, column_start, width))
        .collect::<Vec<_>>()
        .join("\n");
    let cursor_row = cursor_line.saturating_sub(start_line).min(height - 1) as u16;
    let cursor_col = cursor_column.saturating_sub(column_start) as u16;

    (visible, cursor_col, cursor_row)
}

fn split_lines_for_editor(value: &str) -> Vec<String> {
    if value.is_empty() {
        return vec![String::new()];
    }

    value
        .split('\n')
        .map(ToString::to_string)
        .collect::<Vec<_>>()
}

fn cursor_position(value: &str, cursor: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;

    for ch in value.chars().take(cursor.min(value.chars().count())) {
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }

    (line, column)
}

fn visible_line(value: &str, start: usize, width: usize) -> String {
    value.chars().skip(start).take(width).collect()
}

fn centered_rect(width_percent: u16, height: u16, area: Rect) -> Rect {
    let width = area.width.saturating_mul(width_percent).saturating_div(100);
    let min_width = 32.min(area.width.max(1));
    let width = width.max(min_width).min(area.width.max(1));
    let height = height.min(area.height.max(1));
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

fn focus_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_input_keeps_cursor_inside_width() {
        assert_eq!(visible_input("abcdef", 0, 4), ("abcd".to_string(), 0));
        assert_eq!(visible_input("abcdef", 6, 4), ("def".to_string(), 3));
    }

    #[test]
    fn visible_text_area_tracks_cursor_line() {
        assert_eq!(
            visible_text_area("one\ntwo\nthree", 8, 10, 2),
            ("two\nthree".to_string(), 0, 1)
        );
    }

    #[test]
    fn visible_text_area_scrolls_long_lines_to_cursor() {
        assert_eq!(
            visible_text_area("abcdef", 6, 4, 2),
            ("def".to_string(), 3, 0)
        );
    }
}
