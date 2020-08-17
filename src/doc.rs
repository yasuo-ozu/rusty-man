// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::collections;
use std::convert;
use std::fmt;
use std::ops;
use std::str;

use crate::parser::html;

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

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Text {
    pub plain: String,
    pub html: String,
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Code(String);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ItemType {
    // module members
    ExternCrate,
    Import,
    Primitive,
    Module,
    Macro,
    Struct,
    Enum,
    Constant,
    Static,
    Trait,
    Function,
    Typedef,
    Union,

    // struct and union members
    StructField,

    // enum members
    Variant,

    // associated items
    AssocType,
    AssocConst,
    Method,
    Impl,

    // other items
    TyMethod,
    ForeignType,
    Keyword,
    OpaqueTy,
    ProcAttribute,
    ProcDerive,
    TraitAlias,
}

#[derive(Clone, Debug)]
pub struct Doc {
    pub name: Fqn,
    pub ty: ItemType,
    pub description: Option<Text>,
    pub definition: Option<Code>,
    pub groups: collections::BTreeMap<ItemType, Vec<MemberGroup>>,
}

#[derive(Clone, Debug)]
pub struct MemberGroup {
    pub title: Option<String>,
    pub members: Vec<Doc>,
}

#[derive(Clone, Debug)]
pub struct Example {
    pub description: Option<Text>,
    pub code: Code,
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

impl Code {
    pub fn new(s: String) -> Code {
        Code(s)
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl ops::Deref for Code {
    type Target = String;

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
            html::Parser::from_string(&description.html)?.find_examples()
        } else {
            Ok(Vec::new())
        }
    }
}

impl fmt::Display for Doc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(description) = &self.description {
            write!(f, "{}: {}", &self.name, description.plain)
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
    pub fn new(description: Option<Text>, code: Code) -> Self {
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
