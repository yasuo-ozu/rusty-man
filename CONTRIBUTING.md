<!---
SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Contributing to rusty-man

## Project infrastructure

rusty-man is hosted on [sourcehut.org](https://sourcehut.org) and uses these
services:
- [git.sr.ht/~ireas/rusty-man][git]: Git repository
- [todo.sr.ht/~ireas/rusty-man][todo]: issue tracker
- [lists.sr.ht/~ireas/rusty-man-dev][ml]: mailing list
- [builds.sr.ht/~ireas/rusty-man][ci]: build server for continuous integration

## How to contribute

### Writing code

Have a look at the [issues][todo] in rusty-man’s issue tracker, especially
those with the label [good first issue][], to find an issue to work on.

### Writing documentation

You can help by proofreading and extending the documentation in the readme file
and the contributing and installation guides.  Also, rusty-man is lacking a man
page and a usage guide – contributions are welcome!

### Testing

If you are using rusty-man and are encountering any issues, please let me know.
I’m especially interested in reports from other operating systems than Linux.
You can also help by writing unit tests, especially for the HTML documentation
parser.

## Submitting patches

Please submit patches by sending a mail to the mailing list
[~ireas/rusty-man-dev@lists.sr.ht][list].  There are three ways to do that:

1. Use [`git send-email`][] to send your patches.  If you are not familiar with
   the `git send-email` workflow, have a look at [this step-by-step
   guide][guide] or [contact me][] for more information.
2. Or push your changes to a public repository, for example hosted on your own
   Git server, sourcehut.org, Gitlab or GitHub, and use [`git request-pull`][]
   to send a pull request to the mailing list.
3. If options one and two don’t work for you, just use your mail client to send
   a mail with a link to your changes and a short description to the mailing
   list.

## Testing and checking the code

- rusty-man currently has very few unit tests.  You can execute them using
  `cargo test --bins`, but make sure to generate rusty-man’s documentation with
  `cargo doc` before running the tests!
- rusty-man has an integration test suite that uses [`insta`][] for snapshot
  testing.  The test suite takes care of generating the required documentation
  so you don’t have to run `cargo doc` manually.  Use `cargo test` to execute
  the tests that should work on all supported Rust versions.  Use `cargo test
  -- --ignored` to run the tests that only work with the latest stable Rust
  version.
- Use `cargo fmt` for code formatting.
- Fix all warnings and errors reported by `clippy`.

[git]: https://git.sr.ht/~ireas/rusty-man
[todo]: https://todo.sr.ht/~ireas/rusty-man
[ml]: https://lists.sr.ht/~ireas/rusty-man-dev
[ci]: https://builds.sr.ht/~ireas/rusty-man

[good first issue]: https://todo.sr.ht/~ireas/rusty-man?search=label:%22good%20first%20issue%22%20status%3Aopen

[list]: mailto:~ireas/rusty-man-dev@lists.sr.ht
[`git send-email`]: https://git-scm.com/docs/git-send-email
[`git request-pull`]: https://git-scm.com/docs/git-request-pull
[guide]: https://git-send-email.io
[contact me]: mailto:robin.krahl@ireas.org
[`insta`]: https://lib.rs/crates/insta
