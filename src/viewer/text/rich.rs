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
        let start_style = get_style(ts, get_start_style);
        let end_style = get_style(ts, get_end_style);
        write!(io::stdout(), "{}{}{}", start_style, ts.s, end_style)
    }
}

impl super::Printer for RichTextRenderer {
    fn print_title(&self, left: &str, middle: &str, right: &str) -> io::Result<()> {
        write!(io::stdout(), "{}", termion::style::Bold)?;
        super::print_title(self.line_length, left, middle, right)?;
        writeln!(io::stdout(), "{}", termion::style::Reset)
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
        let text = match level {
            1..=3 => format!("{}{}{}", termion::style::Bold, s, termion::style::Reset),
            _ => s.to_owned(),
        };
        writeln!(io::stdout(), "{}{}", " ".repeat(indent), &text)
    }

    fn println(&self) -> io::Result<()> {
        writeln!(io::stdout())
    }
}

fn get_style<F>(ts: &RichString, f: F) -> String
where
    F: Fn(&text_renderer::RichAnnotation) -> String,
{
    ts.tag.iter().map(f).collect::<Vec<_>>().join("")
}

fn get_start_style(annotation: &text_renderer::RichAnnotation) -> String {
    use termion::{color, style};
    use text_renderer::RichAnnotation;

    match annotation {
        RichAnnotation::Default => String::new(),
        RichAnnotation::Link(_) => style::Underline.to_string(),
        RichAnnotation::Image => String::new(),
        RichAnnotation::Emphasis => style::Italic.to_string(),
        RichAnnotation::Strong => style::Bold.to_string(),
        RichAnnotation::Code => color::Fg(color::LightYellow).to_string(),
        RichAnnotation::Preformat(_) => String::new(),
    }
}

fn get_end_style(annotation: &text_renderer::RichAnnotation) -> String {
    use termion::{color, style};
    use text_renderer::RichAnnotation;

    match annotation {
        RichAnnotation::Default => String::new(),
        RichAnnotation::Link(_) => style::NoUnderline.to_string(),
        RichAnnotation::Image => String::new(),
        RichAnnotation::Emphasis => style::NoItalic.to_string(),
        // TODO: investigate why NoBold does not work
        RichAnnotation::Strong => style::Reset.to_string(),
        RichAnnotation::Code => color::Fg(color::Reset).to_string(),
        RichAnnotation::Preformat(_) => String::new(),
    }
}
