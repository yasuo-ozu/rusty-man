// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::path;

use crate::parser;

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

    pub fn find_item(&self, item: &[&str]) -> anyhow::Result<Option<Item>> {
        let name = item.join("::");
        parser::find_item(self.path.join("all.html"), &name)
            .map(|o| o.map(|s| Item::new(name, self.path.join(path::PathBuf::from(s)))))
    }

    pub fn find_module(&self, item: &[&str]) -> Option<Item> {
        let path = self
            .path
            .join(path::PathBuf::from(item.join("/")))
            .join("index.html");
        if path.is_file() {
            Some(Item::new(item.join("::"), path))
        } else {
            None
        }
    }
}

impl Item {
    pub fn new(name: String, path: path::PathBuf) -> Self {
        Item { path, name }
    }

    pub fn load_doc(&self) -> anyhow::Result<Doc> {
        parser::parse_doc(&self.path)
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
