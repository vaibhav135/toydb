# kitchen_sink.db — every SQLite file-format concept in one file

Built 2026-07-11, page_size **512**, 101 pages, UTF-8, `PRAGMA integrity_check` = ok.
Every claim below verified via `dbstat`, SQL queries, and raw `hexdump` of type bytes.

## Concept map → where to find it

| Concept | Where | Proof |
|---|---|---|
| Multi-page schema, **page 1 is INTERIOR** | `sqlite_schema`: 1 internal + 3 leaf pages | byte at offset 100 = `05` |
| Interior TABLE pages | `many` (root page 23): 1 internal + 30 leaves | byte at (23-1)*512 = `05` |
| Interior INDEX pages | `idx_many_val` (root 55): 3 internal + 30 leaves | dbstat |
| Overflow chains | `big_rows` (root 5): 15 overflow pages | dbstat |
| Freelist | 10 free pages (dropped table `junk`) | `PRAGMA freelist_count` = 10, first free page 93 |
| Freeblocks inside pages | `DELETE FROM many WHERE n % 97 = 0` (10 rows) | — |
| View (rootpage = 0) | `v_small_many` | schema row |
| Trigger (rootpage = 0) | `trg_many_insert` → writes into `audit_log` | audit_log has 1 row: `inserted row 1001` |
| Auto-index, **sql = NULL** | `sqlite_autoindex_users_1` (root 4), from `users.email UNIQUE` | schema row |
| INTEGER PRIMARY KEY = rowid alias | `users.id`, `many.n`, `big_rows.id` → stored as **NULL** (stype 0) in the record; real value is the cell's rowid | §2.3 |
| WITHOUT ROWID table (stored as index b-tree!) | `kv_store` (root 88) | type byte = `0a` (leaf INDEX) |
| All integer serial-type widths, signed | `mixed_types` (root 2, single leaf) | rows below |
| Serial types 8/9 (constants 0/1) | `mixed_types` rows 1-2 (schema format 4) | — |
| Floats, blobs, text, NULLs | `mixed_types` | rows below |

## Rootpages (from sqlite_schema)

```
mixed_types=2  users=3  sqlite_autoindex_users_1=4  big_rows=5
many=23  idx_many_val=55  kv_store=88  audit_log=89
v_small_many=0  trg_many_insert=0
```

## Ground truth for your parser

### mixed_types — 14 rows (a INTEGER, b TEXT, c REAL, d BLOB, e)
Exercises every integer width incl. sign extension:
```
(0, 'stype8-zero', 0.0, X'00', NULL)          -- a uses stype 8
(1, 'stype9-one', 1.5, X'DEADBEEF', 'e-text') -- a uses stype 9
(127, 'i8-max', ...)   (-128, 'i8-min', ...)          -- 1 byte
(32767, ...)           (-32768, ...)                   -- 2 bytes
(8388607, ...)         (-8388608, ...)                 -- 3 bytes (sign-extend!)
(2147483647, ...)      (-2147483648, ...)              -- 4 bytes
(140737488355327, ...) (-140737488355328, ...)         -- 6 bytes (sign-extend!)
(9223372036854775807)  (-9223372036854775808)          -- 8 bytes
```

### big_rows — 3 rows, overflow ground truth
- id=1: doc TEXT, **length 6009**, starts `ABABABAB`, ends `ENDOFTEXT`, bin NULL
- id=2: doc NULL, bin BLOB **2000 bytes**, hex head `43444344`, hex tail `43444344`
  (whole blob is ASCII "CD" repeated)
- id=3: `('tiny doc, no overflow', X'0102030405')` — control, fits in page

### many — **991 rows** (n INTEGER PRIMARY KEY, val TEXT)
- was 1..1001 with val = 'val-'||n, then rows where n % 97 = 0 deleted
  (97, 194, 291, 388, 485, 582, 679, 776, 873, 970)
- so: n=1 → 'val-1' ... n=1001 → 'val-1001', minus those 10
- full scan in rowid order must yield 991 rows, ascending n

### users — 4 rows (id INTEGER PRIMARY KEY, email TEXT UNIQUE, name TEXT)
```
(1,'a@x.com','alice') (2,'b@x.com','bob') (7,'g@x.com','grace') (100,'z@x.com','zed')
```
NOTE: the `id` column decodes as NULL in the record — substitute the rowid.

### kv_store (WITHOUT ROWID) — 3 rows
`('alpha',1) ('beta',2) ('gamma',3)` — lives in an INDEX b-tree; your table-read
path is NOT expected to handle this one. It exists to be recognized, not parsed.

### audit_log — 1 row: `'inserted row 1001'`

## Handy commands

```sh
sqlite3 testdb/kitchen_sink.db "SELECT name,pagetype,count(*) FROM dbstat GROUP BY 1,2"
hexdump -C -s $(( (N-1) * 512 )) -n 64 testdb/kitchen_sink.db   # page N header
```
