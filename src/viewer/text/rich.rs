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
        write!(io::stdout(), "{}", crossterm::style::Attribute::Bold)?;
        super::print_title(self.line_length, left, middle, right)?;
        writeln!(io::stdout(), "{}", crossterm::style::Attribute::Reset)
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
        use crossterm::style::Attribute;
        let mut text = crossterm::style::style(s);
        text = text.attribute(Attribute::Bold).attribute(Attribute::Reset);
        writeln!(io::stdout(), "{}{}", " ".repeat(usize::from(indent)), &text)
    }

    fn println(&mut self) -> io::Result<()> {
        writeln!(io::stdout())
    }
}

pub fn style_syntect_string(
    style: syntect::highlighting::Style,
    s: &str,
) -> crossterm::style::StyledContent<&str> {
    use crossterm::style::{Attribute, Color};
    use syntect::highlighting::FontStyle;

    let mut content = crossterm::style::style(s).with(Color::Rgb {
        r: style.foreground.r,
        g: style.foreground.g,
        b: style.foreground.b,
    });
    if style.font_style.contains(FontStyle::BOLD) {
        // TODO: investigate why NoBold does not work
        content = content
            .attribute(Attribute::Bold)
            .attribute(Attribute::Reset);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        content = content.attribute(Attribute::Underlined);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        content = content.attribute(Attribute::Italic);
    }
    content
}

pub fn style_rich_string(ts: &RichString) -> crossterm::style::StyledContent<&str> {
    use crossterm::style::{Attribute, Color};
    use text_renderer::RichAnnotation;

    let mut content = crossterm::style::style(ts.s.as_ref());

    for annotation in &ts.tag {
        content = match annotation {
            RichAnnotation::Default => content,
            RichAnnotation::Link(_) => content.attribute(Attribute::Underlined),
            RichAnnotation::Image => content,
            RichAnnotation::Emphasis => content.attribute(Attribute::Italic),
            // TODO: investigate why NoBold does not work
            RichAnnotation::Strong => content
                .attribute(Attribute::Bold)
                .attribute(Attribute::Reset),
            RichAnnotation::Code => content.with(Color::Yellow),
            RichAnnotation::Preformat(_) => content,
        };
    }

    content
}
