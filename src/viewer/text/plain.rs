// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use html2text::render::text_renderer;

use crate::viewer;

#[derive(Clone, Debug)]
pub struct PlainTextRenderer {
    line_length: usize,
}

struct Decorator {
    links: Vec<String>,
    ignore_next_link: bool,
    show_links: bool,
}

impl PlainTextRenderer {
    pub fn new() -> Self {
        Self {
            line_length: viewer::get_line_length(),
        }
    }
}

impl super::Printer for PlainTextRenderer {
    fn print_title(&self, left: &str, middle: &str, right: &str) -> io::Result<()> {
        super::print_title(self.line_length, left, middle, right)?;
        writeln!(io::stdout())
    }

    fn print_html(&self, indent: usize, s: &str, show_links: bool) -> io::Result<()> {
        let lines = html2text::from_read_with_decorator(
            s.as_bytes(),
            self.line_length - indent,
            Decorator::new(show_links),
        );
        for line in lines.trim().split('\n') {
            writeln!(io::stdout(), "{}{}", " ".repeat(indent), line)?;
        }
        Ok(())
    }

    fn print_code(&self, indent: usize, code: &str) -> io::Result<()> {
        for line in code.split('\n') {
            writeln!(io::stdout(), "{}{}", " ".repeat(indent), line)?;
        }
        Ok(())
    }

    fn print_heading(&self, indent: usize, _level: usize, s: &str) -> io::Result<()> {
        writeln!(io::stdout(), "{}{}", " ".repeat(indent), s)
    }

    fn println(&self) -> io::Result<()> {
        writeln!(io::stdout())
    }
}

impl Decorator {
    pub fn new(show_links: bool) -> Self {
        Self {
            links: Vec::new(),
            ignore_next_link: false,
            show_links,
        }
    }

    fn show_link(&self, url: &str) -> bool {
        self.show_links &&
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
        Decorator::new(self.show_links)
    }
}
