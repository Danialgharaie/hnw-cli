use chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Tabs, Wrap,
    },
};

use crate::{
    app::{App, Prompt, Section},
    model::Analytics,
    theme,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Block::default().style(theme::base()), frame.area());
    let root = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .split(frame.area());

    draw_header(frame, app, root[0]);
    match app.section {
        Section::Sites => draw_sites(frame, app, root[1]),
        Section::Drives => draw_drives(frame, app, root[1]),
        Section::Account => draw_account(frame, app, root[1]),
    }
    draw_status(frame, app, root[2]);

    if app.show_help {
        draw_help(frame);
    }
    if let Some(prompt) = &app.prompt {
        draw_prompt(frame, prompt, &app.input);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::horizontal([Constraint::Length(17), Constraint::Min(20)]).split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("hnw", theme::accent()),
            Span::styled("  / here.now", theme::muted()),
        ]))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(rule()),
        ),
        layout[0],
    );
    let selected = Section::ALL
        .iter()
        .position(|section| *section == app.section)
        .unwrap_or_default();
    let tabs = Tabs::new(Section::ALL.map(Section::label))
        .select(selected)
        .style(theme::muted())
        .highlight_style(theme::accent())
        .divider("  ")
        .padding(" ", " ")
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(rule()),
        );
    frame.render_widget(tabs, layout[1]);
}

fn draw_sites(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.width < 86 {
        let rows =
            Layout::vertical([Constraint::Percentage(54), Constraint::Percentage(46)]).split(area);
        draw_site_table(frame, app, rows[0]);
        draw_site_detail(frame, app, rows[1]);
    } else {
        let columns = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);
        draw_site_table(frame, app, columns[0]);
        draw_site_detail(frame, app, columns[1]);
    }
}

fn draw_site_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.sites.iter().map(|site| {
        let status_style = if site.status.as_deref() == Some("active") {
            Style::default().fg(theme::SUCCESS).bg(theme::PAPER)
        } else {
            theme::muted()
        };
        Row::new([
            Cell::from(site.label().to_owned()),
            Cell::from(site.status.as_deref().unwrap_or("—").to_owned()).style(status_style),
            Cell::from(relative_time(site.updated_at.as_deref())),
        ])
    });
    let title = format!(" Sites  {} ", app.sites.len());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(58),
            Constraint::Length(11),
            Constraint::Length(13),
        ],
    )
    .header(
        Row::new(["NAME / SLUG", "STATUS", "UPDATED"])
            .style(theme::muted().add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .row_highlight_style(theme::selected())
    .highlight_symbol("▸ ")
    .block(panel(&title));
    frame.render_stateful_widget(table, area, &mut app.site_state);
}

fn draw_site_detail(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(analytics) = &app.analytics {
        draw_analytics(frame, analytics, area);
        return;
    }
    let site = app
        .site_detail
        .as_ref()
        .filter(|detail| {
            app.selected_site()
                .is_some_and(|site| site.slug == detail.slug)
        })
        .or_else(|| app.selected_site());
    let Some(site) = site else {
        frame.render_widget(
            empty_panel("No Sites", "Publish something, then press r."),
            area,
        );
        return;
    };

    let detail_height = if area.height > 18 { 10 } else { 7 };
    let layout =
        Layout::vertical([Constraint::Length(detail_height), Constraint::Min(3)]).split(area);
    let mut lines = vec![
        Line::styled(site.label(), theme::accent()),
        Line::styled(&site.slug, theme::muted()),
        Line::raw(&site.site_url),
        Line::raw(format!(
            "status  {}",
            site.status.as_deref().unwrap_or("unknown")
        )),
    ];
    if let Some(description) = &site.display_description {
        lines.push(Line::raw(description));
    }
    if let Some(version) = &site.current_version_id {
        lines.push(Line::styled(format!("version  {version}"), theme::muted()));
    }
    if let Some(pending) = &site.pending_version_id {
        lines.push(Line::styled(format!("pending  {pending}"), warning()));
    }
    if let Some(expires) = &site.expires_at {
        lines.push(Line::styled(format!("expires  {expires}"), warning()));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" Detail ")),
        layout[0],
    );

    let files: Vec<ListItem> = site
        .manifest
        .iter()
        .map(|file| {
            ListItem::new(Line::from(vec![
                Span::raw(&file.path),
                Span::styled(format!("  {}", human_size(file.size)), theme::muted()),
                Span::styled(
                    file.content_type
                        .as_deref()
                        .map(|kind| format!("  {kind}"))
                        .unwrap_or_default(),
                    theme::muted(),
                ),
            ]))
        })
        .collect();
    let file_title = if site.manifest.is_empty() {
        " Files · Enter to load ".to_owned()
    } else {
        format!(" Files  {} ", site.manifest.len())
    };
    frame.render_widget(List::new(files).block(panel(&file_title)), layout[1]);
}

fn draw_analytics(frame: &mut Frame, analytics: &Analytics, area: Rect) {
    let totals = &analytics.totals;
    let stats = format!(
        "{} views · {} visitors · {} all time",
        totals.range_views.unwrap_or_default(),
        totals.range_visitors.unwrap_or_default(),
        totals.all_time_views.unwrap_or_default()
    );
    let mut lines = vec![
        Line::styled(format!("{} analytics", analytics.range), theme::accent()),
        Line::raw(stats),
    ];
    if let Some(last) = &analytics.last_event_at {
        lines.push(Line::styled(format!("last event  {last}"), theme::muted()));
    }
    if !analytics.top_paths.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled("Top paths", theme::muted()));
        for item in analytics.top_paths.iter().take(8) {
            lines.push(Line::raw(format!(
                "{:>7}  {}",
                item.views.unwrap_or_default(),
                item.path.as_deref().unwrap_or("/")
            )));
        }
    }
    if !analytics.top_referrers.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled("Top referrers", theme::muted()));
        for item in analytics.top_referrers.iter().take(4) {
            lines.push(Line::raw(format!(
                "{:>7}  {}",
                item.views.unwrap_or_default(),
                item.referrer.as_deref().unwrap_or("Direct")
            )));
        }
    }
    if !analytics.top_countries.is_empty() {
        let countries = analytics
            .top_countries
            .iter()
            .take(5)
            .map(|item| {
                format!(
                    "{} {}",
                    item.country.as_deref().unwrap_or("—"),
                    item.views.unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        lines.push(Line::raw(""));
        lines.push(Line::styled(countries, theme::muted()));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" Analytics · Enter for files ")),
        area,
    );
}

fn draw_drives(frame: &mut Frame, app: &mut App, area: Rect) {
    let columns = if area.width < 74 {
        Layout::vertical([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area)
    } else {
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(area)
    };
    let rows = app.drives.iter().map(|drive| {
        Row::new([
            drive.name.clone(),
            drive.status.clone(),
            if drive.is_default { "default" } else { "" }.into(),
        ])
    });
    let drives_title = format!(" Drives  {} ", app.drives.len());
    let table = Table::new(
        rows,
        [
            Constraint::Min(12),
            Constraint::Length(9),
            Constraint::Length(9),
        ],
    )
    .header(Row::new(["DRIVE", "STATUS", "KIND"]).style(theme::muted()))
    .row_highlight_style(theme::selected())
    .highlight_symbol("▸ ")
    .block(panel(&drives_title));
    frame.render_stateful_widget(table, columns[0], &mut app.drive_state);

    let files: Vec<ListItem> = app
        .drive_files
        .iter()
        .map(|file| {
            let attribution = file
                .last_modified_by
                .as_deref()
                .map(|actor| format!(" · {actor}"))
                .unwrap_or_default();
            ListItem::new(vec![
                Line::from(vec![
                    Span::raw(&file.path),
                    Span::styled(
                        format!("  {}", human_size(file.size.unwrap_or_default())),
                        theme::muted(),
                    ),
                ]),
                Line::styled(
                    format!(
                        "{} · {} · {}{}",
                        file.content_type.as_deref().unwrap_or("file"),
                        relative_time(file.updated_at.as_deref()),
                        file.last_operation.as_deref().unwrap_or("updated"),
                        attribution
                    ),
                    theme::muted(),
                ),
                Line::styled(
                    file.etag
                        .as_deref()
                        .map(|etag| format!("etag {etag}"))
                        .unwrap_or_default(),
                    theme::muted(),
                ),
            ])
        })
        .collect();
    let message = if app.drive_files.is_empty() {
        vec![ListItem::new(Line::styled(
            "No files. Enter loads the selected Drive.",
            theme::muted(),
        ))]
    } else {
        files
    };
    let right = Layout::vertical([Constraint::Length(6), Constraint::Min(3)]).split(columns[1]);
    let drive_lines = app.selected_drive().map_or_else(Vec::new, |drive| {
        vec![
            Line::styled(&drive.name, theme::accent()),
            Line::raw(drive.description.as_deref().unwrap_or("No description")),
            Line::styled(
                format!(
                    "head {} · updated {}",
                    drive.head_version_id.as_deref().unwrap_or("—"),
                    relative_time(drive.updated_at.as_deref())
                ),
                theme::muted(),
            ),
        ]
    });
    frame.render_widget(
        Paragraph::new(drive_lines).block(panel(" Detail ")),
        right[0],
    );
    frame.render_widget(
        List::new(message).block(panel(&format!(" Files  {} ", app.drive_files.len()))),
        right[1],
    );
}

fn draw_account(frame: &mut Frame, app: &App, area: Rect) {
    let Some(profile) = &app.profile else {
        frame.render_widget(empty_panel("Account", "Profile unavailable."), area);
        return;
    };
    let content = vec![
        Line::styled(format!("@{}", profile.username), theme::accent()),
        Line::raw(""),
        labelled(
            "Profile",
            if profile.enabled {
                "public"
            } else {
                "disabled"
            },
        ),
        labelled(
            "New Sites",
            if profile.add_new_sites_to_profile {
                "added automatically"
            } else {
                "private by default"
            },
        ),
        labelled("URL", &profile.url),
        labelled("Feed", &profile.feed_url),
        Line::raw(""),
        Line::styled("Enter or o opens your profile.", theme::muted()),
    ];
    let inner = centered(area, 72, 16);
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(panel(" Account ")),
        inner,
    );
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let message = app.error.as_deref().unwrap_or(&app.status);
    let style = if app.error.is_some() {
        Style::default().fg(theme::DANGER).bg(theme::PANEL)
    } else {
        Style::default().fg(theme::MUTED).bg(theme::PANEL)
    };
    let shortcuts = match app.section {
        Section::Sites => {
            " / search  Enter inspect  e rename  d duplicate  x delete  a stats  ? help "
        }
        Section::Drives => " Enter files  o dashboard  r refresh  ? help ",
        Section::Account => " Enter open  r refresh  ? help ",
    };
    let columns = Layout::horizontal([
        Constraint::Min(10),
        Constraint::Length(shortcuts.len() as u16),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(format!(" {message}")).style(style),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(shortcuts)
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme::MUTED).bg(theme::PANEL)),
        columns[1],
    );
}

fn draw_help(frame: &mut Frame) {
    let area = centered(frame.area(), 66, 23);
    frame.render_widget(Clear, area);
    let help = Text::from(vec![
        Line::styled("Keyboard", theme::accent()),
        Line::raw(""),
        key_line("Tab / h l", "switch section"),
        key_line("j k / ↑ ↓", "move selection"),
        key_line("Enter", "inspect Site or Drive"),
        key_line("/", "search Sites"),
        key_line("e", "rename selected Site"),
        key_line("d", "duplicate selected Site"),
        key_line("x", "delete selected Site (confirmed)"),
        key_line("a", "load 30-day Site analytics"),
        key_line("o", "open selected resource"),
        key_line("r", "refresh and clear search"),
        key_line("g / G", "jump to first / last"),
        key_line("?", "toggle this help"),
        key_line("q", "quit"),
        Line::raw(""),
        Line::styled("Press any key to close.", theme::muted()),
    ]);
    frame.render_widget(
        Paragraph::new(help)
            .block(panel(" hnw help ").border_type(BorderType::Double))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_prompt(frame: &mut Frame, prompt: &Prompt, input: &str) {
    let (title, text, style) = match prompt {
        Prompt::Search => (" Search Sites ", format!("/ {input}"), theme::accent()),
        Prompt::EditName => (" Rename Site ", input.to_owned(), theme::accent()),
        Prompt::ConfirmDelete { slug } => (
            " Permanently delete Site? ",
            format!(
                "{slug}\n\nThis deletes all stored files. Press y to delete or n/Esc to cancel."
            ),
            Style::default().fg(theme::DANGER).bg(theme::PAPER),
        ),
    };
    let height = if matches!(prompt, Prompt::ConfirmDelete { .. }) {
        9
    } else {
        5
    };
    let area = centered(frame.area(), 68, height);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .style(style)
            .wrap(Wrap { trim: true })
            .block(panel(title).border_type(BorderType::Double)),
        area,
    );
    if !matches!(prompt, Prompt::ConfirmDelete { .. }) {
        let cursor_x =
            area.x + 2 + input.chars().count() as u16 + u16::from(*prompt == Prompt::Search);
        frame.set_cursor_position((cursor_x.min(area.right().saturating_sub(2)), area.y + 2));
    }
}

fn panel<'a>(title: &'a str) -> Block<'a> {
    Block::default()
        .title(title)
        .title_style(theme::muted())
        .borders(Borders::ALL)
        .border_style(rule())
        .style(theme::base())
}

fn empty_panel<'a>(title: &'a str, message: &'a str) -> Paragraph<'a> {
    Paragraph::new(message)
        .style(theme::muted())
        .alignment(Alignment::Center)
        .block(panel(title))
}

fn rule() -> Style {
    Style::default().fg(theme::RULE).bg(theme::PAPER)
}

fn warning() -> Style {
    Style::default().fg(theme::WARNING).bg(theme::PAPER)
}

fn labelled<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<12}"), theme::muted()),
        Span::raw(value),
    ])
}

fn key_line<'a>(key: &'a str, description: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{key:<14}"), theme::accent()),
        Span::raw(description),
    ])
}

fn centered(area: Rect, max_width: u16, height: u16) -> Rect {
    let width = area.width.saturating_sub(4).min(max_width);
    let height = area.height.saturating_sub(2).min(height);
    let vertical = Layout::new(
        Direction::Vertical,
        [
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ],
    )
    .split(area);
    Layout::new(
        Direction::Horizontal,
        [
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ],
    )
    .split(vertical[1])[1]
}

fn relative_time(timestamp: Option<&str>) -> String {
    let Some(timestamp) = timestamp else {
        return "—".into();
    };
    let Ok(updated) = DateTime::parse_from_rfc3339(timestamp) else {
        return timestamp.chars().take(10).collect();
    };
    let elapsed = Utc::now().signed_duration_since(updated.with_timezone(&Utc));
    if elapsed.num_minutes() < 1 {
        "now".into()
    } else if elapsed.num_hours() < 1 {
        format!("{}m ago", elapsed.num_minutes())
    } else if elapsed.num_days() < 1 {
        format!("{}h ago", elapsed.num_hours())
    } else if elapsed.num_days() < 30 {
        format!("{}d ago", elapsed.num_days())
    } else {
        updated.format("%Y-%m-%d").to_string()
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1000.0 && unit < UNITS.len() - 1 {
        size /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(human_size(999), "999 B");
        assert_eq!(human_size(1_500), "1.5 KB");
    }

    #[test]
    fn malformed_timestamp_is_safe() {
        assert_eq!(relative_time(Some("not-a-date")), "not-a-date");
    }
}
