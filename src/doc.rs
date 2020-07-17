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
    pub member: Option<String>,
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
        // TODO: add crate to name?
        parser::find_item(self.path.join("all.html"), &name)
            .map(|o| o.map(|s| Item::new(name, self.path.join(path::PathBuf::from(s)), None)))
    }

    pub fn find_module(&self, item: &[&str]) -> Option<Item> {
        let path = self
            .path
            .join(path::PathBuf::from(item.join("/")))
            .join("index.html");
        if path.is_file() {
            Some(Item::new(item.join("::"), path, None))
        } else {
            None
        }
    }

    pub fn find_member(&self, item: &[&str]) -> Option<Item> {
        if let Some((last, elements)) = item.split_last() {
            // TODO: error
            let parent = self.find_item(elements).unwrap();
            parent.and_then(|i| i.find_member(last))
        } else {
            None
        }
    }
}

impl Item {
    pub fn new(name: String, path: path::PathBuf, member: Option<String>) -> Self {
        Item { path, member, name }
    }

    pub fn load_doc(&self) -> anyhow::Result<Doc> {
        if let Some(member) = &self.member {
            parser::parse_member_doc(&self.path, &self.name, member)
        } else {
            parser::parse_item_doc(&self.path)
        }
    }

    pub fn find_member(&self, name: &str) -> Option<Item> {
        // TODO: error handling
        if parser::find_member(&self.path, name).unwrap() {
            Some(Item::new(
                self.name.clone(),
                self.path.clone(),
                Some(name.to_owned()),
            ))
        } else {
            None
        }
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
