// SPDX-FileCopyrightText: 2021 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! Search index format as of Rust 1.69.0.
//!
//! This module contains data structures specific to the search index format introduced with Rust
//! 1.69.0.
//! This change is introduced in [PR #108013](https://github.com/rust-lang/rust/pull/108013), [PR #107629](https://github.com/rust-lang/rust/pull/107629)

use std::collections::HashMap;

#[derive(Debug, Default, PartialEq, serde::Deserialize)]
pub struct CrateData {
    #[serde(rename = "t")]
    item_types: String,
    #[serde(rename = "n")]
    item_names: Vec<String>,
    #[serde(rename = "q")]
    item_paths: ItemPaths,
    #[serde(rename = "d")]
    item_descs: Vec<String>,
    #[serde(rename = "i")]
    item_parents: Vec<usize>,
    #[serde(rename = "p")]
    paths: Vec<(usize, String)>,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(untagged)]
pub enum ItemPaths {
    Raw(Vec<String>),
    Indexed(Vec<(usize, String)>),
}

impl Default for ItemPaths {
    fn default() -> Self {
        Self::Indexed(Vec::new())
    }
}

impl From<CrateData> for super::CrateData {
    fn from(data: CrateData) -> Self {
        use core::convert::TryFrom;
        let path_map: HashMap<usize, String> = match &data.item_paths {
            ItemPaths::Raw(v) => v
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, s)| s.len() > 0)
                .collect(),
            ItemPaths::Indexed(v) => v.iter().cloned().collect(),
        };
        let items = data
            .item_types
            .chars()
            .map(|c| crate::doc::ItemType::try_from(c).unwrap().into())
            .zip(data.item_names.into_iter())
            .zip(data.item_descs.into_iter())
            .zip(data.item_parents.into_iter())
            .enumerate()
            .map(|(index, (((ty, name), desc), parent))| super::ItemData {
                ty,
                name,
                path: path_map.get(&index).cloned().unwrap_or(String::new()),
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
