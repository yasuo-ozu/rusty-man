// SPDX-FileCopyrightText: 2021 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! Search index format as of Rust 1.44.0.
//!
//! This module contains data structures specific to the search index format introduced with Rust
//! 1.44.0 (commit [b4fb3069ce82f61f84a9487d17fb96389d55126a][]).
//!
//! [b4fb3069ce82f61f84a9487d17fb96389d55126a]: https://github.com/rust-lang/rust/commit/b4fb3069ce82f61f84a9487d17fb96389d55126a

#[derive(Debug, Default, PartialEq, serde::Deserialize)]
pub struct CrateData {
    #[serde(rename = "i")]
    items: Vec<super::ItemData>,
    #[serde(rename = "p")]
    paths: Vec<(usize, String)>,
}

impl From<CrateData> for super::CrateData {
    fn from(data: CrateData) -> Self {
        Self {
            items: data.items,
            paths: data.paths,
        }
    }
}
