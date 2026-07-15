# Changelog

All notable changes to `partial_config` and its companion derive crate
`partial_config_derive` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and both crates adhere to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The two crates are versioned independently; each entry notes the versions it covers.
Git tags are `partial_config-vX.Y.Z` and `partial_config_derive-vX.Y.Z`.

## [Unreleased]

## [0.7.3] — 2026-07-15

`partial_config_derive` unchanged at 0.5.2.

### Added

- **`Redacted<T>`** — a wrapper marking a value as sensitive. Its `Debug` and `Display`
  render `[redacted]` and never reveal the inner value, so a secret (a DSN password, an
  API token, a private key) cannot escape through a log line, an error, a `panic!`, or a
  derived `Debug` on a struct containing it. It is a drop-in configuration field —
  `FromStr` and (with `serde`) `Deserialize` delegate to the inner type, so it sources
  from the environment, CLI, or a file exactly as the bare `T` would. The value is
  reachable only through `expose_secret()` / `into_inner()`; there is deliberately no
  `Deref`, `AsRef`, or `Serialize`.

  This is the intended way to keep secrets out of the configuration log that `build()`
  emits: wrap the field, and it redacts itself wherever it is printed, rather than the
  crate trying to guess which fields are sensitive.


`partial_config_derive` 0.5.2.

### Added

- **`#[env(skip)]` on `EnvSourced`** (`partial_config_derive`). A field marked
  `#[env(skip)]` opts out of environment sourcing entirely: the generated environment
  source leaves it `None`, so it is supplied only by the CLI and default layers. This is
  the counterpart to a value that is *operator intent* rather than *configuration* — a
  `--from` expressed at the moment a command runs, which must never pick up a stray
  same-named variable from the surrounding shell. Previously every field of an
  `EnvSourced` struct was required to declare at least one `#[env(...)]` variable, so
  such a field could not be expressed at all.

  `skip` cannot be combined with environment-variable names on the same field — a field
  is either sourced from the environment or explicitly opts out, and mixing the two is a
  compile error with a help note.

## [0.7.1] — 2024-11-14

`partial_config_derive` 0.5.1.

### Added

- `Option<T>` fields are now supported by `EnvSourced`: an absent variable yields `None`,
  and a present-but-unparseable one is reported as `ParseFieldError` rather than silently
  dropped.

### Changed

- Bumped `proc-macro2`/proc-macro tooling versions; documentation updates.

## [0.7.0] — 2024-10-15

`partial_config_derive` 0.5.0.

### Changed

- Stabilised the public API. Switched the derive macros to `proc_macro_error2` for
  clearer, multi-error diagnostics, and improved the messages emitted for malformed
  configuration structs.

### Fixed

- `Error` no longer requires `Send`/`Sync` where it should not.

## [0.6.0] — 2024-04-26

`partial_config_derive` 0.3.0.

### Added

- `impl Default` for the generated environment source.
- Extensive error handling: missing required fields are collected and reported together
  rather than one at a time.
