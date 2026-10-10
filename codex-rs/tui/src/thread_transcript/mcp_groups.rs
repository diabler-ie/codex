//! Assemble MCP activity across hidden reasoning and repair history-page group boundaries.

use super::computer_groups::adjacent_activity_items;
use super::tools::McpHistory;
use crate::history_cell::HistoryCell;
use crate::history_cell::McpToolCallCell;
use crate::history_cell::McpToolCallGroupCell;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::Turn;
use std::sync::Arc;

pub(super) fn append(group: &mut McpToolCallGroupCell, call: McpHistory) {
    let cell = McpToolCallCell::new(call.id, call.invocation, /*animations_enabled*/ false);
    group.complete(cell, call.duration, call.result);
}

/// Rebuild only an adjacent MCP group, using its persisted IDs to verify the turn boundary.
pub(crate) fn join_mcp_groups(
    older: &Arc<dyn HistoryCell>,
    newer: &Arc<dyn HistoryCell>,
    turns: &[Turn],
) -> Option<Arc<dyn HistoryCell>> {
    let older = older.as_any().downcast_ref::<McpToolCallGroupCell>()?;
    let newer = newer.as_any().downcast_ref::<McpToolCallGroupCell>()?;
    let mut group = completed_group(adjacent_items(older, newer, turns)?)?;
    group.group.details = newer.group.details.clone();
    group
        .group
        .details
        .prepend(older.group.details.clone(), older.call_ids().count());
    Some(Arc::new(group))
}

/// Restore only the older half; the caller retains the live half and its pending calls/timers.
pub(crate) fn older_mcp_group(
    older: &dyn HistoryCell,
    newer: &McpToolCallGroupCell,
    turns: &[Turn],
) -> Option<McpToolCallGroupCell> {
    let older = older.as_any().downcast_ref::<McpToolCallGroupCell>()?;
    let items = adjacent_items(older, newer, turns)?;
    let older_items = items
        .iter()
        .filter(|item| !matches!(item, ThreadItem::Reasoning { .. }))
        .take(older.call_ids().count())
        .cloned()
        .collect::<Vec<_>>();
    let mut group = completed_group(&older_items)?;
    group.group.details = older.group.details.clone();
    Some(group)
}

fn adjacent_items<'a>(
    older: &McpToolCallGroupCell,
    newer: &McpToolCallGroupCell,
    turns: &'a [Turn],
) -> Option<&'a [ThreadItem]> {
    let ids = older.call_ids().chain(newer.call_ids()).collect::<Vec<_>>();
    adjacent_activity_items(&ids, turns)
}

pub(super) fn completed_group(items: &[ThreadItem]) -> Option<McpToolCallGroupCell> {
    let mut group = None;
    for item in items {
        if matches!(item, ThreadItem::Reasoning { .. }) {
            continue;
        }
        let call = McpHistory::from_item(item.clone())?;
        if call.invocation.is_computer_activity() {
            return None;
        }
        append(group.get_or_insert_default(), call);
    }
    group
}

#[cfg(test)]
#[path = "mcp_groups_tests.rs"]
mod tests;
