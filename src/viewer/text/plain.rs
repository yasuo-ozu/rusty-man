// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use html2text::render::text_renderer;

use crate::args;
use crate::doc;
use crate::viewer::utils;

#[derive(Clone, Debug)]
pub struct PlainTextRenderer {
    line_length: usize,
}

#[derive(Clone, Debug, Default)]
struct Decorator {
    links: Vec<String>,
    ignore_next_link: bool,
}

impl PlainTextRenderer {
    pub fn new(args: &args::ViewerArgs) -> Self {
        Self {
            line_length: utils::get_line_length(args),
        }
    }
}

impl utils::ManRenderer for PlainTextRenderer {
    type Error = io::Error;

    fn print_title(&mut self, left: &str, middle: &str, right: &str) -> io::Result<()> {
        let title = super::format_title(self.line_length, left, middle, right);
        writeln!(io::stdout(), "{}", title)?;
        writeln!(io::stdout())
    }

    fn print_text(&mut self, indent: u8, s: &doc::Text) -> io::Result<()> {
        let lines = html2text::from_read_with_decorator(
            s.html.as_bytes(),
            self.line_length - usize::from(indent),
            Decorator::new(),
        );
        for line in lines.trim().split('\n') {
            writeln!(io::stdout(), "{}{}", " ".repeat(indent.into()), line)?;
        }
        Ok(())
    }

    fn print_code(&mut self, indent: u8, code: &doc::Code) -> io::Result<()> {
        for line in code.split('\n') {
            writeln!(io::stdout(), "{}{}", " ".repeat(indent.into()), line)?;
        }
        Ok(())
    }

    fn print_heading(
        &mut self,
        indent: u8,
        s: &str,
        _link: Option<utils::DocLink>,
    ) -> io::Result<()> {
        writeln!(io::stdout(), "{}{}", " ".repeat(indent.into()), s)
    }

    fn println(&mut self) -> io::Result<()> {
        writeln!(io::stdout())
    }
}

impl Decorator {
    pub fn new() -> Self {
        Decorator::default()
    }
}

impl text_renderer::TextDecorator for Decorator {
    type Annotation = ();

    fn decorate_link_start(&mut self, url: &str) -> (String, Self::Annotation) {
        if super::list_link(url) {
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

    fn decorate_strikeout_start(&mut self) -> (String, Self::Annotation) {
        ("~".to_owned(), ())
    }

    fn decorate_strikeout_end(&mut self) -> String {
        "~".to_owned()
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
