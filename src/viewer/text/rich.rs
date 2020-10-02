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
    highlight: bool,
    syntax_set: syntect::parsing::SyntaxSet,
    theme: syntect::highlighting::Theme,
}

impl RichTextRenderer {
    pub fn new(args: args::ViewerArgs) -> anyhow::Result<Self> {
        Ok(Self {
            line_length: utils::get_line_length(&args),
            highlight: !args.no_syntax_highlight,
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme: utils::get_syntect_theme(&args)?,
        })
    }
}

impl utils::ManRenderer for RichTextRenderer {
    type Error = io::Error;

    fn print_title(&mut self, left: &str, middle: &str, right: &str) -> io::Result<()> {
        let title = super::format_title(self.line_length, left, middle, right);
        writeln!(
            io::stdout(),
            "{}",
            ansi_term::Style::new().bold().paint(title)
        )?;
        writeln!(io::stdout())
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
            for element in line.iter() {
                if let text_renderer::TaggedLineElement::Str(ts) = element {
                    write!(io::stdout(), "{}", style_rich_string(ts))?;
                }
            }
            writeln!(io::stdout())?;
        }
        Ok(())
    }

    fn print_code(&mut self, indent: u8, code: &doc::Code) -> io::Result<()> {
        let indent = usize::from(indent);
        if self.highlight {
            let syntax = self.syntax_set.find_syntax_by_extension("rs").unwrap();
            let mut h = syntect::easy::HighlightLines::new(syntax, &self.theme);

            for line in syntect::util::LinesWithEndings::from(code.as_ref()) {
                let ranges = h.highlight(line, &self.syntax_set);
                write!(io::stdout(), "{}", " ".repeat(indent))?;
                for (style, text) in ranges {
                    let content = style_syntect_string(style, text);
                    write!(io::stdout(), "{}", content)?;
                }
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
        let text = ansi_term::Style::new().bold().paint(s);
        writeln!(io::stdout(), "{}{}", " ".repeat(usize::from(indent)), text)
    }

    fn println(&mut self) -> io::Result<()> {
        writeln!(io::stdout())
    }
}

pub fn style_syntect_string(
    style: syntect::highlighting::Style,
    s: &str,
) -> ansi_term::ANSIString<'_> {
    use syntect::highlighting::FontStyle;

    let mut text_style = ansi_term::Style::new();
    text_style.foreground = Some(ansi_term::Color::RGB(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ));
    if style.font_style.contains(FontStyle::BOLD) {
        text_style.is_bold = true;
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        text_style.is_underline = true;
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        text_style.is_italic = true;
    }
    text_style.paint(s)
}

pub fn style_rich_string(ts: &RichString) -> ansi_term::ANSIString<'_> {
    use text_renderer::RichAnnotation;

    let mut style = ansi_term::Style::new();

    for annotation in &ts.tag {
        match annotation {
            RichAnnotation::Default => {}
            RichAnnotation::Link(_) => {
                style.is_underline = true;
            }
            RichAnnotation::Image => {}
            RichAnnotation::Emphasis => {
                style.is_italic = true;
            }
            RichAnnotation::Strong => {
                style.is_bold = true;
            }
            RichAnnotation::Code => {
                style.foreground = Some(ansi_term::Color::Yellow);
            }
            RichAnnotation::Preformat(_) => {}
        }
    }

    style.paint(&ts.s)
}
