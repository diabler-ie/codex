use super::*;
use codex_protocol::mcp::CallToolResult;
use pretty_assertions::assert_eq;
use serde_json::json;

fn call(id: &str, server: &str) -> McpToolCallCell {
    McpToolCallCell::new(
        id.into(),
        McpInvocation {
            server: server.into(),
            tool: "get_page".into(),
            arguments: Some(json!({"id": id})),
        },
        false,
    )
}

fn result(id: &str) -> Result<CallToolResult, String> {
    Ok(CallToolResult {
        content: vec![json!({"type": "text", "text": format!("Result {id}")})],
        structured_content: None,
        is_error: Some(false),
        meta: None,
    })
}

fn render(cell: &impl HistoryCell, width: u16) -> String {
    cell.display_lines(width)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn counts_only_completed_calls_and_retains_server_order() {
    let mut group = McpToolCallGroupCell::default();
    for id in 1..=5 {
        group.complete(
            call(&id.to_string(), "Brusher"),
            Duration::ZERO,
            result(&id.to_string()),
        );
    }
    group.start(call("6", "Brusher"));
    assert_eq!(
        render(&group, 100),
        "• Called Brusher 5 times · Calling Brusher…"
    );
    group.complete(call("6", "Brusher"), Duration::ZERO, result("6"));
    assert_eq!(render(&group, 100), "• Called Brusher 6 times");
    group.complete(call("7", "sumologic-mcp"), Duration::ZERO, result("7"));
    group.complete(call("8", "Brusher"), Duration::ZERO, result("8"));
    assert_eq!(
        render(&group, 100),
        "• Called Brusher 7 times, called sumologic-mcp 1 time"
    );
}

#[test]
fn overlapping_calls_deduplicate_and_expand_in_start_order() {
    let mut group = McpToolCallGroupCell::default();
    group.start(call("1", "Brusher"));
    group.start(call("2", "sumologic-mcp"));
    group.start(call("3", "Brusher"));
    group.start(call("1", "Brusher"));
    assert_eq!(
        render(&group, 100),
        "• Calling Brusher… (2 in progress), Calling sumologic-mcp…"
    );
    group.complete(call("2", "sumologic-mcp"), Duration::ZERO, result("2"));
    group.complete(call("1", "Brusher"), Duration::ZERO, result("1"));
    group.complete(call("1", "Brusher"), Duration::ZERO, result("1"));
    assert_eq!(group.call_ids().collect::<Vec<_>>(), ["1", "2", "3"]);
    assert_eq!(
        render(&group, 100),
        "• Called Brusher 1 time, called sumologic-mcp 1 time · Calling Brusher…"
    );
    group.mark_failed();
    assert_eq!(
        render(&group, 100),
        "• Called Brusher 2 times, called sumologic-mcp 1 time · 1 failed"
    );
    let detail = group
        .transcript_lines(100)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(detail.find("Result 1").unwrap() < detail.find("Result 2").unwrap());
    assert!(detail.contains("interrupted"));
    assert!(
        group
            .raw_lines()
            .iter()
            .any(|line| line.to_string().contains("interrupted"))
    );
}

#[test]
fn single_call_preserves_original_rendering_and_wrapping_stays_bounded() {
    let mut group = McpToolCallGroupCell::default();
    group.complete(call("1", "Brusher"), Duration::ZERO, result("1"));
    assert_eq!(
        group.display_hyperlink_lines(50),
        group.group.calls[0].display_hyperlink_lines(50)
    );
    assert_eq!(
        group.compact_hyperlink_lines(50),
        group.group.calls[0].compact_hyperlink_lines(50)
    );
    group.complete(
        call("2", "sumologic-mcp"),
        Duration::ZERO,
        Err("Denied".into()),
    );
    for width in [1, 4, 12, 40, 80] {
        assert!(
            group
                .display_lines(width)
                .iter()
                .all(|line| line.width() <= usize::from(width))
        );
    }
    assert!(group.display_lines(0).is_empty());
    assert!(render(&group, 100).ends_with(" · 1 failed"));
}
