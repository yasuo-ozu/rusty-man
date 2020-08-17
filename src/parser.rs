// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
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

use std::path;

use anyhow::Context;
use markup5ever::local_name;

use crate::doc;

pub struct Parser {
    document: kuchiki::NodeRef,
}

impl Parser {
    pub fn from_file(path: impl AsRef<path::Path>) -> anyhow::Result<Parser> {
        use kuchiki::traits::TendrilSink;

        log::info!("Reading HTML from file '{}'", path.as_ref().display());
        let document = kuchiki::parse_html()
            .from_utf8()
            .from_file(path)
            .context("Could not read HTML file")?;
        log::info!("HTML file parsed successfully");

        Ok(Parser { document })
    }

    pub fn from_string(s: impl Into<String>) -> anyhow::Result<Parser> {
        use kuchiki::traits::TendrilSink;

        log::info!("Reading HTML from string");
        let document = kuchiki::parse_html()
            .from_utf8()
            .read_from(&mut s.into().as_bytes())
            .context("Could not read HTML string")?;
        log::info!("HTML string parsed successfully");

        Ok(Parser { document })
    }

    pub fn find_item(&self, item: &str) -> anyhow::Result<Option<String>> {
        use std::ops::Deref;

        let item = select(&self.document, "ul.docblock li a")?
            .find(|e| e.text_contents() == item)
            .and_then(|e| get_attribute(e.deref(), "href"));
        Ok(item)
    }

    pub fn find_member(&self, name: &doc::Fqn) -> anyhow::Result<bool> {
        get_member(&self.document, name.last()).map(|member| member.is_some())
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
        let description = select_first(&self.document, "#main > .docblock:not(.type-decl)")?;

        let mut doc = doc::Doc::new(name.clone(), ty);
        doc.description = description.map(From::from);
        doc.definition = definition.map(From::from);

        let members = vec![
            get_variants(&self.document, name)?,
            get_fields(&self.document, name)?,
            get_assoc_types(&self.document, name)?,
            get_methods(&self.document, name)?,
            get_implementations(&self.document, name)?,
        ];
        for (ty, groups) in members.into_iter() {
            if !groups.is_empty() {
                doc.groups.push((ty, groups));
            }
        }

        Ok(doc)
    }

    pub fn parse_member_doc(&self, name: &doc::Fqn) -> anyhow::Result<doc::Doc> {
        log::info!("Parsing member documentation for '{}'", name);
        let member = get_member(&self.document, name.last())?
            .with_context(|| format!("Could not find member {}", name))?;
        let heading = member
            .as_node()
            .parent()
            .with_context(|| format!("The member {} does not have a parent", name))?;
        let parent_id = get_node_attribute(&heading, "id")
            .with_context(|| format!("The heading for member {} does not have an ID", name))?;
        let ty: doc::ItemType = parent_id.splitn(2, '.').next().unwrap().parse()?;
        let docblock = heading.next_sibling();

        let mut doc = doc::Doc::new(name.clone(), ty);
        doc.definition = Some(member.into());
        doc.description = docblock.map(From::from);
        Ok(doc)
    }

    pub fn parse_module_doc(&self, name: &doc::Fqn) -> anyhow::Result<doc::Doc> {
        log::info!("Parsing module documentation for '{}'", name);
        let description = select_first(&self.document, ".docblock")?;

        let mut doc = doc::Doc::new(name.clone(), doc::ItemType::Module);
        doc.description = description.map(From::from);
        for item_type in MODULE_MEMBER_TYPES {
            let mut group = doc::MemberGroup::new(None);
            group.members = get_members(&self.document, name, *item_type)?;
            if !group.members.is_empty() {
                doc.groups.push((*item_type, vec![group]));
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

fn node_to_text(node: &kuchiki::NodeRef) -> String {
    let mut s = String::new();
    push_node_to_text(&mut s, node);
    s.trim().to_string()
}

fn push_node_to_text(s: &mut String, node: &kuchiki::NodeRef) {
    let is_docblock = has_class(node, "docblock");

    let add_newline = if is_element(node, &local_name!("br")) {
        true
    } else if has_class(node, "fmt-newline") || is_docblock {
        !s.is_empty() && !s.ends_with('\n')
    } else {
        false
    };
    if add_newline {
        s.push_str("\n");
    }

    if let Some(text) = node.as_text() {
        s.push_str(&text.borrow())
    }

    for child in node.children() {
        push_node_to_text(s, &child);
    }

    if is_docblock && !s.is_empty() && !s.ends_with('\n') {
        s.push_str("\n");
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
    let description_element = node.parent().as_ref().and_then(previous_sibling_element);
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
    get_node_attribute(node, "id").and_then(|s| s.splitn(2, '.').nth(i).map(ToOwned::to_owned))
}

fn get_fields(
    document: &kuchiki::NodeRef,
    parent: &doc::Fqn,
) -> anyhow::Result<(doc::ItemType, Vec<doc::MemberGroup>)> {
    let ty = doc::ItemType::StructField;
    let mut fields = MemberDocs::new(parent, ty);
    let heading = select_first(document, &format!("#{}", ty.group_id()))?;

    let mut next = heading.and_then(|n| next_sibling_element(n.as_node()));
    let mut name: Option<String> = None;
    let mut definition: Option<doc::Text> = None;

    while let Some(element) = &next {
        if is_element(element, &local_name!("span")) && has_class(element, ty.class()) {
            fields.push(&mut name, &mut definition, None)?;
            name = get_id_part(element, 1);
            definition = Some(element.into());
        } else if is_element(element, &local_name!("div")) {
            if has_class(element, "docblock") {
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
    // Rust >= 1.45
    groups.append(&mut get_method_groups(
        document,
        parent,
        "implementations".to_owned(),
        ty,
        &local_name!("h4"),
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
            let title = "Required Methods".to_owned();
            let group = get_method_group(
                parent,
                Some(title),
                &methods,
                doc::ItemType::TyMethod,
                &local_name!("h3"),
            )?;
            if let Some(group) = group {
                groups.push(group);
            }
        }
    }

    let heading = select_first(document, "#provided-methods")?;
    if let Some(heading) = heading {
        if let Some(methods) = heading.as_node().next_sibling() {
            let title = "Provided Methods".to_owned();
            let group = get_method_group(
                parent,
                Some(title),
                &methods,
                doc::ItemType::TyMethod,
                &local_name!("h3"),
            )?;
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
            let group = get_method_group(
                parent,
                None,
                &methods,
                doc::ItemType::AssocType,
                &local_name!("h3"),
            )?;
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
    let mut next = heading.and_then(|n| next_sibling_element(n.as_node()));
    while let Some(subheading) = &next {
        if is_element(subheading, &local_name!("h3")) && has_class(subheading, "impl") {
            if let Some(title_element) = subheading.first_child() {
                let title = title_element.text_contents();
                next = subheading.next_sibling();
                if let Some(impl_items) = &next {
                    if is_element(impl_items, &local_name!("div"))
                        && has_class(impl_items, "impl-items")
                    {
                        let group = get_method_group(
                            parent,
                            Some(title),
                            &impl_items,
                            ty,
                            &subheading_type,
                        )?;
                        if let Some(group) = group {
                            groups.push(group);
                        }
                        next = impl_items.next_sibling();
                    }
                }
            } else {
                next = None;
            }
        } else {
            next = None;
        }
    }
    Ok(groups)
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
    let mut definition: Option<doc::Text> = None;
    for element in impl_items.children() {
        if is_element(&element, heading_type) && has_class(&element, "method") {
            methods.push(&mut name, &mut definition, None)?;
            name = get_id_part(&element, 1);
            definition = it_select_first(element.children(), "code")?.map(From::from);
        } else if is_element(&element, &local_name!("div")) && has_class(&element, "docblock") {
            methods.push(&mut name, &mut definition, Some(element.into()))?;
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
    let heading = select_first(document, &format!("#{}", ty.group_id()))?;

    let mut next = heading.and_then(|n| next_sibling_element(n.as_node()));
    let mut name: Option<String> = None;
    let mut definition: Option<doc::Text> = None;
    while let Some(element) = &next {
        if is_element(element, &local_name!("div")) {
            if has_class(element, ty.class()) {
                variants.push(&mut name, &mut definition, None)?;
                name = get_id_part(element, 1);
                definition = Some(element.into());
            } else if has_class(element, "docblock") {
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
            if is_element(&item, &local_name!("h3")) && has_class(&item, "impl") {
                let code = item.first_child();
                let a = code
                    .as_ref()
                    .and_then(|n| select_first(n, "a").transpose())
                    .transpose()?;
                let mut name = a.map(|n| n.as_node().text_contents());
                let mut definition = code.map(From::from);
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
    if let Some(table) = select_first(document, &format!("#{} + table", ty.group_id()))? {
        let items = select(table.as_node(), "td:first-child > :first-child")?;
        for item in items {
            let item_name = item.as_node().text_contents();
            let docblock = item.as_node().parent().and_then(|n| n.next_sibling());

            let mut doc = doc::Doc::new(parent.child(&item_name), ty);
            doc.description = docblock.map(From::from);
            members.push(doc);
        }
    }
    Ok(members)
}

fn get_member(
    document: &kuchiki::NodeRef,
    name: &str,
) -> anyhow::Result<Option<kuchiki::NodeDataRef<kuchiki::ElementData>>> {
    select_first(document, &format!("#{}\\.v", name))
}

fn get_attribute(element: &kuchiki::ElementData, name: &str) -> Option<String> {
    element.attributes.borrow().get(name).map(ToOwned::to_owned)
}

fn get_node_attribute(node: &kuchiki::NodeRef, name: &str) -> Option<String> {
    node.as_element().and_then(|e| get_attribute(e, name))
}

fn next_sibling_element(node: &kuchiki::NodeRef) -> Option<kuchiki::NodeRef> {
    let mut next = node.next_sibling();
    while let Some(node) = &next {
        if node.as_element().is_some() {
            break;
        }
        next = node.next_sibling();
    }
    next
}

fn previous_sibling_element(node: &kuchiki::NodeRef) -> Option<kuchiki::NodeRef> {
    let mut previous = node.previous_sibling();
    while let Some(node) = &previous {
        if node.as_element().is_some() {
            break;
        }
        previous = node.previous_sibling();
    }
    previous
}

fn is_element(node: &kuchiki::NodeRef, name: &markup5ever::LocalName) -> bool {
    node.as_element()
        .map(|e| &e.name.local == name)
        .unwrap_or(false)
}

fn has_class(node: &kuchiki::NodeRef, class: &str) -> bool {
    node.as_element()
        .and_then(|e| get_attribute(&e, "class"))
        .map(|a| a.split(' ').any(|s| s == class))
        .unwrap_or(false)
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
        definition: &mut Option<doc::Text>,
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

#[cfg(test)]
mod tests {
    use crate::doc;

    #[test]
    fn test_find_item() {
        let path = crate::tests::ensure_docs();
        let path = path.join("kuchiki").join("all.html");
        let parser = super::Parser::from_file(path).unwrap();

        assert_eq!(None, parser.find_item("foobar").unwrap());
        assert_eq!(
            Some("struct.NodeRef.html".to_owned()),
            parser.find_item("NodeRef").unwrap()
        );
    }

    #[test]
    fn test_parse_item_doc() {
        let path = crate::tests::ensure_docs();
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
    }

    #[test]
    fn test_parse_member_doc() {
        let path = crate::tests::ensure_docs();
        let path = path.join("kuchiki").join("struct.NodeDataRef.html");
        let name: doc::Fqn = "kuchiki::NodeDataRef::as_node".to_owned().into();
        let doc = super::Parser::from_file(path)
            .unwrap()
            .parse_member_doc(&name)
            .unwrap();

        assert_eq!(name, doc.name);
        assert_eq!(doc::ItemType::Method, doc.ty);
        let definition = doc.definition.unwrap();
        assert_eq!("pub fn as_node(&self) -> &NodeRef", &definition.plain);
        assert_eq!(
            "<code id=\"as_node.v\">\
             pub fn <a class=\"fnname\" href=\"#method.as_node\">as_node</a>(&amp;self) \
             -&gt; &amp;<a class=\"struct\" href=\"../kuchiki/struct.NodeRef.html\" \
             title=\"struct kuchiki::NodeRef\">NodeRef</a></code>",
            &definition.html
        );
        assert!(doc.description.is_some());
    }
}
