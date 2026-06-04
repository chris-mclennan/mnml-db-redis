//! ratatui rendering + main event loop.

use crate::app::App;
use crate::keys;
use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Tabs},
};
use std::io::Stdout;
use std::time::Duration;

pub async fn run(app: &mut App) -> Result<()> {
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = event_loop(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && key.kind == event::KeyEventKind::Press
            && let Some(action) = keys::handle(key, app)
        {
            let quit = keys::apply(action, app).await;
            if quit {
                break;
            }
        }
    }
    Ok(())
}

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // connection strip
            Constraint::Length(5), // query editor
            Constraint::Min(1),    // results table
            Constraint::Length(1), // status line
        ])
        .split(size);
    draw_connections(f, chunks[0], app);
    draw_query_editor(f, chunks[1], app);
    draw_results(f, chunks[2], app);
    draw_status(f, chunks[3], app);
}

fn draw_connections(f: &mut Frame, area: Rect, app: &App) {
    let labels: Vec<Line> = app
        .connections
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let prefix = if c.client.is_some() { "●" } else { "○" };
            let label = format!("{} Alt+{} {}", prefix, i + 1, c.cfg.name);
            if c.last_error.is_some() {
                Line::from(Span::styled(label, Style::default().fg(Color::Red)))
            } else {
                Line::from(label)
            }
        })
        .collect();
    let tabs = Tabs::new(labels)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" connections "),
        )
        .select(app.active)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn draw_query_editor(f: &mut Frame, area: Rect, app: &App) {
    let chars: Vec<char> = app.query.chars().collect();
    let cursor = app.cursor.min(chars.len());
    let head: String = chars[..cursor].iter().collect();
    let tail: String = chars[cursor..].iter().collect();
    let line = Line::from(vec![
        Span::styled(head, Style::default().fg(Color::White)),
        Span::styled("│", Style::default().fg(Color::Cyan)),
        Span::styled(
            tail,
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
        ),
    ]);
    let hint_line = Line::from(Span::styled(
        "  Ctrl+Enter / F5 run · Ctrl+U clear · Ctrl+↑/↓ scroll results · R double row_limit",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    ));
    let body = vec![Line::from(""), line, Line::from(""), hint_line];
    let p = Paragraph::new(body).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" query @ {} ", app.active_name())),
    );
    f.render_widget(p, area);
}

fn draw_results(f: &mut Frame, area: Rect, app: &App) {
    let result = match app.last_result.as_ref() {
        Some(r) => r,
        None => {
            let p = Paragraph::new("(no results yet — run a query with Ctrl+Enter)")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title(" results "));
            f.render_widget(p, area);
            return;
        }
    };
    if result.rows.is_empty() && !result.columns.is_empty() {
        let title = format!(" results (0 rows · {}ms) ", result.elapsed.as_millis());
        let p = Paragraph::new("(query returned no rows)")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(p, area);
        return;
    }
    let header = Row::new(
        result
            .columns
            .iter()
            .map(|c| Cell::from(c.clone()))
            .collect::<Vec<_>>(),
    )
    .style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = result
        .rows
        .iter()
        .map(|cells| {
            Row::new(
                cells
                    .iter()
                    .map(|c| Cell::from(c.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    // Per-column width: a cell-width budget split evenly with a
    // minimum of 8. Single-column results get the whole area.
    let n = result.columns.len().max(1);
    let widths: Vec<Constraint> = (0..n).map(|_| Constraint::Min(8)).collect();

    let title = if result.truncated {
        format!(
            " results ({}/{} · {}ms · truncated) ",
            result.rows.len(),
            result.server_row_count,
            result.elapsed.as_millis()
        )
    } else {
        format!(
            " results ({} · {}ms) ",
            result.rows.len(),
            result.elapsed.as_millis()
        )
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    let mut state = TableState::default();
    state.select(Some(app.result_row));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let p = Paragraph::new(Line::from(format!(" {} ", app.status)))
        .style(Style::default().fg(Color::White));
    f.render_widget(p, area);
}
