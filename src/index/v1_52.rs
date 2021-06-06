// SPDX-FileCopyrightText: 2021 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! Search index format as of Rust 1.52.0.
//!
//! This module contains data structures specific to the search index format introduced with Rust
//! 1.52.0 (commit [3934dd1b3e7514959202de6ca0d2636bcae21830][]).
//!
//! [3934dd1b3e7514959202de6ca0d2636bcae21830]: https://github.com/rust-lang/rust/commit/3934dd1b3e7514959202de6ca0d2636bcae21830

#[derive(Debug, Default, PartialEq, serde::Deserialize)]
pub struct CrateData {
    #[serde(rename = "t")]
    item_types: Vec<super::ItemType>,
    #[serde(rename = "n")]
    item_names: Vec<String>,
    #[serde(rename = "q")]
    item_paths: Vec<String>,
    #[serde(rename = "d")]
    item_descs: Vec<String>,
    #[serde(rename = "i")]
    item_parents: Vec<usize>,
    #[serde(rename = "p")]
    paths: Vec<(usize, String)>,
}

impl From<CrateData> for super::CrateData {
    fn from(data: CrateData) -> Self {
        let items = data
            .item_types
            .into_iter()
            .zip(data.item_names.into_iter())
            .zip(data.item_paths.into_iter())
            .zip(data.item_descs.into_iter())
            .zip(data.item_parents.into_iter())
            .map(|((((ty, name), path), desc), parent)| super::ItemData {
                ty,
                name,
                path,
                desc,
                parent: match parent {
                    0 => None,
                    parent => Some(parent - 1),
                },
                _ignored: Default::default(),
            })
            .collect();
        Self {
            items,
            paths: data.paths,
        }
    }
}
