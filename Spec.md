# ТЗ: терминальный календарь

## 1. Назначение

Локальное TUI-приложение для:

- календаря;
- событий;
- циклических событий;
- заметок;
- ссылок внутри заметок;
- тегов;
- важности событий;
- поиска;
- просмотра ближайших событий.

Стек:

```text
Rust
ratatui
crossterm
SQLite
rusqlite
time или chrono
serde
toml
```

Без сервера, аккаунтов и облачной синхронизации.

---

## 2. Главный экран

При запуске открывается **текущая неделя**.

```text
┌───────────────────────────────────────────────────────────────┐
│                     31.08 — 06.09.2026                       │
├────────┬────────┬────────┬────────┬────────┬────────┬─────────┤
│ ПН 31  │ ВТ 01  │ СР 02  │ ЧТ 03  │ ПТ 04  │ СБ 05  │ ВС 06   │
│        │ TODAY  │        │        │        │        │         │
├────────┼────────┼────────┼────────┼────────┼────────┼─────────┤
│ 10:30  │ 09:00  │        │ 12:10  │        │        │         │
│ Матан  │ Лекция │        │ Семинар│        │        │         │
│        │        │        │        │        │        │         │
│ 16:20  │!14:40  │ 15:00  │        │        │        │         │
│ Физика │ Коллок │ Встреча│        │        │        │         │
├────────┴────────┴────────┴────────┴────────┴────────┴─────────┤
│ NEXT  09:00 Лекция · !14:40 Коллок · завтра 15:00 Встреча +4 │
├───────────────────────────────────────────────────────────────┤
│ NORMAL │ WEEK │ 01.09.2026 │ a events │ t upcoming │ ? help  │
└───────────────────────────────────────────────────────────────┘
```

При старте:

```text
selected_date = today
active_view = Week
```

Сегодняшний день и выбранный день должны иметь разные индикаторы.

---

## 3. Представления

Основные:

```text
Week
Day
Month
Year
```

Дополнительные панели:

```text
Agenda
Upcoming
```

`Agenda` и `Upcoming` открываются поверх текущего экрана и после закрытия возвращают пользователя в прежнее состояние.

---

## 4. Week View

Основной режим работы.

Содержит семь дней недели.

Навигация:

```text
h / ←    предыдущий день
l / →    следующий день

j / ↓
k / ↑    навигация внутри содержимого дня
```

Переход через границу недели происходит автоматически:

```text
ВС + l → ПН следующей недели
ПН + h → ВС предыдущей недели
```

В ячейке события показывать:

```text
importance
time
title
recurring indicator
```

Пример:

```text
• 09:00 Лекция
! 14:40 Коллок
↻ 16:20 Семинар
```

Теги показывать только при наличии места.

---

## 5. Day View

Подробный экран выбранного дня.

```text
┌─ ПОНЕДЕЛЬНИК 01.09.2026 ─────────────────────────────┐
│                                                     │
│ СОБЫТИЯ                │ ЗАМЕТКИ                    │
│                        │                            │
│ • 09:00 Матан          │ > Домашка                 │
│ ! 14:40 Коллоквиум     │   Идеи                    │
│   #универ #матан       │   Литература              │
│                        ├────────────────────────────┤
│                        │ Домашка                    │
│                        │ Решить задачи 1–5          │
│                        ├────────────────────────────┤
│                        │ ССЫЛКИ                     │
│                        │ > Условие                  │
│                        │   Лекция                   │
└────────────────────────┴────────────────────────────┘
```

На один день может приходиться несколько событий и заметок.

---

## 6. Month View

Классическая сетка месяца:

```text
ПН ВТ СР ЧТ ПТ СБ ВС
```

Ячейка содержит:

- номер дня;
- индикатор событий;
- индикатор заметок.

Не выводить содержимое событий целиком.

---

## 7. Year View

Показывает 12 месяцев.

Используется для быстрого перехода между месяцами и годами.

Не требуется рисовать внутри каждого месяца полный мини-календарь.

---

# События

## 8. Event

```text
Event
├── id
├── title
├── description?
├── start_date
├── start_time?
├── end_time?
├── importance
├── recurrence_id?
├── created_at
└── updated_at
```

Событие без времени считается событием на весь день.

---

## 9. Важность

Четыре уровня:

```text
None
Low
Normal
High
```

Отображение по умолчанию:

```text
  None
· Low
• Normal
! High
```

Важность не означает срочность.

Например:

```text
через час купить хлеб    Low
через две недели экзамен High
```

`NEXT` сортируется по времени, а не по важности.

---

## 10. Теги

Одному событию можно назначить любое число тегов:

```text
#универ
#лекция
#матан
#research
```

Модель:

```text
Event ──< EventTag >── Tag
```

```text
Tag
├── id
├── name
└── normalized_name
```

```text
EventTag
├── event_id
└── tag_id
```

Не хранить:

```text
"универ,лекция,матан"
```

одной строкой.

Нормализация:

```text
"ML"
"ml"
" ml "
```

должны соответствовать одному тегу.

---

## 11. Редактор события

Минимальная форма:

```text
TITLE
Коллоквиум

DATE
15.09.2026

TIME
14:40 - 16:10

IMPORTANCE
High

TAGS
#универ #матан #коллок

REPEAT
Weekly

ENDS
15.12.2026
```

Для тегов нужен autocomplete существующих тегов.

---

# Повторяющиеся события

## 12. Recurrence

Не создавать отдельную запись `Event` на каждую неделю.

```text
Recurrence
├── id
├── frequency
├── interval
├── weekdays
├── start_date
├── end_date?
└── count?
```

Поддержать:

```text
Daily
Weekly
Monthly
```

Для первой версии достаточно полностью реализовать:

```text
Never
Weekly
```

Пример:

```text
frequency = weekly
interval = 1
weekdays = [TUE]
start_date = 2026-09-01
end_date = 2026-12-15
```

означает лекцию каждый вторник.

Поддержать:

```text
каждую неделю
раз в две недели
несколько дней недели
```

---

## 13. EventOccurrence

UI не должен самостоятельно рассчитывать recurrence.

Domain layer возвращает:

```rust
struct EventOccurrence {
    event_id: EventId,
    recurrence_id: Option<RecurrenceId>,

    date: Date,
    start_time: Option<Time>,
    end_time: Option<Time>,

    title: String,
    importance: Importance,
    tags: Vec<Tag>,

    is_recurring: bool,
}
```

Все экраны работают с `EventOccurrence`.

---

## 14. Исключения из серии

Нужно поддержать:

```text
лекция каждую среду

09.09 лекции нет
16.09 лекция перенесена на 16:20
```

Использовать:

```text
RecurrenceException
├── recurrence_id
├── original_date
├── kind
└── replacement_event_id?
```

Тип:

```text
Cancelled
Modified
```

При изменении occurrence:

```text
This occurrence
Entire series
```

То же правило применяется к:

- времени;
- названию;
- importance;
- тегам.

---

# Заметки и ссылки

## 15. Note

```text
Note
├── id
├── date
├── title?
├── body
├── created_at
└── updated_at
```

На один день допускается несколько заметок.

---

## 16. Link

Ссылка хранится отдельно:

```text
Link
├── id
├── note_id
├── label
├── url
└── created_at
```

Пример:

```text
Лекция       https://...
Репозиторий  https://...
Wiki         https://...
```

Для выбранной ссылки:

```text
o    открыть
y    скопировать URL
```

На Linux открывать браузер через безопасный вызов программы с URL отдельным аргументом.

Не использовать:

```text
sh -c "xdg-open ..."
```

---

# Ближайшие события

## 17. NEXT

Постоянная строка над status bar.

```text
NEXT  •09:00 Лекция · !14:40 Коллок · завтра 15:00 Встреча · +4
```

Она рассчитывается относительно реального текущего времени, независимо от просматриваемой даты.

Показывать столько событий, сколько помещается.

---

## 18. Upcoming

Открывается:

```text
t
```

Пример:

```text
┌─ БЛИЖАЙШИЕ ──────────────────────────────┐
│                                         │
│ СЕГОДНЯ                                 │
│ > • 09:00–10:30 Лекция                 │
│     #универ #лекция                     │
│                                         │
│   ! 14:40–16:10 Коллоквиум             │
│     #универ #матан                      │
│                                         │
│ ЗАВТРА                                  │
│   • 15:00 Встреча                       │
└─────────────────────────────────────────┘
```

Навигация:

```text
j/k       выбор
Enter     перейти к событию
e         редактировать
d         удалить
o         открыть ссылку
y         копировать ссылку
Esc / t   закрыть
```

Сортировка:

```text
time
importance
```

По умолчанию:

```text
time
```

---

# Agenda и поиск

## 19. Agenda

Открывается:

```text
a
```

Это единый интерфейс просмотра и поиска по календарной базе.

```text
┌─ EVENTS ────────────────────────────────────────────────────────┐
│ / #универ                                                      │
├────────────┬───────┬─────┬────────────────────┬────────────────┤
│ DATE       │ TIME  │ PRI │ EVENT              │ TAGS           │
├────────────┼───────┼─────┼────────────────────┼────────────────┤
│ 01.09.2026 │ 14:40 │  •  │ Лекция по ГО       │ универ лекция  │
│ 03.09.2026 │ 12:10 │  !  │ Коллоквиум         │ универ матан   │
└────────────┴───────┴─────┴────────────────────┴────────────────┘
```

---

## 20. Поиск

`/` внутри Agenda переводит фокус в строку поиска.

Искать по:

```text
event.title
event.description
note.title
note.body
link.label
link.url
tag.name
```

Специальный синтаксис тегов:

```text
#универ
```

Несколько тегов:

```text
#универ #лекция
```

означают `AND`.

Комбинированный запрос:

```text
матан #лекция
```

означает:

```text
текст содержит "матан"
AND
есть тег #лекция
```

Парсер запросов сделать отдельным модулем.

В будущем он должен позволить без переделки архитектуры добавить:

```text
importance:high
after:2026-09-01
before:2026-10-01
upcoming
```

---

## 21. Фильтры Agenda

Минимальные:

```text
тип:
events
notes
recurring

importance:
High
Normal
Low
None

date:
all
today
this week
this month
upcoming

tags:
выбранные теги

tag matching:
ALL
ANY
```

Сортировка:

```text
date
importance
title
```

---

# Управление

## 22. Клавиши

```text
h/j/k/l        навигация
стрелки        навигация

Enter          открыть
Esc            назад / закрыть

n              создать
e              редактировать
d              удалить

a              Agenda
t              Upcoming

p              изменить importance

o              открыть ссылку
y              скопировать ссылку

w              Week
D              Day
m              Month
Y              Year

g t            перейти на сегодня
g d            перейти к дате

?              help
q              выход
```

Клавиши преобразуются в семантические действия:

```text
KeyEvent
   ↓
Keymap
   ↓
Action
   ↓
App::update()
```

Пример:

```rust
Action::MoveLeft
Action::OpenAgenda
Action::OpenUpcoming
Action::GoToToday
Action::CreateEvent
Action::ChangeImportance
Action::CopyLink
```

UI не должен содержать бизнес-логику обработки клавиш.

---

# Хранение

## 23. SQLite

База:

```text
~/.local/share/<app>/calendar.db
```

Конфигурация:

```text
~/.config/<app>/config.toml
```

Основные таблицы:

```text
events
recurrences
recurrence_exceptions

notes
links

tags
event_tags
```

Нужны миграции схемы.

---

## 24. Запросы

Не выполнять SQL-запрос отдельно для каждого дня или события.

Плохо:

```text
SELECT events WHERE date = day1
SELECT events WHERE date = day2
...
```

Нужно:

```text
events_between(start, end)
```

Аналогично теги загружать batch-запросом, а не:

```text
event 1 → SELECT tags
event 2 → SELECT tags
...
```

Основные domain/storage API:

```text
events_between(start, end)
upcoming_events(from, limit)

create_event(...)
update_event(...)
delete_event(...)

create_note(...)
update_note(...)
delete_note(...)

create_tag(...)
set_event_tags(...)
search_tags(prefix)

search(query, filters)
```

---

# Архитектура

## 25. Структура проекта

```text
src/
├── main.rs
│
├── app/
│   ├── mod.rs
│   ├── state.rs
│   ├── action.rs
│   └── update.rs
│
├── model/
│   ├── event.rs
│   ├── occurrence.rs
│   ├── note.rs
│   ├── link.rs
│   └── tag.rs
│
├── recurrence/
│   ├── rule.rs
│   ├── expand.rs
│   └── exception.rs
│
├── storage/
│   ├── database.rs
│   ├── events.rs
│   ├── notes.rs
│   ├── tags.rs
│   └── migrations.rs
│
├── search/
│   ├── query.rs
│   ├── parser.rs
│   └── filter.rs
│
├── ui/
│   ├── week.rs
│   ├── day.rs
│   ├── month.rs
│   ├── year.rs
│   ├── agenda.rs
│   ├── upcoming.rs
│   ├── popup.rs
│   └── widgets/
│
├── input/
│   └── keymap.rs
│
├── external/
│   ├── browser.rs
│   └── clipboard.rs
│
└── config/
    └── mod.rs
```

Направление:

```text
Input
  ↓
Action
  ↓
App state / domain
  ↓
Storage
```

Рендер:

```text
App state
  ↓
UI
```

`ui/` не обращается напрямую к SQLite.

---

# Состояние приложения

## 26. AppState

Минимально:

```text
today
selected_date

active_view
focused_pane

selected_event
selected_note
selected_link

popup
input_mode

agenda_state
upcoming_state

status_message
```

`selected_date` едина для Week, Day, Month и Year.

---

# Конфигурация

## 27. config.toml

```toml
week_start = "monday"

[agenda]
next_events = 4

[importance]
none_symbol = " "
low_symbol = "·"
normal_symbol = "•"
high_symbol = "!"

[keys]
open_link = "o"
copy_link = "y"

[ui]
show_week_numbers = false
```

Стили и цвета не должны быть частью бизнес-модели.

---

# Производительность

## 28. Требования

Целевой масштаб:

```text
10 000+ событий
100+ тегов
несколько тегов на событие
многолетняя история
```

Обычная навигация не должна выполнять тяжелые операции.

Render не выполняет:

```text
SQL
filesystem IO
process spawning
```

Recurring events разворачиваются только для требуемого диапазона.

---

# Тесты

## 29. Calendar

Проверить:

```text
31 декабря → 1 января
1 января → 31 декабря
високосный февраль
обычный февраль
месяц с 6 строками
переход между неделями
```

---

## 30. Recurrence

Проверить:

```text
каждый вторник
раз в две недели
несколько weekdays
start/end boundary
cancelled occurrence
modified occurrence
```

Пример:

```text
серия:
каждый вторник 14:40

01.09 есть
08.09 cancelled
15.09 modified → 16:20
22.09 снова 14:40
```

---

## 31. Теги

Проверить:

```text
ML
ml
 ml
```

как один тег.

Проверить:

```text
A → #универ #лекция
B → #универ #экзамен
```

Запрос:

```text
#универ
```

возвращает A и B.

```text
#универ #лекция
```

возвращает A.

---

## 32. Storage

CRUD для:

```text
Event
Note
Link
Tag
Recurrence
RecurrenceException
```

Проверить foreign keys и `ON DELETE CASCADE`.

---

# MVP

## 33. Первая версия

Обязательно:

```text
Week View как стартовый экран
Today indicator
Day View
Month View
Year View

обычные события
importance
теги

weekly recurrence
изменение одного occurrence
удаление одного occurrence

заметки
ссылки
copy/open URL

NEXT
Upcoming
Agenda

поиск по тексту
поиск по #tag
фильтрация

SQLite
Vim navigation
конфиг
```

Пока не реализовывать:

```text
Google Calendar
CalDAV
облачную синхронизацию
аккаунты
сервер
daemon/reminders
attachments
plugin system
сложный Markdown renderer
```

---

## 34. Порядок реализации

```text
1. Date/calendar model
2. Event/Note/Link/Tag models
3. SQLite + migrations
4. AppState + Action/update
5. Week View
6. NEXT
7. Day View
8. CRUD событий
9. Importance
10. Tags
11. Notes + links
12. Weekly recurrence
13. Recurrence exceptions
14. Upcoming
15. Agenda
16. Search/query parser
17. Month View
18. Year View
19. Responsive layout
20. Tests и profiling
```

После каждого этапа приложение должно компилироваться и оставаться запускаемым.
