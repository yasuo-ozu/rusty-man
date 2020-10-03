// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use html2text::render::text_renderer;

use crate::args;
use crate::doc;
use crate::viewer::utils;

type RichString = text_renderer::TaggedString<Vec<text_renderer::RichAnnotation>>;

#[derive(Debug)]
pub struct RichTextRenderer {
    line_length: usize,
    highlighter: Option<utils::Highlighter>,
}

impl RichTextRenderer {
    pub fn new(args: args::ViewerArgs) -> anyhow::Result<Self> {
        Ok(Self {
            line_length: utils::get_line_length(&args),
            highlighter: utils::get_highlighter(&args)?,
        })
    }
}

impl utils::ManRenderer for RichTextRenderer {
    type Error = io::Error;

    fn print_title(&mut self, left: &str, middle: &str, right: &str) -> io::Result<()> {
        let title = super::format_title(self.line_length, left, middle, right);
        render(text_style::StyledStr::plain(&title).bold())?;
        writeln!(io::stdout(), "\n")
    }

    fn print_text(&mut self, indent: u8, s: &doc::Text) -> io::Result<()> {
        let indent = usize::from(indent);
        let indent = if indent >= self.line_length / 2 {
            0
        } else {
            indent
        };
        let lines = html2text::from_read_rich(s.html.as_bytes(), self.line_length - indent);
        for line in lines {
            write!(io::stdout(), "{}", " ".repeat(indent))?;
            render_iter(
                line.iter()
                    .filter_map(|tle| match tle {
                        text_renderer::TaggedLineElement::Str(ts) => Some(ts),
                        _ => None,
                    })
                    .map(style_rich_string),
            )?;
            writeln!(io::stdout())?;
        }
        Ok(())
    }

    fn print_code(&mut self, indent: u8, code: &doc::Code) -> io::Result<()> {
        let indent = usize::from(indent);
        if let Some(highlighter) = &self.highlighter {
            for line in highlighter.highlight(code.as_ref()) {
                write!(io::stdout(), "{}", " ".repeat(indent))?;
                // We remove the background as we want to use the terminal background
                render_iter(line.iter().map(text_style::StyledStr::from).map(|mut s| {
                    s.style_mut().bg = None;
                    s
                }))?;
            }
            writeln!(io::stdout())?;
        } else {
            for line in code.split('\n') {
                writeln!(io::stdout(), "{}{}", " ".repeat(indent), line)?;
            }
        }

        Ok(())
    }

    fn print_heading(&mut self, indent: u8, s: &str) -> io::Result<()> {
        write!(io::stdout(), "{}", " ".repeat(usize::from(indent)))?;
        render(text_style::StyledStr::plain(s).bold())?;
        writeln!(io::stdout())
    }

    fn println(&mut self) -> io::Result<()> {
        writeln!(io::stdout())
    }
}

pub fn style_rich_string(ts: &RichString) -> text_style::StyledStr<'_> {
    use text_renderer::RichAnnotation;

    let mut s = text_style::StyledStr::plain(&ts.s);

    for annotation in &ts.tag {
        match annotation {
            RichAnnotation::Default => {}
            RichAnnotation::Link(_) => s.style_mut().set_bold(true),
            RichAnnotation::Image => {}
            RichAnnotation::Emphasis => s.style_mut().set_italic(true),
            RichAnnotation::Strikeout => {}
            RichAnnotation::Strong => s.style_mut().set_bold(true),
            RichAnnotation::Code => s.style_mut().set_fg(text_style::AnsiColor::Yellow.dark()),
            RichAnnotation::Preformat(_) => {}
        }
    }

    s
}

fn render<'a, S>(s: S) -> io::Result<()>
where
    S: Into<text_style::StyledStr<'a>>,
{
    text_style::ansi_term::render(io::stdout(), s)
}

fn render_iter<'a, I, S>(i: I) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<text_style::StyledStr<'a>>,
{
    text_style::ansi_term::render_iter(io::stdout(), i)
}
