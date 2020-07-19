<!---
SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Installing rusty-man

## Installing from source

To install rusty-man from source, you need Rust 1.40 or later.

1. Clone the rusty-man Git repository:
   ```
   $ git clone https://git.sr.ht/~ireas/rusty-man && cd rusty-man
   ```
2. Optional:  Verify the signature of the latest commit:
   ```
   $ curl -s "https://pgp.ireas.org/0x6D533958F070C57C.txt" | gpg --import
   $ gpg verify-commit HEAD
   ```
3. Compile rusty-man:
   ```
   $ cargo build --release
   ```
4. Optional:  Install the rusty-man binary:
   ```
   $ sudo cp ./target/release/rusty-man /usr/local/bin/rusty-man
   ```
