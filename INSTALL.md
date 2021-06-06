<!---
SPDX-FileCopyrightText: 2020-2021 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Installing rusty-man

## Installing a package

rusty-man packages are available for these distributions:
- Arch Linux: [`rusty-man`][pkg-aur] in the Arch User Repository

[pkg-aur]: https://aur.archlinux.org/packages/rusty-man/

## Installing from source

### Build Requirements

To compile rusty-man, you need Rust 1.40 or later.

### Installing from Git

1. Clone the rusty-man Git repository:
   ```
   $ git clone https://git.sr.ht/~ireas/rusty-man && cd rusty-man
   ```
2. Optional:  Checkout the latest release:
   ```
   $ git checkout v0.4.2
   ```
3. Optional:  Verify the signature of the latest commit:
   ```
   $ curl -s "https://pgp.ireas.org/0x6D533958F070C57C.txt" | gpg --import
   $ git verify-commit HEAD
   ```
4. Compile rusty-man:
   ```
   $ cargo build --release --locked
   ```
5. Optional:  Install the rusty-man binary:
   ```
   $ sudo cp ./target/release/rusty-man /usr/local/bin/rusty-man
   ```

### Installing from a tarball

1. Download the tarball for the latest rusty-man release (see the [release
   list][]) and optionally its signature:
   ```
   $ curl -OJ "https://git.sr.ht/~ireas/rusty-man/archive/v0.4.2.tar.gz"
   ```
2. Optional:  Download and verify the signature of the tarball:
   ```
   $ curl -O "https://git.sr.ht/~ireas/rusty-man/refs/v0.4.2/rusty-man-v0.4.2.tar.gz.asc"
   $ curl -s "https://pgp.ireas.org/0x6D533958F070C57C.txt" | gpg --import
   $ gpg --verify rusty-man-v0.4.2.tar.gz.asc
   ```
3. Extract the tarball:
   ```
   $ tar -xf rusty-man-v0.4.2.tar.gz
   $ cd rusty-man-v0.4.2
   ```
4. Compile rusty-man:
   ```
   $ cargo build --release --locked
   ```
5. Optional:  Install the rusty-man binary:
   ```
   $ sudo cp ./target/release/rusty-man /usr/local/bin/rusty-man
   ```

[release list]: https://git.sr.ht/~ireas/rusty-man/refs

### Installing from crates.io

```
cargo install rusty-man --locked
```

You can omit the `--locked` option to use the latest dependency versions
available.  Note that this might cause issues if a dependency breaks semantic
versioning.
