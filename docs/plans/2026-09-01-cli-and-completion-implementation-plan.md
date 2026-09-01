# CLI (`--list`, `--add`) and Autocompletion Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Add a compact CLI to `rutendar` with `--list` (inline interactive event selector across day/week/month with pretty card output) and `--add` (terminal event creation), plus tag and filesystem path autocompletion in both TUI and CLI.

**Architecture:** Use existing `ratatui` (0.29) and `crossterm` (0.28) dependencies without external crates. Implement `Viewport::Inline` for the zsh-style inline search widget, a shared `completion` module for tag and directory completion, an interactive line reader for `--add`, and integrate path suggestions into the TUI event editor.

**Tech Stack:** Rust (edition 2024), ratatui, crossterm, rusqlite, chrono.

---

### Task 1: Shared Autocompletion Module (`src/completion/mod.rs`)

**Files:**
- Create: `src/completion/mod.rs`
- Modify: `src/lib.rs`
- Test: `src/completion/mod.rs` (inline test module)

**Step 1: Write the failing tests**
Test `complete_tags` and `complete_directories`:
- `complete_directories` handles empty input, relative paths, absolute paths, and tilde `~` expansion.
- `complete_directories` filters only directories and appends `/`.
- `complete_tags` returns tags matching the prefix.

**Step 2: Run test to verify it fails**
Run: `cargo test completion`
Expected: FAIL (module does not exist yet)

**Step 3: Write minimal implementation**
- Implement `complete_tags(db: &Database, prefix: &str) -> Vec<String>`.
- Implement `complete_directories(input: &str) -> Vec<String>`.
  - Expand `~` to `dirs::home_dir()` or `$HOME`.
  - Determine parent directory and prefix.
  - Read parent dir with `std::fs::read_dir`, filter entries that `is_dir()`, sort alphabetically.

**Step 4: Run test to verify it passes**
Run: `cargo test completion`
Expected: PASS

**Step 5: Commit**
```bash
git add src/completion/mod.rs src/lib.rs
git commit -m "feat(completion): add tag and directory autocompletion helpers"
```

---

### Task 2: Autocompletion in TUI Event Editor

**Files:**
- Modify: `src/app/state.rs`
- Modify: `src/app/update/editor.rs`
- Modify: `src/ui/popup.rs`
- Test: `src/app/update/editor.rs` (or `src/app/update/tests.rs`)

**Step 1: Write the failing tests**
- In `tests`, test that when editing `EventForm` on `DIRECTORY_FIELD`, typing characters updates `path_suggestions`.
- Pressing `adjust_field(true)` (Tab) autocompletes the directory with the first suggestion.

**Step 2: Run test to verify it fails**
Run: `cargo test directory_autocompletion`
Expected: FAIL (`path_suggestions` field missing or logic not implemented)

**Step 3: Write minimal implementation**
- Add `pub path_suggestions: Vec<String>` to `AppState`.
- In `src/app/update/editor.rs`:
  - Implement `refresh_path_suggestions(&mut self)` calling `completion::complete_directories`.
  - Call `refresh_path_suggestions()` in `push_char` and `backspace` when on `DIRECTORY_FIELD`.
  - In `adjust_field()`, if `form.active == EventForm::DIRECTORY_FIELD`, complete with suggestion.
  - In `move_field()`, clear `path_suggestions`.
- In `src/ui/popup.rs`:
  - Render `path_suggestions` in the footer popup: `│ [AUTO]: dir1/ dir2/` when on `DIRECTORY_FIELD`.

**Step 4: Run test to verify it passes**
Run: `cargo test`
Expected: PASS

**Step 5: Commit**
```bash
git add src/app/state.rs src/app/update/editor.rs src/ui/popup.rs
git commit -m "feat(tui): add directory autocompletion to event editor"
```

---

### Task 3: Pretty Event Card Output Formatting (`src/cli/format.rs`)

**Files:**
- Create: `src/cli/format.rs`
- Test: `src/cli/format.rs` (inline test module)

**Step 1: Write the failing tests**
- Test `format_event_card(&EventOccurrence)`:
  - Contains title, formatted date, time range, importance indicator.
  - Contains tags, directory if present.
  - Contains description block if present.
  - Contains links block with labels and URLs if present.
  - Uses clean box-drawing characters and ANSI color styling.

**Step 2: Run test to verify it fails**
Run: `cargo test cli::format`
Expected: FAIL (module does not exist)

**Step 3: Write minimal implementation**
- Implement `format_event_card(event: &EventOccurrence) -> String`.
- Support colorized output (with ANSI codes) respecting terminal width and pretty borders.

**Step 4: Run test to verify it passes**
Run: `cargo test cli::format`
Expected: PASS

**Step 5: Commit**
```bash
git add src/cli/format.rs
git commit -m "feat(cli): add pretty event card formatting"
```

---

### Task 4: Interactive Inline Menu (`src/cli/list.rs`)

**Files:**
- Create: `src/cli/list.rs`
- Modify: `src/cli/mod.rs`
- Test: `src/cli/list.rs`

**Step 1: Write the failing tests**
- Test event filtering logic by period (Day, Week, Month) and text search filter.

**Step 2: Run test to verify it fails**
Run: `cargo test cli::list`
Expected: FAIL

**Step 3: Write minimal implementation**
- Implement `ListApp` with `ratatui::Viewport::Inline(9)`:
  - Tabs for `[ ДЕНЬ ]`, `[ НЕДЕЛЯ ]`, `[ МЕСЯЦ ]`.
  - Search query line `🔍 Поиск: ...`.
  - List of matching events with cursor indicator `▸`.
  - Key handling:
    - Tab / Left / Right: cycle period.
    - Up / Down / Ctrl-P / Ctrl-N: navigate items.
    - Chars / Backspace: edit query.
    - Enter: return selected event.
    - Esc / Ctrl-C: exit without selection.
- When an event is selected, clear the inline viewport and print the formatted card to stdout via `println!("{}", format_event_card(&event))`.

**Step 4: Run test to verify it passes**
Run: `cargo test cli::list`
Expected: PASS

**Step 5: Commit**
```bash
git add src/cli/list.rs src/cli/mod.rs
git commit -m "feat(cli): implement interactive inline event list"
```

---

### Task 5: Terminal Event Addition (`src/cli/add.rs`)

**Files:**
- Create: `src/cli/add.rs`
- Modify: `src/cli/mod.rs`
- Test: `src/cli/add.rs`

**Step 1: Write the failing tests**
- Test CLI arg parsing for `--add` flags (`--title`, `--date`, `--time`, `--importance`, `--tags`, `--dir`, `--desc`).
- Test validation and creation of `NewEvent` from CLI inputs.

**Step 2: Run test to verify it fails**
Run: `cargo test cli::add`
Expected: FAIL

**Step 3: Write minimal implementation**
- Implement interactive prompt reader:
  - Title (required).
  - Date (default today, supports `DD.MM.YYYY`, `today`, `tomorrow`).
  - Time (optional, format `HH:MM` or `HH:MM-HH:MM`).
  - Importance (None, Low, Normal, High).
  - Tags (optional, with `Tab` autocompletion from database).
  - Directory (optional, with `Tab` autocompletion from filesystem).
  - Description (optional).
- Insert into database via `database.create_event`.
- Print confirmation with summary.

**Step 4: Run test to verify it passes**
Run: `cargo test cli::add`
Expected: PASS

**Step 5: Commit**
```bash
git add src/cli/add.rs
git commit -m "feat(cli): implement terminal event addition with tab completion"
```

---

### Task 6: CLI Argument Dispatch & Entry Point (`src/main.rs`, `src/cli/mod.rs`)

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `src/cli/mod.rs`
- Modify: `TODO.txt`

**Step 1: Write dispatch logic & tests**
- Parse `std::env::args()`:
  - If `--list` / `-l` -> `cli::run_list`
  - If `--add` / `-a` -> `cli::run_add`
  - If `--help` / `-h` -> print help text
  - If no CLI flags -> launch existing full-screen `run_tui()`
- Update `TODO.txt` marking completed items.

**Step 2: Verify full workspace**
- Run `cargo check --workspace --all-targets`
- Run `cargo test --workspace`
- Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Run `cargo fmt --all -- --check`

**Step 3: Commit**
```bash
git add src/main.rs src/lib.rs src/cli/mod.rs TODO.txt
git commit -m "feat(cli): dispatch --list and --add commands and update TODO"
```
