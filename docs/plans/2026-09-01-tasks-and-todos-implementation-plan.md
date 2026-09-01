# Tasks / To-Do Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Provide tasks (todos) with completion state (`[ ]` / `[x]`), due date, and importance, manageable via both CLI (interactive inline menu and one-shot flags) and TUI (Day view and Month day preview).

**Architecture:**
- SQLite migration v4 adding `tasks` table with indexing on `date` and `is_done`.
- Storage CRUD API for tasks in `src/storage/tasks.rs`.
- CLI commands for tasks in `src/cli/task.rs` supporting an inline interactive viewport menu (`--task`), as well as one-shot commands (`--task-add`, `--task-toggle`, `--task-list`).
- TUI integration into Month day preview (`o`) and Day view, with `Space` hotkey to toggle completion state.

**Tech Stack:** Rust 2024 edition, rusqlite 0.32 (bundled), ratatui 0.29, crossterm 0.28, chrono 0.4.

---

### Task 1: Data Model & Storage Layer (Migration v4 & Task CRUD)

**Files:**
- Create: `src/model/task.rs`
- Modify: `src/model/mod.rs`
- Create: `src/storage/tasks.rs`
- Modify: `src/storage/migrations.rs`
- Modify: `src/storage/database.rs`
- Modify: `src/storage/mod.rs`

**Step 1: Write tests for task model & storage**
- Test migration v4 recording.
- Test task creation, retrieval by date, completion toggling, and deletion.

**Step 2: Implement `Task` and `NewTask` models**
- Define `Task`, `NewTask`, and `TaskFilter` in `src/model/task.rs`.
- Re-export in `src/model/mod.rs`.

**Step 3: Implement migration v4 and storage methods**
- Add `VERSION_4` SQL in `src/storage/migrations.rs`.
- Implement `create_task`, `toggle_task`, `delete_task`, `tasks_on_date`, `tasks_between`, `all_tasks` in `src/storage/tasks.rs`.

**Step 4: Run storage tests**
Run: `cargo test storage`

**Step 5: Commit**
```bash
git add src/model/ src/storage/
git commit -m "feat(storage): add tasks table migration v4 and storage methods"
```

---

### Task 2: CLI Layer (Interactive Inline Menu & Flags)

**Files:**
- Create: `src/cli/task.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`

**Step 1: Write tests for CLI parsing of task commands**
- In `src/cli/mod.rs`:
  - parse `--task`, `-t`, `task` -> `CliCommand::TaskMenu`
  - parse `--task-add "Title" ...` -> `CliCommand::TaskAdd`
  - parse `--task-toggle <ID>` -> `CliCommand::TaskToggle`
  - parse `--task-list [FILTER]` -> `CliCommand::TaskList`

**Step 2: Implement CLI task module**
- In `src/cli/task.rs`:
  - Implement `run_task_add`
  - Implement `run_task_toggle`
  - Implement `run_task_list`
  - Implement `run_task_menu` (interactive inline menu with search, `Space` toggle, `Tab` filter tabs, `a` add, `d`/`x` delete, `q`/`Esc` exit).
- Update `src/main.rs` to dispatch task commands.
- Update `print_help()` in `src/cli/mod.rs`.

**Step 3: Run CLI tests**
Run: `cargo test cli`

**Step 4: Commit**
```bash
git add src/cli/ src/main.rs
git commit -m "feat(cli): add task commands (inline menu and flags)"
```

---

### Task 3: TUI & Month Day Preview Integration

**Files:**
- Modify: `src/app/state.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/app/update/mod.rs`
- Modify: `src/app/update/navigation.rs`
- Modify: `src/ui/month.rs`

**Step 1: Load tasks into AppState and calendar refresh**
- Add `pub tasks: Vec<Task>` to `AppState`.
- Load tasks in `refresh_calendar`.

**Step 2: Integrate tasks into Month Day Preview**
- Include tasks in `render_month_day_preview` with checkboxes: `[ ]` (active) / `[x]` (done).
- In `MonthDayPreview`, pressing `Space` toggles the selected task's completion status immediately.
- Pressing `d`/`x` deletes the task.

**Step 3: Run tests**
Run: `cargo test app ui`

**Step 4: Commit**
```bash
git add src/app/ src/ui/
git commit -m "feat(ui): integrate tasks into month day preview with toggle support"
```

---

### Task 4: End-to-End Verification & Formatting

**Files:**
- Entire workspace

**Step 1: Run format check**
Run: `cargo fmt --all -- --check`

**Step 2: Run all workspace tests**
Run: `cargo test --workspace`

**Step 3: Run clippy**
Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

**Step 4: Commit and finalize**
```bash
git commit --allow-empty -m "chore: verify all checks pass"
```
