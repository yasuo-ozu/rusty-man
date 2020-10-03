// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use html2text::render::text_renderer;

use crate::args;
use crate::doc;
use crate::viewer::utils;

type RichString = text_renderer::TaggedString<Vec<text_renderer::RichAnnotation>>;
type RichLine = text_renderer::TaggedLine<Vec<text_renderer::RichAnnotation>>;

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

    fn prepare_html<'s>(&self, html: &'s [RichLine]) -> Vec<Vec<text_style::StyledStr<'s>>> {
        let mut lines = Vec::new();
        let mut highlight_lines = None;
        for line in html {
            let mut styled_strings = Vec::new();

            for ts in line.iter().filter_map(|tle| match tle {
                text_renderer::TaggedLineElement::Str(ts) => Some(ts),
                _ => None,
            }) {
                if let Some(highlighter) = &self.highlighter {
                    if is_pre(ts) {
                        let h = highlight_lines
                            .get_or_insert_with(|| highlighter.get_highlight_lines("rs"));
                        let highlighted_strings = h.highlight(&ts.s, &highlighter.syntax_set);
                        styled_strings.extend(
                            highlighted_strings
                                .iter()
                                .map(text_style::StyledStr::from)
                                .map(reset_background),
                        );
                    } else {
                        highlight_lines = None;
                        styled_strings.push(style_rich_string(ts));
                    }
                } else {
                    styled_strings.push(style_rich_string(ts));
                }
            }

            lines.push(styled_strings);
        }
        lines
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
        for line in self.prepare_html(&lines) {
            write!(io::stdout(), "{}", " ".repeat(indent))?;
            render_iter(line)?;
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
                render_iter(
                    line.iter()
                        .map(text_style::StyledStr::from)
                        .map(reset_background),
                )?;
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

fn reset_background(mut s: text_style::StyledStr<'_>) -> text_style::StyledStr<'_> {
    s.style_mut().bg = None;
    s
}

fn is_pre(ts: &RichString) -> bool {
    ts.tag.iter().any(|annotation| match annotation {
        text_renderer::RichAnnotation::Preformat(_) => true,
        _ => false,
    })
}

fn style_rich_string(ts: &RichString) -> text_style::StyledStr<'_> {
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
