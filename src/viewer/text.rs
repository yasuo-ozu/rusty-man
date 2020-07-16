// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use crate::doc;
use crate::viewer;

#[derive(Clone, Debug)]
pub struct TextViewer {}

impl TextViewer {
    pub fn new() -> Self {
        Self {}
    }

    fn print(&self, s: &str) {
        println!("{}", html2text::from_read(s.as_bytes(), 100));
    }

    fn print_opt(&self, s: Option<&str>) {
        if let Some(s) = s {
            self.print(s);
        }
    }
}

impl viewer::Viewer for TextViewer {
    fn open(&self, doc: &doc::Doc) -> anyhow::Result<()> {
        self.print(&doc.title);
        self.print_opt(doc.definition.as_deref());
        self.print_opt(doc.description.as_deref());
        Ok(())
    }
}
