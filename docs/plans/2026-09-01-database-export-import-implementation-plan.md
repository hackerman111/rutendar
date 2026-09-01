# Database Export/Import, 'r' Edit Everywhere, and Month Day Preview Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Provide CLI commands `--export` and `--import` for safe SQLite database transfer across machines, enable the `r` key for editing events everywhere (including Agenda), and add an interactive day events preview popup under the selected day in Month view on key `o`.

**Architecture:** 
- Use SQLite `VACUUM INTO` for atomic, defragmented database export.
- Use read-only integrity checking (`PRAGMA integrity_check`) and schema validation for import, with confirmation prompt, automatic backup of existing database, and atomic replacement.
- Unify `r` and `e` to trigger `Action::Edit` across the app (remapping Agenda filter cycle to `R`).
- Implement `Popup::MonthDayPreview` anchored below the selected day cell in Month view with `j`/`k` navigation and `Enter`/`r` opening.

**Tech Stack:** Rust 2024 edition, rusqlite 0.32 (bundled), ratatui 0.29, crossterm 0.28, chrono 0.4.

---

### Task 1: Storage Layer - Database Export & Validation

**Files:**
- Modify: `src/storage/database.rs`
- Modify: `src/storage/mod.rs`

**Step 1: Write tests for `export` and `validate_file`**
Add tests in `src/storage/database.rs`:
- `test_database_export_creates_valid_standalone_sqlite_file`
- `test_database_validate_file_checks_integrity_and_schema`

**Step 2: Run tests to verify they fail**
Run: `cargo test storage::database::tests::test_database_export`

**Step 3: Implement `export` and `validate_file` in `Database`**
- `pub fn export(&self, destination: &Path) -> StorageResult<(u64, usize)>`
- `pub fn validate_file(path: &Path) -> StorageResult<usize>`

**Step 4: Run tests to verify they pass**
Run: `cargo test storage::database::tests`

**Step 5: Commit**
```bash
git add src/storage/
git commit -m "feat(storage): add database export and validation functions"
```

---

### Task 2: CLI Layer - `--export` and `--import` Commands

**Files:**
- Modify: `src/cli/mod.rs`
- Create: `src/cli/export.rs`
- Create: `src/cli/import.rs`
- Modify: `src/main.rs`

**Step 1: Write tests for CLI parsing of export and import**
In `src/cli/mod.rs`:
- test parsing `--export`, `-e`, `export` with and without path
- test parsing `--import`, `-i`, `import` with path and optional `--force` / `-f`

**Step 2: Run test to verify it fails**
Run: `cargo test cli::tests`

**Step 3: Implement `run_export` and `run_import`**
- Implement `CliCommand::Export` and `CliCommand::Import`
- Implement `run_export` with formatted output (file size, event count)
- Implement `run_import` with integrity verification, user prompt `[y/N]`, automatic `.bak` copy, atomic replacement, and migration execution
- Update `main.rs` to dispatch `CliCommand::Export` and `CliCommand::Import`
- Update `--help` text in `src/cli/mod.rs`

**Step 4: Run CLI tests**
Run: `cargo test cli`

**Step 5: Commit**
```bash
git add src/cli/ src/main.rs
git commit -m "feat(cli): add --export and --import commands"
```

---

### Task 3: Keymap & Agenda - Edit on 'r' and Filter Cycle on 'R'

**Files:**
- Modify: `src/input/keymap.rs`
- Modify: `src/app/update/mod.rs`
- Modify: `src/ui/popup.rs` (Help reference)

**Step 1: Write test for keymap mapping 'r' to Edit and 'R' to CycleItemType**
In `src/input/keymap.rs`:
- test that `r` maps to `Action::Edit` in normal mode
- test that `R` maps to `Action::CycleItemType` in normal mode

**Step 2: Run test to verify it fails**
Run: `cargo test input::keymap::tests`

**Step 3: Implement mapping in `src/input/keymap.rs` and update Help in `src/ui/popup.rs`**
- Map `'r'` to `Action::Edit`
- Map `'R'` to `Action::CycleItemType`
- Update Help reference rows to reflect `e / r` for edit and `R` for item type in Agenda

**Step 4: Run tests**
Run: `cargo test input::keymap::tests`

**Step 5: Commit**
```bash
git add src/input/keymap.rs src/app/update/mod.rs src/ui/popup.rs
git commit -m "feat(keymap): support 'r' for editing and 'R' for Agenda type filter"
```

---

### Task 4: Month View - Day Preview Popup on 'o'

**Files:**
- Modify: `src/app/state.rs`
- Modify: `src/app/action.rs`
- Modify: `src/app/update/mod.rs`
- Modify: `src/app/update/navigation.rs`
- Modify: `src/ui/month.rs`
- Modify: `src/ui/popup.rs`

**Step 1: Add `Popup::MonthDayPreview` and navigation actions**
- In `src/app/state.rs`: add `MonthDayPreview { date: NaiveDate, selected: usize }` to `Popup`
- In `src/app/action.rs`: add `OpenMonthDay` (or trigger on `'o'` in `View::Month`)
- In `src/input/keymap.rs`: when in normal mode, handle `'o'` appropriately or dispatch action

**Step 2: Implement update logic for MonthDayPreview**
- In `src/app/update/navigation.rs`:
  - Handle opening preview on `'o'` in `View::Month`
  - When preview is open: `j`/`k` moves selection; `Enter` opens the item in Day view; `r`/`e` opens editor; `Esc`/`o` closes preview.
- Write unit tests for opening and navigating `MonthDayPreview`.

**Step 3: Implement rendering of day preview popup anchored to the month grid cell**
- In `src/ui/month.rs`:
  - Calculate the cell rectangle for the active day
  - Compute popup rectangle: anchored directly below the day cell (or above if on lower rows)
  - Render box with title (day date), list of occurrences with time, importance symbol, title, and notes
  - Highlight selected item

**Step 4: Verify with `cargo test` and `cargo check`**
Run: `cargo test && cargo check`

**Step 5: Commit**
```bash
git add src/app/ src/input/ src/ui/
git commit -m "feat(ui): add month view day preview popup on 'o'"
```

---

### Task 5: End-to-End Verification & Formatting

**Files:**
- Whole project

**Step 1: Run workspace formatting**
Run: `cargo fmt --all -- --check`

**Step 2: Run all workspace tests**
Run: `cargo test`

**Step 3: Run clippy**
Run: `cargo clippy --all-targets --all-features -- -D warnings`

**Step 4: Commit and finalize**
```bash
git commit --allow-empty -m "chore: verify all checks pass"
```
