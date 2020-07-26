<!---
SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Changelog for rusty-man

## Unreleased

- Use `rustc --print sysroot` to determine the Rust installation directory
  instead of always using `/usr`.
- Improve the documentation parser:
  - Fix the definition of methods to only contain the actual definition.

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
