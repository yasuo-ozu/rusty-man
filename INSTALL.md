<!---
SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Installing rusty-man

## Installing a package

rusty-man packages are available for these distributions:
- Arch Linux: [`rusty-man`][pkg-aur] in the Arch User Repository

[pkg-aur]: https://aur.archlinux.org/packages/rusty-man/

## Installing from source

### Build Requirements

To compile rusty-man, you need Rust 1.40 or later.  The `tui` backend requires
the ncurses library in the default search path and a C compiler.

If you don’t want to use ncurses, you can select another [`cursive`][] backend
by replacing this line in `Cargo.toml`:

```toml
cursive = "0.15.0"
```

with this:

```toml
cursive = { version = "0.15.0", default-features = false, feature = ["termion-backend"] }
```

[`cursive`]: https://lib.rs/cursive

### Installing from Git

1. Clone the rusty-man Git repository:
   ```
   $ git clone https://git.sr.ht/~ireas/rusty-man && cd rusty-man
   ```
2. Optional:  Checkout the latest release:
   ```
   $ git checkout v0.3.0
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
   $ curl "https://git.sr.ht/~ireas/rusty-man/archive/v0.3.0.tar.gz" \
         --output rusty-man-v0.3.0.tar.gz
   ```
2. Optional:  Download and verify the signature of the tarball:
   ```
   $ curl "https://git.sr.ht/~ireas/rusty-man/refs/v0.3.0/v0.3.0.tar.gz.asc" \
         --output rusty-man-v0.3.0.tar.gz.asc
   $ curl -s "https://pgp.ireas.org/0x6D533958F070C57C.txt" | gpg --import
   $ gpg --verify rusty-man-v0.3.0.tar.gz.asc
   ```
3. Extract the tarball:
   ```
   $ tar -xf rusty-man-v0.3.0.tar.gz
   $ cd rusty-man-v0.3.0
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
cargo install rusty-man
```
