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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde_repr::Deserialize_repr)]
#[repr(u8)]
pub enum ItemType {
    Module = 0,
    ExternCrate = 1,
    Import = 2,
    Struct = 3,
    Enum = 4,
    Function = 5,
    Typedef = 6,
    Static = 7,
    Trait = 8,
    Impl = 9,
    TyMethod = 10,
    Method = 11,
    StructField = 12,
    Variant = 13,
    Macro = 14,
    Primitive = 15,
    AssocType = 16,
    Constant = 17,
    AssocConst = 18,
    Union = 19,
    ForeignType = 20,
    Keyword = 21,
    OpaqueTy = 22,
    ProcAttribute = 23,
    ProcDerive = 24,
    TraitAlias = 25,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Crate {
    pub name: String,
    pub path: path::PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub name: Fqn,
    pub ty: ItemType,
    pub path: path::PathBuf,
}

#[derive(Clone, Debug)]
pub struct Doc {
    pub name: Fqn,
    pub ty: ItemType,
    pub description: Option<String>,
    pub definition: Option<String>,
    pub groups: Vec<(ItemType, Vec<MemberGroup>)>,
}

#[derive(Clone, Debug)]
pub struct MemberGroup {
    pub title: Option<String>,
    pub members: Vec<Doc>,
}

#[derive(Clone, Debug)]
pub struct Example {
    pub description: Option<String>,
    pub code: String,
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

impl ItemType {
    pub fn name(&self) -> &str {
        match self {
            ItemType::Module => "Module",
            ItemType::ExternCrate => "Extern Crate",
            ItemType::Import => "Import",
            ItemType::Struct => "Struct",
            ItemType::Enum => "Enum",
            ItemType::Function => "Function",
            ItemType::Typedef => "Typedef",
            ItemType::Static => "Static",
            ItemType::Trait => "Trait",
            ItemType::Impl => "Implementation",
            ItemType::TyMethod => "Required Method",
            ItemType::Method => "Method",
            ItemType::StructField => "Field",
            ItemType::Variant => "Variant",
            ItemType::Macro => "Macro",
            ItemType::Primitive => "Primitive",
            ItemType::AssocType => "Associated Type",
            ItemType::Constant => "Constant",
            ItemType::AssocConst => "Associated Const",
            ItemType::Union => "Union",
            ItemType::ForeignType => "Foreign Type",
            ItemType::Keyword => "Keyword",
            ItemType::OpaqueTy => "Opaque Type",
            ItemType::ProcAttribute => "Proc Attribute",
            ItemType::ProcDerive => "Proc Derive",
            ItemType::TraitAlias => "Trait Alias",
        }
    }

    pub fn class(&self) -> &str {
        match self {
            ItemType::Module => "module",
            ItemType::ExternCrate => "extern-crate",
            ItemType::Import => "import",
            ItemType::Struct => "struct",
            ItemType::Enum => "enum",
            ItemType::Function => "function",
            ItemType::Typedef => "type",
            ItemType::Static => "static",
            ItemType::Trait => "trait",
            ItemType::Impl => "impl",
            ItemType::TyMethod => "required-method",
            ItemType::Method => "method",
            ItemType::StructField => "structfield",
            ItemType::Variant => "variant",
            ItemType::Macro => "macro",
            ItemType::Primitive => "primitive",
            ItemType::AssocType => "associated-type",
            ItemType::Constant => "constant",
            ItemType::AssocConst => "associated-const",
            ItemType::Union => "union",
            ItemType::ForeignType => "foreign-type",
            ItemType::Keyword => "keyword",
            ItemType::OpaqueTy => "opaque-type",
            ItemType::ProcAttribute => "proc-attribute",
            ItemType::ProcDerive => "proc-derive",
            ItemType::TraitAlias => "trait-alias",
        }
    }

    pub fn group_name(&self) -> &str {
        match self {
            ItemType::Module => "Modules",
            ItemType::ExternCrate => "Extern Crates",
            ItemType::Import => "Imports",
            ItemType::Struct => "Structs",
            ItemType::Enum => "Enums",
            ItemType::Function => "Functions",
            ItemType::Typedef => "Typedefs",
            ItemType::Static => "Statics",
            ItemType::Trait => "Traits",
            ItemType::Impl => "Implementations",
            ItemType::TyMethod => "Required Methods",
            ItemType::Method => "Methods",
            ItemType::StructField => "Fields",
            ItemType::Variant => "Variants",
            ItemType::Macro => "Macros",
            ItemType::Primitive => "Primitives",
            ItemType::AssocType => "Associated Types",
            ItemType::Constant => "Constants",
            ItemType::AssocConst => "Associated Consts",
            ItemType::Union => "Unions",
            ItemType::ForeignType => "Foreign Types",
            ItemType::Keyword => "Keywords",
            ItemType::OpaqueTy => "Opaque Types",
            ItemType::ProcAttribute => "Proc Attributes",
            ItemType::ProcDerive => "Proc Derives",
            ItemType::TraitAlias => "Trait Aliases",
        }
    }

    pub fn group_id(&self) -> &str {
        match self {
            ItemType::Module => "modules",
            ItemType::ExternCrate => "extern-crates",
            ItemType::Import => "imports",
            ItemType::Struct => "structs",
            ItemType::Enum => "enums",
            ItemType::Function => "functions",
            ItemType::Typedef => "types",
            ItemType::Static => "statics",
            ItemType::Trait => "traits",
            ItemType::Impl => "impls",
            ItemType::TyMethod => "required-methods",
            ItemType::Method => "methods",
            ItemType::StructField => "fields",
            ItemType::Variant => "variants",
            ItemType::Macro => "macros",
            ItemType::Primitive => "primitives",
            ItemType::AssocType => "associated-types",
            ItemType::Constant => "constants",
            ItemType::AssocConst => "associated-consts",
            ItemType::Union => "unions",
            ItemType::ForeignType => "foreign-types",
            ItemType::Keyword => "keywords",
            ItemType::OpaqueTy => "opaque-types",
            ItemType::ProcAttribute => "proc-attributes",
            ItemType::ProcDerive => "proc-derives",
            ItemType::TraitAlias => "trait-aliases",
        }
    }
}

impl str::FromStr for ItemType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mod" => Ok(ItemType::Module),
            "externcrate" => Ok(ItemType::ExternCrate),
            "import" => Ok(ItemType::Import),
            "struct" => Ok(ItemType::Struct),
            "union" => Ok(ItemType::Union),
            "enum" => Ok(ItemType::Enum),
            "fn" => Ok(ItemType::Function),
            "type" => Ok(ItemType::Typedef),
            "static" => Ok(ItemType::Static),
            "trait" => Ok(ItemType::Trait),
            "impl" => Ok(ItemType::Impl),
            "tymethod" => Ok(ItemType::TyMethod),
            "method" => Ok(ItemType::Method),
            "structfield" => Ok(ItemType::StructField),
            "variant" => Ok(ItemType::Variant),
            "macro" => Ok(ItemType::Macro),
            "primitive" => Ok(ItemType::Primitive),
            "associatedtype" => Ok(ItemType::AssocType),
            "constant" => Ok(ItemType::Constant),
            "associatedconstant" => Ok(ItemType::AssocConst),
            "foreigntype" => Ok(ItemType::ForeignType),
            "keyword" => Ok(ItemType::Keyword),
            "opaque" => Ok(ItemType::OpaqueTy),
            "attr" => Ok(ItemType::ProcAttribute),
            "derive" => Ok(ItemType::ProcDerive),
            "traitalias" => Ok(ItemType::TraitAlias),
            _ => Err(anyhow::anyhow!("Unsupported item type: {}", s)),
        }
    }
}

impl Crate {
    pub fn new(name: String, path: path::PathBuf) -> Self {
        Crate { name, path }
    }

    pub fn find_item(&self, name: &Fqn) -> anyhow::Result<Option<Item>> {
        log::info!("Searching item '{}' in crate '{}'", name, self.name);
        if self.name == name.krate() {
            if let Some(local_name) = name.rest() {
                if let Some(path) = parser::find_item(self.path.join("all.html"), local_name)? {
                    let path = path::PathBuf::from(path);
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    let item_type: ItemType = file_name.splitn(2, '.').next().unwrap().parse()?;
                    return Ok(Some(Item::new(
                        name.clone(),
                        self.path.join(path),
                        item_type,
                    )));
                }
            }
        }
        Ok(None)
    }

    pub fn find_module(&self, name: &Fqn) -> Option<Item> {
        log::info!("Searching module '{}' in crate '{}'", name, self.name);
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
                Some(Item::new(name.clone(), path, ItemType::Module))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn find_member(&self, name: &Fqn) -> Option<Item> {
        log::info!("Searching member '{}' in crate '{}'", name, self.name);
        if let Some(parent) = name.parent() {
            // TODO: error
            self.find_item(&parent)
                .unwrap()
                .and_then(|i| i.find_member(name))
        } else {
            None
        }
    }
}

impl Item {
    pub fn new(name: Fqn, path: path::PathBuf, ty: ItemType) -> Self {
        Item { name, ty, path }
    }

    pub fn load_doc(&self) -> anyhow::Result<Doc> {
        log::info!("Loading documentation for '{}'", self.name);
        match self.ty {
            ItemType::TyMethod
            | ItemType::Method
            | ItemType::StructField
            | ItemType::Variant
            | ItemType::AssocType
            | ItemType::AssocConst => parser::parse_member_doc(&self),
            ItemType::Module => parser::parse_module_doc(&self),
            _ => parser::parse_item_doc(&self),
        }
    }

    pub fn find_member(&self, name: &Fqn) -> Option<Item> {
        log::info!("Searching member '{}' in item '{}'", name, self.name);
        // TODO: error handling
        parser::find_member(&self.path, name).unwrap()
    }
}

impl Doc {
    pub fn new(name: Fqn, ty: ItemType) -> Self {
        Self {
            name,
            ty,
            description: Default::default(),
            definition: Default::default(),
            groups: Default::default(),
        }
    }

    pub fn find_examples(&self) -> anyhow::Result<Vec<Example>> {
        if let Some(description) = &self.description {
            parser::find_examples(&description)
        } else {
            Ok(Vec::new())
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

impl MemberGroup {
    pub fn new(title: Option<String>) -> Self {
        MemberGroup {
            title,
            members: Vec::new(),
        }
    }
}

impl Example {
    pub fn new(description: Option<String>, code: String) -> Self {
        Example { description, code }
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
