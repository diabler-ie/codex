use super::*;

#[tokio::test]
async fn image_result_live_and_replay_render_in_the_owning_call() {
    let item: AppServerThreadItem = serde_json::from_value(json!({
        "type": "mcpToolCall", "id": "image", "server": "node_repl", "tool": "js",
        "status": "completed", "arguments": {"title": "Inspect screenshot", "code": "showImage()"},
        "result": {"content": [
            {"type": "text", "text": "Screenshot captured"},
            {"type": "image", "mimeType": "image/png", "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=="}
        ]},
        "durationMs": 5
    })).expect("MCP image result");
    let mut outputs = Vec::new();
    for replay in [false, true] {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        if replay {
            chat.replay_thread_item(item.clone(), "turn-1".into(), ReplayKind::ThreadSnapshot);
        } else {
            let mut started = item.clone();
            if let AppServerThreadItem::McpToolCall { status, result, .. } = &mut started {
                *status = codex_app_server_protocol::McpToolCallStatus::InProgress;
                *result = None;
            }
            chat.on_mcp_tool_call_started(started);
            chat.on_mcp_tool_call_completed(item.clone());
        }
        chat.flush_active_cell();
        let cells = drain_insert_history(&mut rx);
        assert_eq!(cells.len(), 1);
        outputs.push(lines_to_single_string(&cells[0]));
    }
    assert_eq!(outputs[0], outputs[1]);
    insta::assert_snapshot!(outputs[0]);
}

fn grouped_item(id: &str, server: &str, status: &str) -> AppServerThreadItem {
    serde_json::from_value(json!({
        "type": "mcpToolCall", "id": id, "server": server, "tool": "get_page",
        "status": status, "arguments": {"id": id},
        "result": {"content": [{"type": "text", "text": format!("Full result {id}")}]},
        "durationMs": 5
    }))
    .expect("MCP fixture")
}

#[tokio::test]
async fn mixed_mcp_calls_match_live_replay_and_history_projection() {
    let mut outputs = Vec::new();
    for replay in [false, true] {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(None).await;
        let mut items = Vec::new();
        for (id, server) in [("1", "Brusher"), ("2", "sumologic-mcp"), ("3", "Brusher")] {
            let item = grouped_item(id, server, "completed");
            items.push(item.clone());
            if replay {
                chat.replay_thread_item(item, "turn".into(), ReplayKind::ThreadSnapshot);
            } else {
                chat.on_mcp_tool_call_started(grouped_item(id, server, "inProgress"));
                chat.on_mcp_tool_call_completed(item);
            }
            if id == "1" {
                let reasoning = AppServerThreadItem::Reasoning {
                    id: "reasoning".into(),
                    summary: vec!["Checking the next page".into()],
                    content: Vec::new(),
                };
                items.push(reasoning.clone());
                if replay {
                    chat.replay_thread_item(reasoning, "turn".into(), ReplayKind::ThreadSnapshot);
                } else {
                    chat.on_agent_reasoning_delta("Checking the next page".into());
                    chat.on_agent_reasoning_final();
                }
            }
        }
        assert!(drain_insert_history(&mut rx).is_empty());
        let projected = crate::thread_transcript::thread_items_to_transcript_cells(
            chat.thread_id,
            &chat.config.cwd,
            items,
            crate::thread_transcript::RawReasoningVisibility::Hidden,
            Some(&chat.config),
        );
        let active = chat.transcript.active_cell.as_ref().expect("active group");
        assert_eq!(projected.len(), 1);
        assert_eq!(active.display_lines(100), projected[0].display_lines(100));
        assert_eq!(
            active.transcript_lines(100),
            projected[0].transcript_lines(100)
        );
        assert_eq!(active.raw_lines(), projected[0].raw_lines());
        outputs.push((active.display_lines(100), active.transcript_lines(100)));
        chat.prepare_assistant_message();
        let cells = drain_insert_history(&mut rx);
        assert_eq!(cells.len(), 1);
        assert!(
            lines_to_single_string(&cells[0])
                .contains("Called Brusher 2 times, called sumologic-mcp 1 time")
        );
        chat.on_mcp_tool_call_completed(grouped_item("4", "Brusher", "completed"));
        chat.finalize_turn();
        let cells = drain_insert_history(&mut rx);
        assert_eq!(cells.len(), 1);
        assert!(!lines_to_single_string(&cells[0]).contains("2 times"));
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[tokio::test]
async fn pending_mcp_count_updates_then_interruption_retains_failure() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(None).await;
    chat.on_mcp_tool_call_started(grouped_item("1", "Brusher", "inProgress"));
    chat.on_mcp_tool_call_started(grouped_item("2", "sumologic-mcp", "inProgress"));
    chat.on_mcp_tool_call_completed(grouped_item("2", "sumologic-mcp", "completed"));
    assert!(active_blob(&chat).contains("Called sumologic-mcp 1 time · Calling Brusher…"));
    chat.finalize_active_cell_as_failed();
    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 1);
    assert!(
        lines_to_single_string(&cells[0])
            .contains("Called Brusher 1 time, called sumologic-mcp 1 time · 1 failed")
    );
}
