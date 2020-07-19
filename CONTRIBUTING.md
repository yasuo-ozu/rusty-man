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

## Submitting patches

There are two ways to submit patches for rusty-man:

1. Use `git send-email` to send your patches to the mailing list
   [~ireas/rusty-man-dev@lists.sr.ht][list].  If you are not familiar with the
   `git send-email` workflow, have a look at [this step-by-step
   guide](https://git-send-email.io) and feel free to [contact
   me](mailto:robin.krahl@ireas.org) for more information.
2. Push your changes to a public repository, for example hosted on your own Git
   server, sourcehut.org, Gitlab or GitHub, and use `git request-pull` to send
   a pull request to the mailing list [~ireas/rusty-man-dev@lists.sr.ht][list].

[git]: https://git.sr.ht/~ireas/rusty-man
[todo]: https://todo.sr.ht/~ireas/rusty-man
[ml]: https://lists.sr.ht/~ireas/rusty-man-dev
[ci]: https://builds.sr.ht/~ireas/rusty-man
[list]: mailto:~ireas/rusty-man-dev@lists.sr.ht
