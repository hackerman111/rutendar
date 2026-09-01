# Дизайн: Задания (Tasks / To-Do) с отметкой о выполнении в CLI и TUI

## 1. Контекст и цели

Предоставить возможность вести список заданий (задач) с отметкой о выполнении (`[ ]` / `[x]`), опциональным дедлайном (датой) и важностью:
1. **Полноценная поддержка в CLI:**
   - Интерактивное inline-меню (`rutendar --task` / `-t` / `task`) с переключением статуса по `Space`, фильтрами (Активные / Все / Выполненные) по `Tab` и быстрым добавлением по `a`.
   - Однострочные команды через флаги: `--task-add`, `--task-toggle` / `--task-done`, `--task-list`.
2. **Интеграция в TUI:**
   - Задачи на выбранный день отображаются в представлении дня с чекбоксами `[ ]` / `[x]`.
   - В выпадающем меню дня в режиме месяца (по клавише `o`) задачи выводятся вместе с событиями дня и переключаются по `Space`.

---

## 2. Архитектура и структура базы данных

### 2.1. Схема данных (SQLite Migration v4)
В `src/storage/migrations.rs` добавляется версия 4:
```sql
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT,
    date TEXT, -- 'YYYY-MM-DD' или NULL (без срока)
    is_done INTEGER NOT NULL DEFAULT 0 CHECK (is_done IN (0, 1)),
    importance INTEGER NOT NULL DEFAULT 1 CHECK (importance BETWEEN 0 AND 3),
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_date ON tasks(date);
CREATE INDEX IF NOT EXISTS idx_tasks_done ON tasks(is_done);
```

### 2.2. Модель данных (`src/model/task.rs`, `src/model/mod.rs`)
- `Task`:
  - `pub id: i64`
  - `pub title: String`
  - `pub description: Option<String>`
  - `pub date: Option<NaiveDate>`
  - `pub is_done: bool`
  - `pub importance: Importance`
  - `pub completed_at: Option<DateTime<Local>>` (или String)
- `NewTask`:
  - `pub title: String`
  - `pub description: Option<String>`
  - `pub date: Option<NaiveDate>`
  - `pub importance: Importance`
- `TaskFilter`: `Active`, `Done`, `All`

### 2.3. Хранилище (`src/storage/tasks.rs`, `src/storage/database.rs`)
- `create_task(&mut self, task: &NewTask) -> StorageResult<i64>`
- `toggle_task(&mut self, id: i64) -> StorageResult<bool>`
- `delete_task(&mut self, id: i64) -> StorageResult<()>`
- `tasks_on_date(&self, date: NaiveDate) -> StorageResult<Vec<Task>>`
- `tasks_between(&self, start: NaiveDate, end: NaiveDate) -> StorageResult<Vec<Task>>`
- `all_tasks(&self, filter: TaskFilter) -> StorageResult<Vec<Task>>`

---

## 3. CLI-интерфейс

### 3.1. Интерактивное inline-меню (`src/cli/task.rs`)
- Вызов: `rutendar --task` / `-t` / `task`
- Inline viewport на 9-10 строк (на базе `ratatui::Viewport::Inline`).
- Верхняя строка: строка поиска и вкладки фильтра `[ Активные ] [ Все ] [ Выполненные ]` (переключение по `Tab` / `← / →`).
- Список задач: `› [ ] ! Подготовить отчет`, `[x] Купить продукты`.
- Управление:
  - `Space` — переключить статус выбранной задачи (сразу сохраняет в БД).
  - `j` / `k` (или `↓` / `↑`) — выбор задачи.
  - `a` — интерактивный ввод названия задачи и добавление в БД.
  - `d` / `x` — удаление задачи.
  - `Esc` / `q` — выход.

### 3.2. Флаги командной строки
- `--task-add <НАЗВАНИЕ> [--date <ДАТА>] [--desc <ОПИСАНИЕ>] [--importance <ВАЖНОСТЬ>]`:
  Создает задачу и выводит подтверждение с ID.
- `--task-toggle <ID>` / `--task-done <ID>`:
  Переключает статус задачи и выводит новый статус.
- `--task-list [today|all|done]`:
  Форматированный вывод списка задач в stdout.

---

## 4. TUI-интеграция

- `AppState` загружает задачи дня в `pub tasks: Vec<Task>`.
- В режиме Дня: задачи отображаются в списке дел с чекбоксами `[ ]` / `[x]`. Нажатие `Space` на задаче переключает ее статус.
- В превью дня в режиме Месяца (`o`): задачи дня выводятся в списке превью с чекбоксами, `Space` переключает отметку выполнения.

---

## 5. План верификации
1. Unit-тесты для миграции схемы v4 и CRUD-операций задач в `src/storage`.
2. Unit-тесты парсера CLI-аргументов для `--task`, `--task-add`, `--task-toggle`, `--task-list`.
3. Интеграционные тесты переключения статуса задачи в CLI.
4. Проверка работы TUI и превью месяца с задачами.
5. Прогон `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt`.
