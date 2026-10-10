# Grouped MCP tool calls

This fork condenses a consecutive run of two or more MCP calls into one summary:

```text
• Called Brusher 3 times, called sumologic-mcp 2 times
```

Counts include completed calls, including failures. While a call is running, the
row updates in place with a spinner and a separate pending label:

```text
⠋ Called Brusher 5 times · Calling Brusher…
```

Servers appear in the order their first calls started. Multiple pending calls to
one server show an `in progress` count. A failed or interrupted call adds a red
`· N failed` indicator. A single call retains its original presentation.

## Expand and collapse

In the fullscreen TUI, press **F4** to focus tool activity, then **Ctrl+Y** to toggle
all grouped MCP rows currently visible. Ctrl+Y also works while browsing a scrolled
transcript. If some visible groups are collapsed, it expands them; otherwise it
collapses the visible groups. **Escape** returns to composing.

Click a collapsed summary, or use F4 followed by **Enter** or **Right**, to expand
one group. Expanded calls retain their original invocation, result, and order.
Expanded live groups stay expanded when committed to history.

At the live composer, Ctrl+Y retains the existing **yank** behavior, including when
the input is empty. Search, selection, popups, and raw output retain their existing
key handling. Ctrl+T still opens the full transcript.

## Boundaries and history

Visible assistant/user content, other tool activity, and turn boundaries end a
group. Hidden reasoning stays in its original position within expanded details.
History replay and pagination use the same grouping as live calls. Existing
computer activity (`cua_repl`) keeps its specialized computer summary. Server
labels use their MCP identifiers, as provided by Codex's tool events.

The fullscreen transcript is enabled by default upstream. If disabled with
`tui.fullscreen_transcript=false` or `--no-alt-screen`, native terminal scrollback
still shows the summaries; use Ctrl+T to inspect the full calls there.

## Build and run this fork

From `codex-rs`:

```sh
cargo build --locked -p codex-cli --bin codex
./target/debug/codex
```

For an optimized build, add `--release` and run `./target/release/codex`.
The fork does not replace your installed Codex executable.

Checks for this change, from `codex-rs`:

```sh
NO_COLOR= FORCE_COLOR=1 RUST_MIN_STACK=33554432 cargo nextest run --locked -p codex-tui --lib
cargo clippy --locked -p codex-tui --lib --no-deps -- -D warnings
```

The larger stack accommodates the upstream TUI's async test fixtures. Forced
color keeps terminal escape-sequence snapshots consistent in headless sessions.
