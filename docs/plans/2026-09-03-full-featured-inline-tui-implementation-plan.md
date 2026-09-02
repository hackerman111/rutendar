# Full-Featured Compact Inline TUI Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Transform `rutendar`'s default execution into a full-featured compact interactive inline TUI (13-line `ratatui::Viewport::Inline`), starting on Today, with day/week navigation, live search, inline task toggling, interactive event addition, pretty-printing of day summaries and cards to stdout, and seamless transition to the full fullscreen TUI.

**Architecture:** Modular architecture in `src/cli/inline/` (`state.rs`, `render.rs`, `mod.rs`) cleanly decoupled from fullscreen `App`. Extends `src/cli/format.rs` with `format_day_summary` adhering to strict terminal box-drawing invariants. Connects default CLI invocation in `src/main.rs` to inline mode with seamless handoff to `App` on alternate screen if requested.

**Tech Stack:** Rust 2024, Ratatui 0.29 (`Viewport::Inline`), Crossterm 0.28, Chrono 0.4, Rusqlite 0.32.

---

### Task 1: Pretty Day Summary Formatter (`src/cli/format.rs`)

**Files:**
- Modify: `src/cli/format.rs`
- Test: `src/cli/format.rs:tests`

**Step 1: Write the failing unit test**
Add test `test_format_day_summary_renders_boxed_schedule_and_tasks` to `src/cli/format.rs` testing that `format_day_summary` produces closed borders (`╭`, `╮`, `╰`, `╯`, `├`, `┤`), contains event titles, times, importance badges, and task checkboxes.

**Step 2: Run test to verify it fails**
Run: `cargo test --lib cli::format::tests::test_format_day_summary`
Expected: FAIL with function `format_day_summary` not found.

**Step 3: Implement `format_day_summary` in `src/cli/format.rs`**
Implement:
```rust
pub fn format_day_summary(
    date: NaiveDate,
    events: &[EventOccurrence],
    tasks: &[Task],
) -> String
```
Formatting rules:
- Min width 72, closed box with rounded corners.
- Header: `╭── 📅 <День недели>, DD.MM.YYYY ──╮`
- Schedule section: time ranges, importance indicator, title, tags.
- Tasks section: `[ ]` / `[x]`, title, importance.
- Bottom border: `╰────────────────────────────────╯`.

**Step 4: Run test to verify it passes**
Run: `cargo test --lib cli::format`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/cli/format.rs
git commit -m "feat(cli): add format_day_summary for beautiful terminal day output"
```

---

### Task 2: Inline State and Model (`src/cli/inline/state.rs`)

**Files:**
- Create: `src/cli/inline/state.rs`
- Create: `src/cli/inline/mod.rs` (initial exports)
- Modify: `src/cli/mod.rs` (declare `pub mod inline;`)
- Test: `src/cli/inline/state.rs:tests`

**Step 1: Write failing tests for `InlineApp`**
Test tab switching (`Day`, `Week`, `Search`), date increment/decrement, today reset, event filtering for search, and task status toggle logic.

**Step 2: Run tests to verify failure**
Run: `cargo test --lib cli::inline::state`
Expected: FAIL with compilation error (module missing).

**Step 3: Implement `InlineApp` and `InlineTab` in `src/cli/inline/state.rs`**
- `pub enum InlineTab { Day, Week, Search }`
- `pub enum InlineOutcome { Exit, OpenFullTui { initial_date: Option<NaiveDate> } }`
- `pub struct InlineApp`
- Implement methods:
  - `new(today: NaiveDate, initial_tab: InlineTab) -> Self`
  - `reload_all(&mut self, db: &Database) -> Result<(), Box<dyn Error>>`
  - `reload_day(&mut self, db: &Database) -> Result<(), Box<dyn Error>>`
  - `reload_week(&mut self, db: &Database) -> Result<(), Box<dyn Error>>`
  - `reload_search(&mut self, db: &Database) -> Result<(), Box<dyn Error>>`
  - `next_day(&mut self, db: &Database)`
  - `prev_day(&mut self, db: &Database)`
  - `today(&mut self, db: &Database)`
  - `toggle_selected_task(&mut self, db: &Database) -> Result<(), Box<dyn Error>>`
  - `current_items_count(&self) -> usize`

**Step 4: Run tests to verify they pass**
Run: `cargo test --lib cli::inline::state`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/cli/inline/state.rs src/cli/inline/mod.rs src/cli/mod.rs
git commit -m "feat(cli): implement inline state management and navigation"
```

---

### Task 3: Inline Ratatui Rendering (`src/cli/inline/render.rs`)

**Files:**
- Create: `src/cli/inline/render.rs`
- Modify: `src/cli/inline/mod.rs` (re-export `render_inline`)
- Test: `src/cli/inline/render.rs:tests`

**Step 1: Write test for layout and widget creation**
Add headless render test using `ratatui::backend::TestBackend` verifying that 13 lines are rendered without panics and contain active tab markers and day information.

**Step 2: Run test to verify failure**
Run: `cargo test --lib cli::inline::render`
Expected: FAIL.

**Step 3: Implement `render_inline` in `src/cli/inline/render.rs`**
Render 13 lines:
- Header line: Tabs (`[ 1 ДЕНЬ ]  2 НЕДЕЛЯ   3 ПОИСК`), current date, shortcut hint `(F: Полный TUI · q: Выход)`.
- Context / Subheader line: Date nav hints or search prompt with cursor `█`.
- Content area (8-9 lines):
  - Day tab: Events list (marker `▸`, importance, time, title, tags) + Tasks section (`[ ]`/`[x]`).
  - Week tab: Grouped day headers + week events.
  - Search tab: Filtered events list with dates.
- Footer action bar: `[↑/↓] Навигация  [Enter] Карточка  [p] Сводка дня  [a] Добавить  [Tab/1/2/3] Вкладка`.

**Step 4: Run test to verify it passes**
Run: `cargo test --lib cli::inline::render`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/cli/inline/render.rs src/cli/inline/mod.rs
git commit -m "feat(cli): implement 13-line inline tui layout renderer"
```

---

### Task 4: Interactive Inline Event Loop & Actions (`src/cli/inline/mod.rs`)

**Files:**
- Modify: `src/cli/inline/mod.rs`

**Step 1: Implement `run_inline`**
```rust
pub fn run_inline(
    database: &mut Database,
    config: &Config,
    initial_period: Option<Period>,
) -> Result<InlineOutcome, Box<dyn Error>>
```
- Setup terminal with `Viewport::Inline(13)`.
- Event loop with crossterm:
  - `Tab` / `BackTab`: cycle tabs.
  - `1`, `2`, `3`: switch tab directly.
  - `←` / `→` / `h` / `l`: next / previous day (in Day tab when not searching).
  - `t`: jump to today.
  - `↑` / `↓` / `k` / `j`: move selection cursor.
  - `Space`: toggle task status if selection is on a task.
  - `Enter`: exit inline, print formatted card of selected event to stdout.
  - `p`: exit inline, print full day summary via `format_day_summary` to stdout.
  - `a` / `+`: suspend inline raw mode, run `prompt_interactive(database, app.current_date)`, create event, reload `InlineApp`, redraw inline.
  - `F` / `Shift+T`: clean exit inline returning `InlineOutcome::OpenFullTui { initial_date: Some(app.current_date) }`.
  - `q` / `Esc`: clean exit without output.
  - Search input: append chars, backspace, live filtering.

**Step 2: Run workspace checks**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit**
```bash
git add src/cli/inline/mod.rs
git commit -m "feat(cli): implement interactive inline event loop and actions"
```

---

### Task 5: CLI Arguments and Main Entrypoint Integration (`src/cli/mod.rs`, `src/main.rs`)

**Files:**
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`
- Test: `src/cli/mod.rs:tests`

**Step 1: Update CLI parser tests in `src/cli/mod.rs`**
- Test `rutendar --full`, `rutendar --tui`, `rutendar tui` -> `CliCommand::FullTui`.
- Test `rutendar --inline`, `rutendar -i` -> `CliCommand::Inline(None)`.
- Test empty args -> `Ok(None)`.

**Step 2: Run test to verify failure**
Run: `cargo test --lib cli::tests::parse_cli_commands`
Expected: FAIL.

**Step 3: Update `parse_cli_command` and `print_help` in `src/cli/mod.rs`**
Add:
- `CliCommand::FullTui`
- `CliCommand::Inline(Option<Period>)`
Update help text to reflect inline by default and `--full` for fullscreen.

**Step 4: Wire default execution in `src/main.rs`**
- When `cmd.is_none()` or `cmd == Some(CliCommand::Inline(..))`:
  - Run `rutendar::cli::inline::run_inline`.
  - If it returns `InlineOutcome::OpenFullTui { initial_date }`:
    - Instantiate `App`, select `initial_date`, and run full TUI loop.
- When `cmd == Some(CliCommand::FullTui)`:
  - Run full TUI directly.

**Step 5: Run tests to verify they pass**
Run: `cargo test --lib cli`
Expected: PASS.

**Step 6: Commit**
```bash
git add src/cli/mod.rs src/main.rs
git commit -m "feat: wire full-featured inline tui as default CLI behavior"
```

---

### Task 6: Full Verification and Polish

**Files:**
- Run workspace validation:
  - `cargo check --workspace --all-targets`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo fmt --all -- --check`
- Manual smoke testing via CLI flags and simulated keystrokes.

**Step 1: Run automated checks**
Fix any warnings or formatting.

**Step 2: Commit any cleanups**
```bash
git commit -am "chore: polish and verify full inline tui"
```
