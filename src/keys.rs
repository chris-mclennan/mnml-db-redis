//! Keyboard chord → action mapping. v0.1 — single-line query
//! editor; results-row navigation; connection switching.

use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Action {
    Quit,
    SwitchConnection(usize),
    QueryInsert(char),
    QueryBackspace,
    QueryClear,
    RunQuery,
    ResultUp,
    ResultDown,
    ResultPageUp,
    ResultPageDown,
    ResultTop,
    ResultBottom,
    DoubleRowLimit,
}

/// Map a keypress to an Action. The query-editor + result-navigation
/// share keys — keypresses with no Ctrl/Alt routes to the editor;
/// modified keys (or `Ctrl+P/N`) navigate the result table.
pub fn handle(key: KeyEvent, app: &App) -> Option<Action> {
    let m = key.modifiers;
    let ctrl = m.contains(KeyModifiers::CONTROL);
    let alt = m.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char('c') if ctrl => Some(Action::Quit),
        KeyCode::Char('d') if ctrl && app.query.is_empty() => Some(Action::Quit),

        // Run query — Ctrl+Enter is the canonical chord; F5 is the
        // terminal-proof fallback (Ctrl+Enter doesn't arrive
        // distinctly in many terminals).
        KeyCode::Enter if ctrl => Some(Action::RunQuery),
        KeyCode::F(5) => Some(Action::RunQuery),

        // Result navigation (Ctrl-prefixed so plain text editing
        // doesn't fight it).
        KeyCode::Up if ctrl => Some(Action::ResultUp),
        KeyCode::Down if ctrl => Some(Action::ResultDown),
        KeyCode::Char('p') if ctrl => Some(Action::ResultUp),
        KeyCode::Char('n') if ctrl => Some(Action::ResultDown),
        KeyCode::PageUp => Some(Action::ResultPageUp),
        KeyCode::PageDown => Some(Action::ResultPageDown),
        KeyCode::Home if ctrl => Some(Action::ResultTop),
        KeyCode::End if ctrl => Some(Action::ResultBottom),

        // Connection switching — Alt+digit (1-9) to avoid stealing
        // digits from the query editor.
        KeyCode::Char(c @ '1'..='9') if alt => {
            Some(Action::SwitchConnection((c as u8 - b'1') as usize))
        }

        // Editor wipe.
        KeyCode::Char('u') if ctrl => Some(Action::QueryClear),

        // Bump row_limit + retry.
        KeyCode::Char('R') if !ctrl && !alt => Some(Action::DoubleRowLimit),

        // Query editor — printable + backspace.
        KeyCode::Backspace => Some(Action::QueryBackspace),
        KeyCode::Char(c) if !ctrl && !alt => Some(Action::QueryInsert(c)),
        _ => None,
    }
}

pub async fn apply(action: Action, app: &mut App) -> bool {
    match action {
        Action::Quit => return true,
        Action::SwitchConnection(i) => app.switch_connection(i),
        Action::QueryInsert(c) => app.query_insert(c),
        Action::QueryBackspace => app.query_backspace(),
        Action::QueryClear => app.query_clear(),
        Action::RunQuery => app.run_query().await,
        Action::ResultUp => app.move_result_row(-1),
        Action::ResultDown => app.move_result_row(1),
        Action::ResultPageUp => app.move_result_row(-10),
        Action::ResultPageDown => app.move_result_row(10),
        Action::ResultTop => app.move_result_row(-(i32::MAX as isize)),
        Action::ResultBottom => app.move_result_row(i32::MAX as isize),
        Action::DoubleRowLimit => app.double_row_limit(),
    }
    false
}
