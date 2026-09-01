# Экспорт заметок в CLI и Inline-меню: План реализации

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Добавить возможность экспорта заметок за день, неделю, месяц, все и выбранные пользователем в формате Markdown (.md) через CLI флаги и интерактивное 10-строчное inline-меню.

**Architecture:** 
- Хранилище: метод `all_notes` в `Database` для выборки всех заметок с прикрепленными ссылками.
- Форматирование: генератор структурированного Markdown-документа из `&[Note]`.
- CLI & Inline: модуль `src/cli/note_export.rs` с поддержкой однострочного экспорта (`--export-notes`) и интерактивного Ratatui inline-интерфейса (`--notes` / `-n`) с вкладками периодов, чекбоксами выбора (`Space`), выбором всех (`a`) и экспортом (`e`).

**Tech Stack:** Rust, Ratatui (Viewport::Inline), Chrono, Rusqlite.

---

### Task 1: Модель выборки и генератор Markdown
**Files:**
- Modify: `src/storage/database.rs`
- Create: `src/cli/note_export.rs`
- Modify: `src/cli/mod.rs`

**Шаги:**
1. Добавить в `Database` метод `pub fn all_notes(&self) -> Result<Vec<Note>, rusqlite::Error>` с загрузкой прикрепленных ссылок через `links_for_notes`.
2. Написать юнит-тест `storage::database::tests::all_notes_loads_with_links`.
3. Создать `src/cli/note_export.rs` и реализовать функцию `format_notes_markdown(notes: &[Note], period_label: &str) -> String`.
4. Написать юнит-тест `format_notes_markdown_includes_all_fields_and_links`.
5. Зафиксировать коммитом в git.

---

### Task 2: Однострочные команды CLI и интерактивное Inline-меню
**Files:**
- Modify: `src/cli/note_export.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/main.rs`

**Шаги:**
1. В `src/cli/note_export.rs` реализовать:
   - Перечисление `NotesPeriod`: `Day`, `Week`, `Month`, `All`.
   - Функцию `run_notes_export(database, period, file_path, stdout)`.
   - Интерактивную функцию `run_notes_menu(database)` с `Viewport::Inline(10)`:
     - Вкладки периодов `[ День ] [ Неделя ] [ Месяц ] [ Все ]` (`Tab`).
     - Живой поиск по тексту и заголовку.
     - Чекбоксы выбора `[*] / [ ]` (`Space`).
     - Выбор всех в текущем фильтре (`a`).
     - Экспорт выбранных заметок в `.md` (`e`).
2. В `src/cli/mod.rs`:
   - Расширить `CliCommand` вариантами `NotesMenu` и `NotesExport`.
   - Добавить парсинг аргументов `--notes`, `-n`, `notes`, `--export-notes`.
   - Обновить `print_help()`.
3. В `src/main.rs` подключить диспетчеризацию новых команд.
4. Добавить интеграционные тесты CLI в `src/cli/tests.rs`.
5. Зафиксировать коммитом в git.

---

### Task 3: Верификация и обновление документации
**Files:**
- Modify: `README.md`

**Шаги:**
1. Добавить описание команд `--notes` и `--export-notes` в `README.md`.
2. Запустить `cargo fmt --all -- --check`.
3. Запустить `cargo check --workspace --all-targets`.
4. Запустить `cargo test --workspace`.
5. Запустить `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
6. Зафиксировать финальные изменения в git.
