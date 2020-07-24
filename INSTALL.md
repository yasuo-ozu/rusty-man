<!---
SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Installing rusty-man

## Requirements

To compile rusty-man, you need Rust 1.40 or later.

## Installing from source

1. Clone the rusty-man Git repository:
   ```
   $ git clone https://git.sr.ht/~ireas/rusty-man && cd rusty-man
   ```
2. Optional:  Checkout the latest release:
   ```
   $ git checkout v0.1.0
   ```
3. Optional:  Verify the signature of the latest commit:
   ```
   $ curl -s "https://pgp.ireas.org/0x6D533958F070C57C.txt" | gpg --import
   $ gpg verify-commit HEAD
   ```
4. Compile rusty-man:
   ```
   $ cargo build --release
   ```
5. Optional:  Install the rusty-man binary:
   ```
   $ sudo cp ./target/release/rusty-man /usr/local/bin/rusty-man
   ```

## Installing from crates.io

```
cargo install rusty-man
```
