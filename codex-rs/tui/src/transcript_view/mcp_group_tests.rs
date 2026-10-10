use super::*;
use crate::history_cell::McpInvocation;
use crate::history_cell::McpToolCallCell;
use crate::history_cell::McpToolCallGroupCell;
use crate::transcript_view::tests::render;
use crate::transcript_view::tests::text;
use codex_protocol::mcp::CallToolResult;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::time::Duration;

fn group(prefix: &str) -> McpToolCallGroupCell {
    let mut group = McpToolCallGroupCell::default();
    for index in 1..=2 {
        group.complete(
            McpToolCallCell::new(
                format!("{prefix}-{index}"),
                McpInvocation {
                    server: "Brusher".into(),
                    tool: "get_page".into(),
                    arguments: None,
                },
                false,
            ),
            Duration::ZERO,
            Ok(CallToolResult {
                content: vec![
                    serde_json::json!({"type": "text", "text": format!("{prefix} result {index}")}),
                ],
                structured_content: None,
                is_error: None,
                meta: None,
            }),
        );
    }
    group
}

#[test]
fn ctrl_y_toggles_visible_groups_and_stays_with_editor_at_latest() {
    let cells: Vec<Arc<dyn HistoryCell>> =
        vec![Arc::new(group("first")), Arc::new(group("second"))];
    let mut view = TranscriptView::default();
    let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
    let collapsed = text(&render(&mut view, &cells, 80, 30));
    assert_eq!(collapsed.matches("Called Brusher 2 times").count(), 2);
    assert!(!collapsed.contains("Show details"));
    assert!(!view.owns_tool_call_toggle(key));
    assert!(
        view.handle_key(key, &cells).is_none(),
        "composer keeps Ctrl+Y for yank"
    );
    view.handle_key(KeyCode::F(4).into(), &cells).unwrap();
    assert!(view.owns_tool_call_toggle(key));
    view.handle_key(key, &cells).unwrap();
    assert!(view.is_following());
    let expanded = text(&render(&mut view, &cells, 80, 30));
    assert!(expanded.contains("first result 1"));
    assert!(expanded.contains("second result 2"));
    view.handle_key(key, &cells).unwrap();
    let collapsed_again = text(&render(&mut view, &cells, 80, 30));
    assert!(!collapsed_again.contains("result"));
    assert_eq!(collapsed_again.matches("Called Brusher 2 times").count(), 2);
}

#[test]
fn partly_expanded_groups_expand_together_before_collapsing() {
    let cells: Vec<Arc<dyn HistoryCell>> =
        vec![Arc::new(group("first")), Arc::new(group("second"))];
    let mut view = TranscriptView::default();
    render(&mut view, &cells, 80, 30);
    view.disclosure.expanded.extend(cells[0].activity_ids());
    view.handle_key(KeyCode::F(4).into(), &cells).unwrap();
    render(&mut view, &cells, 80, 30);
    let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
    view.handle_key(key, &cells).unwrap();
    assert!(
        cells
            .iter()
            .all(|cell| view.disclosure.is_expanded(&cell.activity_ids()))
    );
    render(&mut view, &cells, 80, 30);
    view.handle_key(key, &cells).unwrap();
    assert!(
        cells
            .iter()
            .all(|cell| !view.disclosure.is_expanded(&cell.activity_ids()))
    );
}

#[test]
fn expanded_live_group_keeps_details_when_committed_to_history() {
    let group = group("live");
    let mut view = TranscriptView::default();
    view.sync_live_activity(&[], group.activity_ids());
    view.sync_live_activity_tail(80, None, false, |width| {
        Some(ActivityTranscriptLines {
            activity: group.compact_hyperlink_lines(width),
            auxiliary: Vec::new(),
            disclosure: group.activity_disclosure(width),
        })
    });
    render(&mut view, &[], 80, 30);
    view.handle_key(KeyCode::F(4).into(), &[]).unwrap();
    view.handle_key(
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        &[],
    )
    .unwrap();
    let cells: Vec<Arc<dyn HistoryCell>> = vec![Arc::new(group)];
    view.sync_live_activity(&cells, Vec::new());
    view.sync_live_tail(80, None, |_| None);
    let rendered = text(&render(&mut view, &cells, 80, 30));
    assert!(rendered.contains("live result 1"));
    assert!(!rendered.contains("Called Brusher 2 times"));
}
