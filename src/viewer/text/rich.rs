// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use html2text::render::text_renderer;

use crate::viewer;

type RichString = text_renderer::TaggedString<Vec<text_renderer::RichAnnotation>>;

#[derive(Clone, Debug)]
pub struct RichTextRenderer {
    line_length: usize,
}

impl RichTextRenderer {
    pub fn new() -> Self {
        Self {
            line_length: viewer::get_line_length(),
        }
    }

    fn render_string(&self, ts: &RichString) -> io::Result<()> {
        let content = get_styled_content(ts);
        write!(io::stdout(), "{}", content)
    }
}

impl super::Printer for RichTextRenderer {
    fn print_title(&self, left: &str, middle: &str, right: &str) -> io::Result<()> {
        write!(io::stdout(), "{}", crossterm::style::Attribute::Bold)?;
        super::print_title(self.line_length, left, middle, right)?;
        writeln!(io::stdout(), "{}", crossterm::style::Attribute::Reset)
    }

    fn print_html(&self, indent: usize, s: &str, _show_links: bool) -> io::Result<()> {
        let indent = if indent >= self.line_length / 2 {
            0
        } else {
            indent
        };
        let lines = html2text::from_read_rich(s.as_bytes(), self.line_length - indent);
        for line in lines {
            write!(io::stdout(), "{}", " ".repeat(indent))?;
            for element in line.iter() {
                if let text_renderer::TaggedLineElement::Str(ts) = element {
                    self.render_string(ts)?;
                }
            }
            writeln!(io::stdout())?;
        }
        Ok(())
    }

    fn print_heading(&self, indent: usize, level: usize, s: &str) -> io::Result<()> {
        let mut text = crossterm::style::style(s);
        if level < 4 {
            use crossterm::style::Attribute;
            text = text.attribute(Attribute::Bold).attribute(Attribute::Reset);
        }
        writeln!(io::stdout(), "{}{}", " ".repeat(indent), &text)
    }

    fn println(&self) -> io::Result<()> {
        writeln!(io::stdout())
    }
}

fn get_styled_content(ts: &RichString) -> crossterm::style::StyledContent<&str> {
    use crossterm::style::{Attribute, Color};
    use text_renderer::RichAnnotation;

    let mut content = crossterm::style::style(ts.s.as_ref());

    for annotation in &ts.tag {
        content = match annotation {
            RichAnnotation::Default => content,
            RichAnnotation::Link(_) => content.attribute(Attribute::Underlined),
            RichAnnotation::Image => content,
            RichAnnotation::Emphasis => content.attribute(Attribute::Italic),
            // TODO: investigeate why NoBold does not work
            RichAnnotation::Strong => content
                .attribute(Attribute::Bold)
                .attribute(Attribute::Reset),
            RichAnnotation::Code => content.with(Color::Yellow),
            RichAnnotation::Preformat(_) => content,
        };
    }

    content
}
