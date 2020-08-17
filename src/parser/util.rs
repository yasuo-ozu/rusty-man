// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! Helper methods for HTML parsing.

pub trait NodeRefExt {
    fn get_attribute(&self, name: &str) -> Option<String>;
    fn is_element(&self, name: &markup5ever::LocalName) -> bool;
    fn has_class(&self, class: &str) -> bool;
    fn next_sibling_element(&self) -> Option<kuchiki::NodeRef>;
    fn previous_sibling_element(&self) -> Option<kuchiki::NodeRef>;
}

impl NodeRefExt for kuchiki::NodeRef {
    fn get_attribute(&self, name: &str) -> Option<String> {
        self.as_element().and_then(|e| get_attribute(e, name))
    }

    fn is_element(&self, name: &markup5ever::LocalName) -> bool {
        self.as_element()
            .map(|e| &e.name.local == name)
            .unwrap_or(false)
    }

    fn has_class(&self, class: &str) -> bool {
        self.get_attribute("class")
            .map(|a| a.split(' ').any(|s| s == class))
            .unwrap_or(false)
    }

    fn next_sibling_element(&self) -> Option<kuchiki::NodeRef> {
        let mut next = self.next_sibling();
        while let Some(node) = &next {
            if node.as_element().is_some() {
                break;
            }
            next = node.next_sibling();
        }
        next
    }

    fn previous_sibling_element(&self) -> Option<kuchiki::NodeRef> {
        let mut previous = self.previous_sibling();
        while let Some(node) = &previous {
            if node.as_element().is_some() {
                break;
            }
            previous = node.previous_sibling();
        }
        previous
    }
}

impl<T> NodeRefExt for kuchiki::NodeDataRef<T> {
    fn get_attribute(&self, name: &str) -> Option<String> {
        self.as_node().get_attribute(name)
    }

    fn is_element(&self, name: &markup5ever::LocalName) -> bool {
        self.as_node().is_element(name)
    }

    fn has_class(&self, class: &str) -> bool {
        self.as_node().has_class(class)
    }

    fn next_sibling_element(&self) -> Option<kuchiki::NodeRef> {
        self.as_node().next_sibling_element()
    }

    fn previous_sibling_element(&self) -> Option<kuchiki::NodeRef> {
        self.as_node().previous_sibling_element()
    }
}

fn get_attribute(element: &kuchiki::ElementData, name: &str) -> Option<String> {
    element.attributes.borrow().get(name).map(ToOwned::to_owned)
}
