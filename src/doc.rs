// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::convert;
use std::fmt;
use std::ops;
use std::path;
use std::str;

use crate::parser;

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Name {
    // s[..first_end] == first
    // s[last_name_start..] == last_name
    s: String,
    first_end: usize,
    last_start: usize,
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Fqn(Name);

#[derive(Clone, Debug, PartialEq)]
pub struct Crate {
    pub name: String,
    pub path: path::PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub path: path::PathBuf,
    pub name: Fqn,
    pub member: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub name: Fqn,
    pub title: Option<String>,
    pub description: Option<String>,
    pub definition: Option<String>,
    pub members: Vec<(String, Vec<Doc>)>,
}

impl Name {
    pub fn is_singleton(&self) -> bool {
        self.last_start == 0
    }

    pub fn first(&self) -> &str {
        &self.s[..self.first_end]
    }

    pub fn last(&self) -> &str {
        &self.s[self.last_start..]
    }

    pub fn full(&self) -> &str {
        &self.s
    }

    pub fn rest(&self) -> Option<&str> {
        if self.is_singleton() {
            None
        } else {
            Some(&self.s[self.first_end + 2..])
        }
    }

    pub fn rest_or_first(&self) -> &str {
        self.rest().unwrap_or_else(|| self.first())
    }

    pub fn parent(&self) -> Option<Self> {
        if self.is_singleton() {
            None
        } else {
            Some((&self.s[..self.last_start - 2]).to_owned().into())
        }
    }

    pub fn child(&self, s: &str) -> Self {
        let mut name = self.s.clone();
        name.push_str("::");
        name.push_str(s);
        name.into()
    }

    pub fn ends_with(&self, name: &Name) -> bool {
        self.s == name.s || self.s.ends_with(&format!("::{}", name.s))
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.s.as_ref()
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        let first_end = s.find("::").unwrap_or_else(|| s.len());
        let last_start = s.rfind("::").map(|i| i + 2).unwrap_or(0);
        Self {
            s,
            first_end,
            last_start,
        }
    }
}

impl From<Name> for String {
    fn from(n: Name) -> Self {
        n.s
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.s)
    }
}

impl str::FromStr for Name {
    type Err = convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_owned().into())
    }
}

impl Fqn {
    pub fn krate(&self) -> &str {
        self.first()
    }

    pub fn parent(&self) -> Option<Self> {
        self.0.parent().map(From::from)
    }

    pub fn child(&self, s: &str) -> Self {
        self.0.child(s).into()
    }
}

impl AsRef<str> for Fqn {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<Name> for Fqn {
    fn from(n: Name) -> Self {
        Self(n)
    }
}

impl From<String> for Fqn {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl ops::Deref for Fqn {
    type Target = Name;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Crate {
    pub fn new(name: String, path: path::PathBuf) -> Self {
        Crate { name, path }
    }

    pub fn find_item(&self, name: &Fqn) -> anyhow::Result<Option<Item>> {
        if self.name == name.krate() {
            if let Some(local_name) = name.rest() {
                if let Some(path) = parser::find_item(self.path.join("all.html"), local_name)? {
                    let path = path::PathBuf::from(path);
                    return Ok(Some(Item::new(name.clone(), self.path.join(path), None)));
                }
            }
        }
        Ok(None)
    }

    pub fn find_module(&self, name: &Fqn) -> Option<Item> {
        if self.name == name.krate() {
            let module_path = if let Some(rest) = name.rest() {
                rest.split("::").fold(path::PathBuf::new(), |mut p, s| {
                    p.push(s);
                    p
                })
            } else {
                path::PathBuf::new()
            };
            let path = self.path.join(module_path).join("index.html");
            if path.is_file() {
                Some(Item::new(name.clone(), path, None))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn find_member(&self, name: &Fqn) -> Option<Item> {
        if self.name == name.krate() {
            if let Some(parent) = name.parent() {
                // TODO: error
                self.find_item(&parent)
                    .unwrap()
                    .and_then(|i| i.find_member(name.last()))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Item {
    pub fn new(name: Fqn, path: path::PathBuf, member: Option<String>) -> Self {
        Item { path, member, name }
    }

    pub fn load_doc(&self) -> anyhow::Result<Doc> {
        if let Some(member) = &self.member {
            parser::parse_member_doc(&self.path, &self.name, member)
        } else {
            parser::parse_item_doc(&self.path, &self.name)
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
    pub fn new(name: Fqn) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

impl fmt::Display for Doc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(description) = &self.description {
            write!(f, "{}: {}", &self.name, description)
        } else {
            write!(f, "{}", &self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Name;

    fn assert_name(input: &str, first: &str, last: &str, rest: &str) {
        let name: Name = input.to_owned().into();
        assert_eq!(first, name.first(), "first for '{}'", input);
        assert_eq!(last, name.last(), "last for '{}'", input);
        assert_eq!(input, name.full(), "full for '{}'", input);
        if rest == input {
            assert_eq!(None, name.rest(), "rest for '{}'", input);
        } else {
            assert_eq!(Some(rest), name.rest(), "rest for '{}'", input);
        }
    }

    #[test]
    fn test_empty_name() {
        assert_name("", "", "", "");
    }

    #[test]
    fn test_crate_name() {
        assert_name("rand", "rand", "rand", "rand");
    }

    #[test]
    fn test_module_name() {
        assert_name("rand::error", "rand", "error", "error");
        assert_name("rand::error::nested", "rand", "nested", "error::nested");
    }

    #[test]
    fn test_item_name() {
        assert_name("rand::Error", "rand", "Error", "Error");
        assert_name("rand::error::Error", "rand", "Error", "error::Error");
    }

    #[test]
    fn test_member_name() {
        assert_name("rand::Error::source", "rand", "source", "Error::source");
        assert_name(
            "rand::error::Error::source",
            "rand",
            "source",
            "error::Error::source",
        );
    }

    #[test]
    fn test_colon() {
        assert_name("er:ror::Error", "er:ror", "Error", "Error");
    }
}
