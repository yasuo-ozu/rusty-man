// SPDX-FileCopyrightText: 2020-2021 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! Parses HTML files generated by rustdoc.
//!
//! For details on the format of the parsed HTML files, check the following items in the
//! `html::render` module of `librustdoc` (in the Rust source):
//! - The `krate` and `render_item` methods of the `Context` struct are the main entry points for
//!   the rendering.
//! - The `print_item` and `item_*`functions generate the HTML for an item (module, struct, …).
//! - The `AllTypes::print` function generates the HTML for the `all.html` page using the
//!   `print_entries` function.

mod util;

use std::path;

use anyhow::Context;
use markup5ever::local_name;

use crate::doc;

use util::NodeRefExt;

pub struct Parser {
    document: kuchiki::NodeRef,
    path: Option<path::PathBuf>,
}

impl Parser {
    pub fn from_file(path: impl AsRef<path::Path>) -> anyhow::Result<Parser> {
        use kuchiki::traits::TendrilSink;

        log::info!("Reading HTML from file '{}'", path.as_ref().display());
        let document = kuchiki::parse_html()
            .from_utf8()
            .from_file(path.as_ref())
            .context("Could not read HTML file")?;
        log::info!("HTML file parsed successfully");

        Ok(Parser {
            document,
            path: Some(path.as_ref().to_owned()),
        })
    }

    pub fn from_string(s: impl Into<String>) -> anyhow::Result<Parser> {
        use kuchiki::traits::TendrilSink;

        log::info!("Reading HTML from string");
        let document = kuchiki::parse_html()
            .from_utf8()
            .read_from(&mut s.into().as_bytes())
            .context("Could not read HTML string")?;
        log::info!("HTML string parsed successfully");

        Ok(Parser {
            document,
            path: None,
        })
    }

    pub fn find_item(&self, item: &str) -> anyhow::Result<Option<String>> {
        let item = select(&self.document, "ul.docblock li a")?
            .find(|e| e.text_contents() == item)
            .and_then(|e| e.get_attribute("href"));
        Ok(item)
    }

    pub fn find_member(&self, name: &doc::Fqn) -> anyhow::Result<Option<doc::ItemType>> {
        let member = get_member(&self.document, name.last())?;
        if let Some(member) = member {
            let id = member
                .get_attribute("id")
                .with_context(|| format!("The member {} does not have an ID", name))?;
            let ty = id.splitn(2, '.').next().unwrap().parse()?;
            Ok(Some(ty))
        } else {
            Ok(None)
        }
    }

    pub fn parse_item_doc(&self, name: &doc::Fqn, ty: doc::ItemType) -> anyhow::Result<doc::Doc> {
        log::info!("Parsing item documentation for '{}'", name);
        let definition_selector = match ty {
            doc::ItemType::Constant => "pre.const",
            doc::ItemType::Function => "pre.fn",
            doc::ItemType::Typedef => "pre.typedef",
            _ => ".docblock.type-decl",
        };
        let definition = select_first(&self.document, definition_selector)?;
        // Since Rust 1.54.0, the main description is wrapped in a details element
        let mut description = select_first(
            &self.document,
            "#main > details.top-doc > .docblock:not(.type-decl)",
        )?;
        if description.is_none() {
            description = select_first(&self.document, "#main > .docblock:not(.type-decl)")?;
        }

        let mut doc = doc::Doc::new(name.clone(), ty);
        doc.description = description.map(From::from);
        doc.definition = definition.map(From::from);
        if let Some(path) = self.path.as_ref() {
            doc.set_url(path, None);
        }

        let members = vec![
            get_variants(&self.document, name)?,
            get_fields(&self.document, name)?,
            get_assoc_types(&self.document, name)?,
            get_methods(&self.document, name)?,
            get_implementations(&self.document, name)?,
        ];
        for (ty, groups) in members.into_iter() {
            if !groups.is_empty() {
                doc.groups.insert(ty, groups);
            }
        }

        Ok(doc)
    }

    pub fn parse_member_doc(&self, name: &doc::Fqn, ty: doc::ItemType) -> anyhow::Result<doc::Doc> {
        log::info!("Parsing member documentation for '{}'", name);
        let member_selector = get_member_selector(ty, name.last());
        let heading = select_first(&self.document, &member_selector)?
            .with_context(|| format!("Could not find member {}", name))?;

        // Since Rust 1.54.0, the <code> element is replaced with a <h4 class="code-header">
        let code = if let Some(code) = select_first(heading.as_node(), "code")? {
            Ok(code)
        } else if let Some(code) = select_first(heading.as_node(), "h4.code-header")? {
            Ok(code)
        } else {
            Err(anyhow::anyhow!(
                "The member {} does not have a definition",
                name
            ))
        }?;

        // Since Rust 1.54.0, there is an additional summary element around the definition
        let docblock = heading.as_node().next_sibling().or_else(|| {
            heading
                .as_node()
                .parent()
                .and_then(|parent| parent.next_sibling())
        });

        let mut doc = doc::Doc::new(name.clone(), ty);
        doc.definition = Some(code.into());
        doc.description = docblock.map(From::from);
        if let Some(path) = self.path.as_ref() {
            doc.set_url(path, Some(member_selector));
        }
        Ok(doc)
    }

    pub fn parse_module_doc(&self, name: &doc::Fqn) -> anyhow::Result<doc::Doc> {
        log::info!("Parsing module documentation for '{}'", name);
        let description = select_first(&self.document, ".docblock")?;

        let mut doc = doc::Doc::new(name.clone(), doc::ItemType::Module);
        doc.description = description.map(From::from);
        if let Some(path) = self.path.as_ref() {
            doc.set_url(path, None);
        }
        for item_type in MODULE_MEMBER_TYPES {
            let mut group = doc::MemberGroup::new(None);
            group.members = get_members(&self.document, name, *item_type)?;
            if !group.members.is_empty() {
                doc.groups.insert(*item_type, vec![group]);
            }
        }
        Ok(doc)
    }

    pub fn find_examples(&self) -> anyhow::Result<Vec<doc::Example>> {
        let examples = select(&self.document, ".rust-example-rendered")?;
        Ok(examples.map(|n| get_example(n.as_node())).collect())
    }
}

impl From<kuchiki::NodeRef> for doc::Text {
    fn from(node: kuchiki::NodeRef) -> doc::Text {
        doc::Text::from(&node)
    }
}

impl From<&kuchiki::NodeRef> for doc::Text {
    fn from(node: &kuchiki::NodeRef) -> doc::Text {
        doc::Text {
            plain: node_to_text(node),
            html: node.to_string(),
        }
    }
}

impl<T> From<kuchiki::NodeDataRef<T>> for doc::Text {
    fn from(node: kuchiki::NodeDataRef<T>) -> doc::Text {
        node.as_node().into()
    }
}

impl From<kuchiki::NodeRef> for doc::Code {
    fn from(node: kuchiki::NodeRef) -> doc::Code {
        doc::Code::from(&node)
    }
}

impl From<&kuchiki::NodeRef> for doc::Code {
    fn from(node: &kuchiki::NodeRef) -> doc::Code {
        doc::Code::new(node_to_text(node))
    }
}

impl<T> From<kuchiki::NodeDataRef<T>> for doc::Code {
    fn from(node: kuchiki::NodeDataRef<T>) -> doc::Code {
        node.as_node().into()
    }
}

fn node_to_text(node: &kuchiki::NodeRef) -> String {
    let mut s = String::new();
    push_node_to_text(&mut s, node);
    s.trim().to_string()
}

fn push_node_to_text(s: &mut String, node: &kuchiki::NodeRef) {
    if node.has_class("notable-traits") {
        // The notable-traits element lists informations about types in a code block.  But we only
        // want to extract the code, so we skip this element.
        return;
    }

    let is_docblock = node.has_class("docblock");

    let add_newline = if node.is_element(&local_name!("br")) {
        true
    } else if node.has_class("fmt-newline") || is_docblock {
        !s.is_empty() && !s.ends_with('\n')
    } else {
        false
    };
    if add_newline {
        s.push('\n');
    }

    if let Some(text) = node.as_text() {
        s.push_str(&text.borrow())
    }

    for child in node.children() {
        push_node_to_text(s, &child);
    }

    if is_docblock && !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
}

fn select(
    element: &kuchiki::NodeRef,
    selector: &str,
) -> anyhow::Result<kuchiki::iter::Select<kuchiki::iter::Elements<kuchiki::iter::Descendants>>> {
    element
        .select(selector)
        .ok()
        .with_context(|| format!("Could not apply selector {}", selector))
}

fn it_select<I: kuchiki::iter::NodeIterator>(
    iter: I,
    selector: &str,
) -> anyhow::Result<kuchiki::iter::Select<kuchiki::iter::Elements<I>>> {
    iter.select(selector)
        .ok()
        .with_context(|| format!("Could not apply selector {}", selector))
}

fn select_first(
    element: &kuchiki::NodeRef,
    selector: &str,
) -> anyhow::Result<Option<kuchiki::NodeDataRef<kuchiki::ElementData>>> {
    select(element, selector).map(|mut i| i.next())
}

fn it_select_first<I: kuchiki::iter::NodeIterator>(
    iter: I,
    selector: &str,
) -> anyhow::Result<Option<kuchiki::NodeDataRef<kuchiki::ElementData>>> {
    it_select(iter, selector).map(|mut i| i.next())
}

fn get_example(node: &kuchiki::NodeRef) -> doc::Example {
    let description_element = node
        .parent()
        .as_ref()
        .and_then(NodeRefExt::previous_sibling_element);
    let description = description_element
        .and_then(|n| {
            if n.text_contents().ends_with(':') {
                Some(n)
            } else {
                None
            }
        })
        .map(From::from);
    doc::Example::new(description, node.into())
}

const MODULE_MEMBER_TYPES: &[doc::ItemType] = &[
    doc::ItemType::ExternCrate,
    doc::ItemType::Import,
    doc::ItemType::Primitive,
    doc::ItemType::Module,
    doc::ItemType::Macro,
    doc::ItemType::Struct,
    doc::ItemType::Enum,
    doc::ItemType::Constant,
    doc::ItemType::Static,
    doc::ItemType::Trait,
    doc::ItemType::Function,
    doc::ItemType::Typedef,
    doc::ItemType::Union,
];

fn get_id_part(node: &kuchiki::NodeRef, i: usize) -> Option<String> {
    // id of format <type>.<name> (or <type>.<name>-<idx> for name collisions)
    if let Some(id) = node.get_attribute("id") {
        id.splitn(2, '.')
            .nth(i)
            .and_then(|s| s.splitn(2, '-').next())
            .map(ToOwned::to_owned)
    } else {
        None
    }
}

fn get_fields(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
) -> anyhow::Result<(doc::ItemType, Vec<doc::MemberGroup>)> {
    let ty = doc::ItemType::StructField;
    let mut fields = MemberDocs::new(parent, ty);
    let heading = select_first(document, &format!("#{}", get_item_group_id(ty)))?;

    let mut next = heading.as_ref().and_then(NodeRefExt::next_sibling_element);
    let mut name: Option<String> = None;
    let mut definition: Option<doc::Code> = None;

    while let Some(element) = &next {
        if element.is_element(&local_name!("span")) && element.has_class("structfield") {
            fields.push(&mut name, &mut definition, None)?;
            name = get_id_part(element, 1);
            definition = Some(element.into());
        } else if element.is_element(&local_name!("div")) {
            if element.has_class("docblock") {
                fields.push(&mut name, &mut definition, Some(element.into()))?;
            }
        } else {
            fields.push(&mut name, &mut definition, None)?;
            break;
        }
        next = element.next_sibling();
    }

    Ok((ty, fields.into_member_groups(None)))
}

fn get_methods(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
) -> anyhow::Result<(doc::ItemType, Vec<doc::MemberGroup>)> {
    let ty = doc::ItemType::Method;
    let mut groups: Vec<doc::MemberGroup> = Vec::new();

    // Rust < 1.45
    groups.append(&mut get_method_groups(
        document,
        parent,
        "methods".to_owned(),
        ty,
        &local_name!("h4"),
    )?);
    // Rust >= 1.45, < 1.54.0
    groups.append(&mut get_method_groups(
        document,
        parent,
        "implementations".to_owned(),
        ty,
        &local_name!("h4"),
    )?);
    // Rust >= 1.54.0
    groups.append(&mut get_method_groups(
        document,
        parent,
        "implementations".to_owned(),
        ty,
        &local_name!("h2"),
    )?);

    let heading = select_first(document, "#deref-methods")?;
    if let Some(heading) = heading {
        let title = heading.as_node().text_contents();
        if let Some(impl_items) = heading.as_node().next_sibling() {
            let group = get_method_group(
                parent,
                Some(title),
                &impl_items,
                doc::ItemType::Method,
                &local_name!("h4"),
            )?;
            if let Some(group) = group {
                groups.push(group);
            }
        }
    }

    let heading = select_first(document, "#required-methods")?;
    if let Some(heading) = heading {
        if let Some(methods) = heading.as_node().next_sibling() {
            let title = "Required Methods";
            // Rust < 1.54.0
            let group = if let Some(group) = get_method_group(
                parent,
                Some(title.to_owned()),
                &methods,
                doc::ItemType::TyMethod,
                &local_name!("h3"),
            )? {
                Some(group)
            } else {
                // Rust >= 1.54.0
                get_method_group(
                    parent,
                    Some(title.to_owned()),
                    &methods,
                    doc::ItemType::TyMethod,
                    &local_name!("h4"),
                )?
            };
            if let Some(group) = group {
                groups.push(group);
            }
        }
    }

    let heading = select_first(document, "#provided-methods")?;
    if let Some(heading) = heading {
        if let Some(methods) = heading.as_node().next_sibling() {
            let title = "Provided Methods";
            // Rust < 1.54.0
            let group = if let Some(group) = get_method_group(
                parent,
                Some(title.to_owned()),
                &methods,
                doc::ItemType::TyMethod,
                &local_name!("h3"),
            )? {
                Some(group)
            } else {
                // Rust >= 1.54.0
                get_method_group(
                    parent,
                    Some(title.to_owned()),
                    &methods,
                    doc::ItemType::TyMethod,
                    &local_name!("h4"),
                )?
            };
            if let Some(group) = group {
                groups.push(group);
            }
        }
    }

    Ok((ty, groups))
}

fn get_assoc_types(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
) -> anyhow::Result<(doc::ItemType, Vec<doc::MemberGroup>)> {
    let ty = doc::ItemType::AssocType;
    let mut groups: Vec<doc::MemberGroup> = Vec::new();

    let heading = select_first(document, "#associated-types")?;
    if let Some(heading) = heading {
        if let Some(methods) = heading.as_node().next_sibling() {
            // Rust < 1.54.0
            let group = if let Some(group) = get_method_group(
                parent,
                None,
                &methods,
                doc::ItemType::AssocType,
                &local_name!("h3"),
            )? {
                Some(group)
            } else {
                // Rust >= 1.54.0
                get_method_group(
                    parent,
                    None,
                    &methods,
                    doc::ItemType::AssocType,
                    &local_name!("h4"),
                )?
            };
            if let Some(group) = group {
                groups.push(group);
            }
        }
    }

    Ok((ty, groups))
}

fn get_method_groups(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
    heading_id: String,
    ty: doc::ItemType,
    subheading_type: &markup5ever::LocalName,
) -> anyhow::Result<Vec<doc::MemberGroup>> {
    let mut groups: Vec<doc::MemberGroup> = Vec::new();
    let heading = select_first(document, &format!("#{}", heading_id))?;
    let mut next = heading.as_ref().and_then(NodeRefExt::next_sibling_element);

    while let Some(subheading) = next.take() {
        if subheading.is_element(&local_name!("h3")) && subheading.has_class("impl") {
            if let Some(title) = subheading.first_child() {
                if let Some(impl_items) = subheading.next_sibling() {
                    if let Some(group) =
                        get_impl_items(parent, &title, &impl_items, ty, subheading_type)?
                    {
                        groups.push(group);
                    }
                    next = impl_items.next_sibling();
                }
            }
        } else if subheading.is_element(&local_name!("details")) {
            if let Some(summary) = subheading.first_child() {
                let h3 = select_first(&summary, "h3")?;
                if let Some(title) = h3.map(|n| {
                    n.as_node()
                        .first_child()
                        .filter(|n| n.is_element(&local_name!("code")))
                        .unwrap_or_else(|| n.as_node().to_owned())
                }) {
                    if let Some(impl_items) = summary.next_sibling() {
                        if let Some(group) =
                            get_impl_items(parent, &title, &impl_items, ty, subheading_type)?
                        {
                            groups.push(group);
                        }
                        next = subheading.next_sibling();
                    }
                }
            }
        }
    }

    Ok(groups)
}

fn get_impl_items(
    parent: &doc::Fqn,
    title: &kuchiki::NodeRef,
    impl_items: &kuchiki::NodeRef,
    ty: doc::ItemType,
    subheading_type: &markup5ever::LocalName,
) -> anyhow::Result<Option<doc::MemberGroup>> {
    let title = title.text_contents();
    if impl_items.is_element(&local_name!("div")) && impl_items.has_class("impl-items") {
        get_method_group(parent, Some(title), impl_items, ty, subheading_type)
    } else {
        Ok(None)
    }
}

fn get_method_group(
    parent: &doc::Fqn,
    title: Option<String>,
    impl_items: &kuchiki::NodeRef,
    ty: doc::ItemType,
    heading_type: &markup5ever::LocalName,
) -> anyhow::Result<Option<doc::MemberGroup>> {
    let mut methods = MemberDocs::new(parent, ty);

    let mut name: Option<String> = None;
    let mut definition: Option<doc::Code> = None;
    for element in impl_items.children() {
        if element.is_element(heading_type) && element.has_class("method") {
            methods.push(&mut name, &mut definition, None)?;
            name = get_id_part(&element, 1);
            definition = it_select_first(element.children(), "code")?.map(From::from);
        } else if element.is_element(&local_name!("div")) && element.has_class("docblock") {
            methods.push(&mut name, &mut definition, Some(element.into()))?;
        } else if element.is_element(&local_name!("details")) {
            // Since Rust 1.54.0, the heading and the docblock are wrapped in details and summary
            // elements.
            if let Some(div) = select_first(&element, "summary div.method")? {
                if div.as_node().children().any(|n| n.is_element(heading_type)) {
                    methods.push(&mut name, &mut definition, None)?;
                    name = get_id_part(div.as_node(), 1);
                    definition =
                        it_select_first(div.as_node().children(), ".code-header")?.map(From::from);
                }
            }
            if let Some(docblock) = select_first(&element, "div.docblock")? {
                methods.push(&mut name, &mut definition, Some(docblock.into()))?;
            }
        }
    }

    Ok(methods.into_member_group(title))
}

fn get_variants(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
) -> anyhow::Result<(doc::ItemType, Vec<doc::MemberGroup>)> {
    let ty = doc::ItemType::Variant;
    let mut variants = MemberDocs::new(parent, ty);
    let heading = select_first(document, &format!("#{}", get_item_group_id(ty)))?;

    let mut next = heading.as_ref().and_then(NodeRefExt::next_sibling_element);
    let mut name: Option<String> = None;
    let mut definition: Option<doc::Code> = None;
    while let Some(element) = &next {
        if element.is_element(&local_name!("div")) {
            if element.has_class("variant") {
                variants.push(&mut name, &mut definition, None)?;
                name = get_id_part(element, 1);
                definition = Some(element.into());
            } else if element.has_class("docblock") {
                variants.push(&mut name, &mut definition, Some(element.into()))?;
            }

            next = element.next_sibling();
        } else {
            variants.push(&mut name, &mut definition, None)?;
            break;
        }
    }

    Ok((ty, variants.into_member_groups(None)))
}

fn get_implementations(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
) -> anyhow::Result<(doc::ItemType, Vec<doc::MemberGroup>)> {
    let mut groups: Vec<doc::MemberGroup> = Vec::new();

    let group_data = vec![
        // Rust < 1.45
        ("Trait Implementations", "implementations-list"),
        // Rust >= 1.45
        ("Trait Implementations", "trait-implementations-list"),
        (
            "Auto Trait Implementations",
            "synthetic-implementations-list",
        ),
        ("Blanket Implementations", "blanket-implementations-list"),
    ];

    for (title, id) in group_data {
        if let Some(group) = get_implementation_group(document, parent, title, id)? {
            groups.push(group);
        }
    }

    Ok((doc::ItemType::Impl, groups))
}

fn get_implementation_group(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
    title: &str,
    list_id: &str,
) -> anyhow::Result<Option<doc::MemberGroup>> {
    let ty = doc::ItemType::Impl;
    let mut impls = MemberDocs::new(parent, ty);
    let list_div = select_first(document, &format!("#{}", list_id))?;

    if let Some(list_div) = list_div {
        for item in list_div.as_node().children() {
            let h3 = if item.is_element(&local_name!("details")) {
                if let Some(summary) = item.first_child() {
                    select_first(&summary, "h3.impl, h3.code-header")?
                        .map(|n| n.as_node().to_owned())
                } else {
                    None
                }
            } else if item.is_element(&local_name!("h3")) && item.has_class("impl") {
                Some(item)
            } else if item.is_element(&local_name!("div")) && item.has_class("impl") {
                select_first(&item, "h3")?.map(|n| n.as_node().to_owned())
            } else {
                None
            };

            if let Some(h3) = h3 {
                let a = select_first(&h3, "a")?;
                let mut name = a.map(|n| n.as_node().text_contents());
                let mut definition = Some(
                    h3.first_child()
                        .filter(|n| n.is_element(&local_name!("code")))
                        .map(doc::Code::from)
                        .unwrap_or_else(|| h3.into()),
                );
                impls.push(&mut name, &mut definition, None)?;
            }
        }
    }

    impls.sort();

    Ok(impls.into_member_group(Some(title.to_owned())))
}

fn get_members(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
    ty: doc::ItemType,
) -> anyhow::Result<Vec<doc::Doc>> {
    let mut members: Vec<doc::Doc> = Vec::new();
    if let Some(table) = select_first(document, &format!("#{} + table", get_item_group_id(ty)))? {
        let items = select(table.as_node(), "td:first-child > :first-child")?;
        for item in items {
            let item_name = item.as_node().text_contents();
            let docblock = item.as_node().parent().and_then(|n| n.next_sibling());

            let mut doc = doc::Doc::new(parent.child(&item_name), ty);
            doc.description = docblock.map(From::from);
            members.push(doc);
        }
    }
    if let Some(div) = select_first(
        document,
        &format!("#{} + div.item-table", get_item_group_id(ty)),
    )? {
        let mut iter = div.as_node().children();
        while let (Some(item), Some(docblock)) = (iter.next(), iter.next()) {
            if !item.has_class("module-item") || !docblock.has_class("docblock-short") {
                continue;
            }
            let item_name = item.text_contents();
            let mut doc = doc::Doc::new(parent.child(&item_name), ty);
            doc.description = Some(docblock.into());
            members.push(doc);
        }
    }
    Ok(members)
}

const MEMBER_TYPES: &[doc::ItemType] = &[
    doc::ItemType::StructField,
    doc::ItemType::Variant,
    doc::ItemType::AssocType,
    doc::ItemType::AssocConst,
    doc::ItemType::Method,
];

fn get_member(
    document: &kuchiki::NodeRef,
    name: &str,
) -> anyhow::Result<Option<kuchiki::NodeDataRef<kuchiki::ElementData>>> {
    let selectors: Vec<_> = MEMBER_TYPES
        .iter()
        .map(|ty| get_member_selector(*ty, name))
        .collect();
    select_first(document, &selectors.join(", "))
}

fn get_member_selector(ty: doc::ItemType, name: &str) -> String {
    format!("#{}\\.{}", get_item_id(ty), name)
}

struct MemberDocs<'a> {
    docs: Vec<doc::Doc>,
    parent: &'a doc::Fqn,
    ty: doc::ItemType,
}

impl<'a> MemberDocs<'a> {
    pub fn new(parent: &'a doc::Fqn, ty: doc::ItemType) -> Self {
        Self {
            docs: Vec::new(),
            parent,
            ty,
        }
    }

    pub fn sort(&mut self) {
        self.docs.sort_by(|d1, d2| {
            d1.name
                .cmp(&d2.name)
                .then_with(|| d1.definition.cmp(&d2.definition))
        })
    }

    pub fn push(
        &mut self,
        name: &mut Option<String>,
        definition: &mut Option<doc::Code>,
        description: Option<doc::Text>,
    ) -> anyhow::Result<()> {
        let name = name.take();
        let definition = definition.take();

        if let Some(name) = name {
            let mut doc = doc::Doc::new(self.parent.child(&name), self.ty);
            doc.definition = definition;
            doc.description = description;
            self.docs.push(doc);
        }
        Ok(())
    }

    pub fn into_member_group(self, title: Option<String>) -> Option<doc::MemberGroup> {
        if self.docs.is_empty() {
            None
        } else {
            let mut group = doc::MemberGroup::new(title);
            group.members = self.docs;
            Some(group)
        }
    }

    pub fn into_member_groups(self, title: Option<String>) -> Vec<doc::MemberGroup> {
        self.into_member_group(title).into_iter().collect()
    }
}

impl<'a> From<MemberDocs<'a>> for Vec<doc::Doc> {
    fn from(md: MemberDocs<'a>) -> Self {
        md.docs
    }
}

fn get_item_id(ty: doc::ItemType) -> &'static str {
    use doc::ItemType;

    match ty {
        ItemType::Module => "mod",
        ItemType::ExternCrate => "externcrate",
        ItemType::Import => "import",
        ItemType::Struct => "struct",
        ItemType::Union => "union",
        ItemType::Enum => "enum",
        ItemType::Function => "fn",
        ItemType::Typedef => "type",
        ItemType::Static => "static",
        ItemType::Trait => "trait",
        ItemType::Impl => "impl",
        ItemType::TyMethod => "tymethod",
        ItemType::Method => "method",
        ItemType::StructField => "structfield",
        ItemType::Variant => "variant",
        ItemType::Macro => "macro",
        ItemType::Primitive => "primitive",
        ItemType::AssocType => "associatedtype",
        ItemType::Constant => "constant",
        ItemType::AssocConst => "associatedconstant",
        ItemType::ForeignType => "foreigntype",
        ItemType::Keyword => "keyword",
        ItemType::OpaqueTy => "opaque",
        ItemType::ProcAttribute => "attr",
        ItemType::ProcDerive => "derive",
        ItemType::TraitAlias => "traitalias",
    }
}

fn get_item_group_id(ty: doc::ItemType) -> &'static str {
    use doc::ItemType;

    match ty {
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

#[cfg(test)]
mod tests {
    use crate::doc;
    use crate::test_utils::{with_rustdoc, Format};

    #[test]
    fn test_find_item() {
        with_rustdoc("*", Format::all(), |_, _, path| {
            let path = path.join("kuchiki").join("all.html");
            let parser = super::Parser::from_file(path).unwrap();

            assert_eq!(None, parser.find_item("foobar").unwrap());
            assert_eq!(
                Some("struct.NodeRef.html".to_owned()),
                parser.find_item("NodeRef").unwrap()
            );
        });
    }

    #[test]
    fn test_parse_item_doc() {
        with_rustdoc("*", Format::all(), |_, _, path| {
            let path = path.join("kuchiki").join("struct.NodeRef.html");
            let name: doc::Fqn = "kuchiki::NodeRef".to_owned().into();
            let doc = super::Parser::from_file(path)
                .unwrap()
                .parse_item_doc(&name, doc::ItemType::Struct)
                .unwrap();

            assert_eq!(name, doc.name);
            assert_eq!(doc::ItemType::Struct, doc.ty);
            assert!(doc.definition.is_some());
            assert!(doc.description.is_some());
        });
    }

    #[test]
    fn test_find_member() {
        with_rustdoc("*", Format::all(), |_, _, path| {
            let path = path.join("kuchiki").join("struct.NodeDataRef.html");
            let name: doc::Fqn = "kuchiki::NodeDataRef::as_node".to_owned().into();
            let ty = super::Parser::from_file(path)
                .unwrap()
                .find_member(&name)
                .unwrap();
            assert_eq!(Some(doc::ItemType::Method), ty);
        });
    }

    #[test]
    fn test_parse_member_doc() {
        with_rustdoc("*", Format::all(), |_, _, path| {
            let path = path.join("kuchiki").join("struct.NodeDataRef.html");
            let name: doc::Fqn = "kuchiki::NodeDataRef::as_node".to_owned().into();
            let doc = super::Parser::from_file(path)
                .unwrap()
                .parse_member_doc(&name, doc::ItemType::Method)
                .unwrap();

            assert_eq!(name, doc.name);
            assert_eq!(doc::ItemType::Method, doc.ty);
            let definition = doc.definition.unwrap();
            assert_eq!(
                doc::Code::new("pub fn as_node(&self) -> &NodeRef".to_owned()),
                definition
            );
            assert!(doc.description.is_some());
        });
    }
}
