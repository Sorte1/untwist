# The JSON report

`untwist --report json` prints one JSON object on stdout. [`report.schema.json`](report.schema.json)
is its JSON Schema (draft 2020-12), and `crates/untwist-cli/tests/report_schema.rs` checks the
report of several runs against it: a decoded input, a run with `--types`, a budget and parse
errors, a bundle written with `-o`, and a run with a library collection.

## Version

Every report carries `schema_version`, an integer, now `1`.

- A change that only adds a field, or adds a value to a string that is not an enum in the schema,
  keeps the version. Read reports with a parser that ignores fields it does not know.
- Removing or renaming a field, changing its type or its meaning, making an optional field
  required or the other way round, or adding a value to an enum (`origin`, `reason`, `mode`)
  raises the version.
- The schema in the repository always describes the current report exactly: it rejects unknown
  fields, so a field added without the schema fails the test. A report of an older version is
  checked against the schema of the release that wrote it.
- Optional fields are left out rather than written as `null` or empty; `budget.max_memory` and
  `budget.timeout` are `null` when there is no limit.
- Times (`*_ms`, `budget.timeout` in seconds) and `stats` vary from run to run. Pass and stage
  names, note texts and `stop` reasons are not part of the schema and may change in any release.

Builds with the `test-hooks` feature also write `refusal_span` and a per-stage `work` list; release
builds never do.
