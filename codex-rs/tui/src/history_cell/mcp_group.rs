//! Adjacent MCP calls share a summary while retaining each invocation and result in source order.

use super::*;

#[derive(Debug)]
pub(crate) struct McpToolCallGroupCell {
    pub(crate) group: ActivityGroup<McpToolCallCell>,
}

impl Default for McpToolCallGroupCell {
    fn default() -> Self {
        Self {
            group: ActivityGroup::new(Vec::new()),
        }
    }
}

impl McpToolCallGroupCell {
    pub(crate) fn call_ids(&self) -> impl Iterator<Item = &str> {
        self.group.calls.iter().map(McpToolCallCell::call_id)
    }

    pub(crate) fn prepend(&mut self, older: Self) {
        self.group.prepend(older.group);
    }

    pub(crate) fn start(&mut self, call: McpToolCallCell) {
        if !self
            .group
            .calls
            .iter()
            .any(|existing| existing.call_id == call.call_id)
        {
            self.group.calls.push(call);
        }
    }

    pub(crate) fn complete(
        &mut self,
        call: McpToolCallCell,
        duration: Duration,
        result: Result<codex_protocol::mcp::CallToolResult, String>,
    ) {
        let id = call.call_id.clone();
        self.start(call);
        if let Some(call) = self.group.calls.iter_mut().find(|call| call.call_id == id) {
            call.complete(duration, result);
        }
    }

    pub(crate) fn is_grouped(&self) -> bool {
        self.group.calls.len() > 1
    }

    pub(crate) fn is_active(&self) -> bool {
        self.group.calls.iter().any(|call| call.result.is_none())
    }

    pub(crate) fn mark_failed(&mut self) {
        for call in &mut self.group.calls {
            if call.result.is_none() {
                call.mark_failed();
            }
        }
    }

    pub(crate) fn freeze_snapshot(&mut self) {
        for call in &mut self.group.calls {
            call.freeze_snapshot();
        }
    }

    fn summary(&self, width: u16) -> Vec<Line<'static>> {
        if width == 0 || self.group.calls.is_empty() {
            return Vec::new();
        }
        // Keep first-seen server order, including servers whose first call is still pending.
        let mut servers: Vec<(&str, usize, usize)> = Vec::new();
        for call in &self.group.calls {
            let index = servers
                .iter()
                .position(|(server, _, _)| *server == call.invocation.server)
                .unwrap_or_else(|| {
                    servers.push((&call.invocation.server, 0, 0));
                    servers.len() - 1
                });
            if call.result.is_some() {
                servers[index].1 += 1;
            } else {
                servers[index].2 += 1;
            }
        }
        let mut completed = Vec::new();
        let mut pending = Vec::new();
        for (server, count, active) in servers {
            if count > 0 {
                let verb = if completed.is_empty() {
                    "Called"
                } else {
                    "called"
                };
                let unit = if count == 1 { "time" } else { "times" };
                completed.push(format!("{verb} {server} {count} {unit}"));
            }
            if active > 0 {
                let count = if active > 1 {
                    format!(" ({active} in progress)")
                } else {
                    String::new()
                };
                pending.push(format!("Calling {server}…{count}"));
            }
        }
        let mut label = completed.join(", ");
        if !pending.is_empty() {
            if !label.is_empty() {
                label.push_str(" · ");
            }
            label.push_str(&pending.join(", "));
        }
        let failures = self
            .group
            .calls
            .iter()
            .filter(|call| call.success() == Some(false))
            .count();
        let bullet = self
            .group
            .calls
            .iter()
            .find(|call| call.result.is_none())
            .and_then(|call| {
                activity_indicator(
                    Some(call.start_time),
                    MotionMode::from_animations_enabled(call.animations_enabled),
                    ReducedMotionIndicator::StaticBullet,
                )
            })
            .unwrap_or_else(|| {
                if failures > 0 {
                    "•".red()
                } else {
                    "•".dim()
                }
            });
        let mut spans = vec![bullet, " ".into(), label.into()];
        if failures > 0 {
            spans.push(format!(" · {failures} failed").red());
        }
        let indent = " ".repeat(usize::from(width.saturating_sub(1)).min(2));
        adaptive_wrap_line(
            &Line::from(spans),
            RtOptions::new(usize::from(width)).subsequent_indent(indent.into()),
        )
        .iter()
        .map(line_to_static)
        .collect()
    }
}

impl HistoryCell for McpToolCallGroupCell {
    fn append_reasoning(&mut self, cell: Box<dyn HistoryCell>) -> Result<(), Box<dyn HistoryCell>> {
        if self.group.calls.is_empty() {
            Err(cell)
        } else {
            self.group.push_detail(std::sync::Arc::from(cell));
            Ok(())
        }
    }

    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        visible_lines(self.display_hyperlink_lines(width))
    }

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        if self.is_grouped() {
            plain_hyperlink_lines(self.summary(width))
        } else {
            self.group
                .calls
                .first()
                .map_or_else(Vec::new, |call| call.display_hyperlink_lines(width))
        }
    }

    fn compact_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        if self.is_grouped() {
            plain_hyperlink_lines(self.summary(width))
        } else {
            self.group
                .calls
                .first()
                .map_or_else(Vec::new, |call| call.compact_hyperlink_lines(width))
        }
    }

    fn activity_disclosure(&self, _width: u16) -> Option<ActivityDisclosure> {
        Some(if self.is_grouped() {
            ActivityDisclosure::McpCalls
        } else {
            ActivityDisclosure::Generic
        })
    }

    fn activity_ids(&self) -> Vec<String> {
        self.call_ids().map(|id| format!("mcp:{id}")).collect()
    }

    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        visible_lines(self.transcript_hyperlink_lines(width))
    }

    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        let mut lines = Vec::new();
        for (index, call) in self.group.calls.iter().enumerate() {
            lines.extend(call.transcript_hyperlink_lines(width));
            lines.extend(
                self.group
                    .details
                    .lines_after(index + 1, width, HistoryRenderMode::Rich),
            );
        }
        lines
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for (index, call) in self.group.calls.iter().enumerate() {
            lines.extend(call.raw_lines());
            lines.extend(visible_lines(self.group.details.lines_after(
                index + 1,
                u16::MAX,
                HistoryRenderMode::Raw,
            )));
        }
        lines
    }

    fn transcript_animation_tick(&self) -> Option<u64> {
        self.group
            .calls
            .iter()
            .filter_map(HistoryCell::transcript_animation_tick)
            .max()
    }
}

#[cfg(test)]
#[path = "mcp_group_tests.rs"]
mod tests;
