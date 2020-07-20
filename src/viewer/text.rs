// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::fmt;

use html2text::render::text_renderer;

use crate::doc;
use crate::viewer;

#[derive(Clone, Debug)]
pub struct TextViewer {
    line_length: usize,
}

struct Decorator {
    links: Vec<String>,
    ignore_next_link: bool,
}

impl TextViewer {
    pub fn new() -> Self {
        Self {
            line_length: viewer::get_line_length(),
        }
    }

    fn print(&self, s: &str) {
        println!(
            "{}",
            html2text::from_read_with_decorator(s.as_bytes(), self.line_length, Decorator::new())
        );
    }

    fn print_opt(&self, s: Option<&str>) {
        if let Some(s) = s {
            self.print(s);
        }
    }

    fn print_heading(&self, s: &str, level: usize) {
        let prefix = "#".repeat(level);
        print!("{} ", prefix);
        self.print(s);
    }

    fn print_list(&self, items: &[impl fmt::Display]) {
        let html = items
            .iter()
            .map(|i| format!("<li>{}</li>", i))
            .collect::<Vec<_>>()
            .join("");
        self.print(&format!("<ul>{}</ul>", html));
    }
}

impl viewer::Viewer for TextViewer {
    fn open(&self, doc: &doc::Doc) -> anyhow::Result<()> {
        viewer::spawn_pager();

        self.print_heading(&doc.title, 1);
        self.print_opt(doc.definition.as_deref());
        self.print_opt(doc.description.as_deref());
        for (heading, items) in &doc.members {
            if !items.is_empty() {
                println!();
                self.print_heading(heading, 2);
                self.print_list(items);
            }
        }
        Ok(())
    }
}

impl Decorator {
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            ignore_next_link: false,
        }
    }

    fn show_link(&self, url: &str) -> bool {
        // only show absolute links -- local links are most likely not helpful
        (url.starts_with("http") || url.starts_with("https")) &&
            // ignore playground links -- typically, these links are too long to display in a
            // sensible fasshion
            !url.starts_with("http://play.rust-lang.org") &&
            !url.starts_with("https://play.rust-lang.org")
    }
}

impl text_renderer::TextDecorator for Decorator {
    type Annotation = ();

    fn decorate_link_start(&mut self, url: &str) -> (String, Self::Annotation) {
        if self.show_link(url) {
            self.ignore_next_link = false;
            self.links.push(url.to_string());
            ("[".to_owned(), ())
        } else {
            self.ignore_next_link = true;
            (String::new(), ())
        }
    }

    fn decorate_link_end(&mut self) -> String {
        if self.ignore_next_link {
            String::new()
        } else {
            format!("][{}]", self.links.len())
        }
    }

    fn decorate_em_start(&mut self) -> (String, Self::Annotation) {
        ("*".to_owned(), ())
    }

    fn decorate_em_end(&mut self) -> String {
        "*".to_owned()
    }

    fn decorate_strong_start(&mut self) -> (String, Self::Annotation) {
        ("**".to_owned(), ())
    }

    fn decorate_strong_end(&mut self) -> String {
        "**".to_owned()
    }

    fn decorate_code_start(&mut self) -> (String, Self::Annotation) {
        ("`".to_owned(), ())
    }

    fn decorate_code_end(&mut self) -> String {
        "`".to_owned()
    }

    fn decorate_preformat_first(&mut self) -> Self::Annotation {}
    fn decorate_preformat_cont(&mut self) -> Self::Annotation {}

    fn decorate_image(&mut self, title: &str) -> (String, Self::Annotation) {
        (format!("[{}]", title), ())
    }

    fn finalise(self) -> Vec<text_renderer::TaggedLine<()>> {
        self.links
            .into_iter()
            .enumerate()
            .map(|(idx, s)| {
                text_renderer::TaggedLine::from_string(format!("[{}] {}", idx + 1, s), &())
            })
            .collect()
    }

    fn make_subblock_decorator(&self) -> Self {
        Decorator::new()
    }
}
