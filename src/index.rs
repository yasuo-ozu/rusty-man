// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! Search index for a documentation source.
//!
//! The search index is read from the `search-index.js` file generated by rustdoc.  It contains a
//! list of items groupd by their crate.
//!
//! For details on the format of the search index, see the `html/render.rs` file in `librustdoc`.
//! Note that the format of the search index changed in April 2020 with commit
//! b4fb3069ce82f61f84a9487d17fb96389d55126a.  We only support the new format as the old format is
//! much harder to parse.
//!
//! For details on the generation of the search index, see the `html/render/cache.rs` file in
//! `librustdoc`.

use std::collections;
use std::fmt;
use std::fs;
use std::io;
use std::path;

#[derive(Debug)]
pub struct Index {
    data: Data,
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct IndexItem {
    pub path: String,
    pub name: String,
    pub description: String,
}

impl fmt::Display for IndexItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.description.is_empty() {
            write!(f, "{}::{}", &self.path, &self.name)
        } else {
            write!(f, "{}::{}: {}", &self.path, &self.name, &self.description)
        }
    }
}

#[derive(Debug, Default, PartialEq, serde::Deserialize)]
#[serde(transparent)]
struct Data {
    crates: collections::HashMap<String, CrateData>,
}

#[derive(Debug, Default, PartialEq, serde::Deserialize)]
struct CrateData {
    #[serde(rename = "i")]
    items: Vec<ItemData>,
    #[serde(rename = "p")]
    paths: Vec<(usize, String)>,
}

#[derive(Debug, Default, PartialEq, serde_tuple::Deserialize_tuple)]
struct ItemData {
    ty: usize,
    name: String,
    path: String,
    desc: String,
    parent: Option<usize>,
    _ignored: serde_json::Value,
}

impl Index {
    pub fn load(path: impl AsRef<path::Path>) -> anyhow::Result<Option<Self>> {
        use std::io::BufRead;

        anyhow::ensure!(
            path.as_ref().is_file(),
            "Search index '{}' must be a file",
            path.as_ref().display()
        );

        let mut json: Option<String> = None;
        let mut finished = false;

        for line in io::BufReader::new(fs::File::open(path)?).lines() {
            let line = line?;
            if let Some(json) = &mut json {
                if line == "}');" {
                    json.push_str("}");
                    finished = true;
                    break;
                } else {
                    json.push_str(line.trim_end_matches('\\'));
                }
            } else if line == "var searchIndex = JSON.parse('{\\" {
                json = Some(String::from("{"));
            }
        }

        if let Some(json) = json {
            if finished {
                use anyhow::Context;
                let json = json.replace("\\'", "'");
                let data: Data =
                    serde_json::from_str(&json).context("Could not parse search index")?;

                Ok(Some(Index { data }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn find(&self, keyword: &str) -> Vec<IndexItem> {
        let keyword = format!("::{}", keyword);
        let mut matches: Vec<IndexItem> = Vec::new();
        for (krate, data) in &self.data.crates {
            let mut path = krate;
            for item in &data.items {
                path = if item.path.is_empty() {
                    path
                } else {
                    &item.path
                };

                if item.ty == 16 {
                    // Skip associated types (== item type 16)
                    continue;
                }

                let full_path = match item.parent {
                    Some(idx) => {
                        let parent = &data.paths[idx].1;
                        format!("{}::{}", path, parent)
                    }
                    None => path.to_owned(),
                };
                let full_name = format!("{}::{}", &full_path, &item.name);
                if full_name.ends_with(&keyword) {
                    matches.push(IndexItem {
                        name: item.name.clone(),
                        path: full_path,
                        description: item.desc.clone(),
                    });
                }
            }
        }
        matches.sort_unstable();
        matches.dedup();
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::{CrateData, Data, ItemData};

    #[test]
    fn test_empty() {
        let expected: Data = Default::default();
        let actual: Data = serde_json::from_str("{}").unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_empty_crate() {
        let mut expected: Data = Default::default();
        expected
            .crates
            .insert("test".to_owned(), Default::default());
        let actual: Data = serde_json::from_str("{\"test\": {\"i\": [], \"p\": []}}").unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_one_item() {
        let mut expected: Data = Default::default();
        let mut krate: CrateData = Default::default();
        let mut item: ItemData = Default::default();
        item.name = "name".to_owned();
        item.path = "path".to_owned();
        item.desc = "desc".to_owned();
        krate.items.push(item);
        expected.crates.insert("test".to_owned(), krate);
        let actual: Data = serde_json::from_str(
            "{\"test\": {\"i\": [[0, \"name\", \"path\", \"desc\", null, null]], \"p\": []}}",
        )
        .unwrap();
        assert_eq!(expected, actual);
    }
}
