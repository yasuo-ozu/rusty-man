<!---
SPDX-FileCopyrightText: 2020-2021 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Changelog for rusty-man

## Unreleased

- Add --open option
- Fix html parse method to support latest rustdoc
- Add support for toolchain >= 1.69

## v0.5.0 (2021-10-26)

This minor release adds support for Rust 1.54.0, 1.55.0 and 1.56.0 and bumps
the MSRV to 1.45.0.

- Scroll by page on PageUp/PageDown in tui viewer.
- Bump MSRV to 1.45.0.
- Add support and tests for Rust 1.54.0, 1.55.0 and 1.56.0.

## v0.4.3 (2021-06-19)

This patch release adds support for Rust 1.53.0 and for non-default target
directories.

- Prepare supporting multiple parser backends.
- Read the `$CARGO_TARGET_DIR` and `$CARGO_BUILD_TARGET_DIR` environment
  variables to determine the Cargo target directory.
- Add support and tests for Rust 1.53.0.

## v0.4.2 (2021-06-06)

This patch releases adds support for the new search index format (Rust 1.52.0
and later).

- Add `o` command for opening a documentation item to the tui viewer.
- Add tests for Rust 1.48.0, 1.49.0, 1.50.0, 1.51.0, 1.52.0 and 1.52.1.
- Add support for the new search index format introduced in Rust 1.52.0.

## v0.4.1 (2020-10-11)

This patch release fixes an issue with the pager configuration.

- Use the `LESS` environment variable to set the options for the pager.
- Add the `--pager` option to set the pager for the plain and rich viewers.

## v0.4.0 (2020-10-09)

This minor release introduces a new interactive viewer, tui.  It also adds
syntax highlighting for code in the documentation and support for Rust 1.47.0.

- Remove suffix from duplicate members (e. g. from Deref implementations).
- Use the `merge` crate to merge the command-line arguments and the settings in
  the configuration file.
  - Add `merge` dependency in version 0.1.0.
- Add syntax highlighting for code snippets in the doc comments.
- Add interactive tui viewer that adds support for following links from the
  documentation.
- Add tests for Rust 1.47.0.
- Remove Notable Traits section from definitions.

## v0.3.0 (2020-09-11)

This minor release adds support for Rust 1.46.0 and significantly improves the
test suite.

- Improve handling of different items with same name:
  - Add the item type to the item list if multiple matches are found in the
    search index.
  - Respect the item type when opening the documentation for an item that has
    been found in the search index.
- Refactor test suite:
  - Store the documentation generated by all supported rustdoc versions in the
    `tests/html` directory.
  - Use one snapshot per test case and rustdoc version.
- Refactor member lookup for compatibility with Rust 1.46.0.
- Add tests for Rust 1.46.0.
- Improve test suite:
  - Add test for `Parser::find_member`.
  - Add test for `Index::find`.

## v0.2.0 (2020-08-11)

This minor release adds support for syntax highlighting of code snippets and
for configuration files.

- Add syntax highlighting:
  - Add syntax highlighting using `syntect` for code snippets displayed with
    the rich text viewer.
  - Add the `--no-syntax-highlight` option to disable syntax highlighting.
  - Add the `--theme [theme]` option to select the syntax highlighting theme.
- Add support for configuration files:
  - Load the `config.toml` file from the config directory according to the XDG
    Base Directory Specification `${XDG_CONFIG_HOME}/rusty-man/config.toml`,
    where `${XDG_CONFIG_HOME}` defaults to `${HOME}/.config`.  The
    configuration file can be used to set defaults for the command-line
    options.
  - Add the `--config-file [file]` option to set a custom configuration file.
- Add the `--width [width]` option to set a fixed output width and the
  `--max-width [max]` option to set the maximum output width.
- Improve line break rendering when displaying code.
- Add integration test suite.

## v0.1.3 (2020-07-28)

This patch release adds support for documentation generated with Rust 1.45.0
and fixes some minor bugs in the documentation parser.  It also adds the
documentation downloaded using rustup to the default sources.

- Use `rustc --print sysroot` to determine the Rust installation directory
  instead of always using `/usr`.
- Improve the documentation parser:
  - Fix the definition of methods to only contain the actual definition.
  - Remove spurious members in module documentation.
  - Show the definition for constants and typedefs.
  - Fix group and ID for typdef items.
  - Extract the description of module items as HTML instead of plain text.
  - Sort implementations alphabetically.
  - Fix list of methods and trait implementations for Rust 1.45.

## v0.1.2 (2020-07-25)

This patch release adds basic logging output and a new `-e`/`--examples` option
to extract only the examples from the documentation.  It also fixes a bug when
displaying the documentation for a function.

- Add basic logging using `env_logger` that can be enabled by setting the
  environment variable `RUST_LOG=info`.
  - Add `env_logger` dependency in version 0.7.1.
  - Add `log` dependency in version 0.4.11.
- Show the definition for global functions.
- Add the `-e`/`--examples` option to only show the examples instead of opening
  the full documentation for an item.

## v0.1.1 (2020-07-24)

This patch release fixes some minor issues with the documentation displayed on
crates.io.

## v0.1.0 (2020-07-24)

Initial release with support for directory sources and including viewers for
plain and rich text.
