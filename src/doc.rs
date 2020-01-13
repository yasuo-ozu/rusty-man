// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::path;

#[derive(Clone, Debug, PartialEq)]
pub struct Crate {
    pub name: String,
    pub path: path::PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub path: path::PathBuf,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub title: String,
    pub description: Option<String>,
    pub definition: Option<String>,
}

impl Crate {
    pub fn new(name: String, path: path::PathBuf) -> Self {
        Crate { name, path }
    }

    pub fn find_item(&self, _item: &[&str]) -> anyhow::Result<Option<Item>> {
        Ok(None)
    }
}

impl Item {
    pub fn load_doc(&self) -> anyhow::Result<Doc> {
        Ok(Doc::new(self.name.clone()))
    }
}

impl Doc {
    pub fn new(title: String) -> Self {
        Self {
            title,
            ..Default::default()
        }
    }
}
