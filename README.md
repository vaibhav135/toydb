# toydb

`toydb` is a small, read-only SQLite file reader written in Rust. It is a
learning project for understanding how SQLite stores schemas, records, pages,
cells, and B-trees on disk.

ToyDB reads an existing SQLite database directly; it does not use SQLite as
its query engine.

## Current status

The current implementation supports a narrow, educational query path:

- SQLite database-header and `sqlite_schema` parsing
- Table and index B-tree traversal
- Interior and leaf B-tree pages
- Overflow-page chains for large cell payloads
- SQLite record serial types, including integer, text, and real values
- Basic table scans and projections
- Simple indexed equality lookups for supported indexes
- Interactive `.dbinfo` and `.tables` commands
- `.quit` and `.exit` from the interactive prompt

The indexing path currently works best for simple equality lookups on existing
rowid tables, especially text values, for example:

```sql
select * from test where name='Ava Martinez';
```

## Running

ToyDB expects an existing SQLite database file:

```bash
cargo run --release -- test.db
```

The interactive prompt uses `$`:

```text
$ .dbinfo
$ .tables
$ select * from test;
$ select * from test where name='Ava Martinez';
$ .quit
```

The repository also contains SQLite fixtures under `testdb/` for exploring
different page, record, overflow, and schema layouts.

## Indexed lookup

For a supported indexed lookup, ToyDB follows this path:

1. Read the schema and locate the table and its index.
2. Search the index B-tree for the indexed value.
3. Use the resulting rowid to search the table B-tree.
4. Decode and print the matching table row.

The CLI output below is an example of the indexed lookup path working:

```text
$ select * from test where name='Ava Martinez';

name | id | rollno | age | balance |

Ava Martinez |  | 20260001 | 28 | 987.35

Total rows: 0
Total execution time: 1.351552ms
```

The displayed row confirms the lookup, although the current executor does not
increment `Total rows` for the indexed `SearchByID` path. That count is a known
small correctness issue.

## Supported SQL surface

The parser recognizes `SELECT` and `CREATE` statements, but the executor
currently executes only `SELECT`. `CREATE` is used when reading schema
definitions; ToyDB does not create or modify database files.

The practical query shape is currently close to:

```sql
select * from table_name;
select * from table_name where indexed_column='value';
```

Current limitations include:

- `INSERT`, `UPDATE`, and `DELETE` are not supported.
- Only one simple equality `WHERE` predicate is supported.
- `FROM` parsing is case-sensitive and currently expects lowercase `from`.
- Joins, expressions, aggregates, ordering, grouping, limits, offsets, and
  subqueries are not implemented.
- WITHOUT ROWID tables, views, triggers, and some automatically generated
  indexes are not supported by the query path.
- Malformed or unsupported database layouts may produce an error or panic;
  this is an experimental reader rather than a production database engine.

## Project layout

```text
src/
├── btree.rs              B-tree search helpers and public exports
├── btree/
│   ├── child.rs          Table/index child-page traversal
│   ├── common.rs         Shared B-tree payload types
│   └── root.rs           Database header and schema-root handling
├── cell.rs               SQLite cell parsing
├── commands.rs           `.dbinfo` and `.tables`
├── custom_error.rs       Page/parser error types
├── file.rs               Database-file reads and initialization
├── file/
│   └── enums.rs          File and encoding enums
├── page.rs               Page headers, cells, and overflow chains
├── query.rs              Query module exports
├── query/
│   ├── common.rs         Query data structures and enums
│   ├── executor.rs       Query execution and row retrieval
│   └── parser.rs         SQL parsing
├── record_type.rs        SQLite record values and conversions
├── schema.rs             `sqlite_schema` parsing
├── utils.rs              Byte parsing and input helpers
└── main.rs               CLI entry point
```

## Learning references

- [SQLite file format](https://www.sqlite.org/fileformat2.html)
- [SQLite B-tree pages](https://www.sqlite.org/fileformat.html#b_tree_pages)
- [SQLite Internals: B-trees](https://fly.io/blog/sqlite-internals-btree/)
- [Exploring SQLite Internals](https://www.bswanson.dev/blog/exploring-sqlite-internals/)
- [SQLite representation visualizer](https://torymur.github.io/sqlite-repr)
- [SQLite internals visual guide](https://sqlite-internal.pages.dev)
- [SQLite limits](https://sqlite.org/limits.html)

This project is for educational purposes.
