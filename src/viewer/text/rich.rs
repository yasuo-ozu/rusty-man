// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use html2text::render::text_renderer;

use crate::args;
use crate::doc;
use crate::viewer;

type RichString = text_renderer::TaggedString<Vec<text_renderer::RichAnnotation>>;

#[derive(Debug)]
pub struct RichTextRenderer {
    line_length: usize,
    highlight: bool,
    syntax_set: syntect::parsing::SyntaxSet,
    theme: syntect::highlighting::Theme,
}

impl RichTextRenderer {
    fn render_string(&self, ts: &RichString) -> io::Result<()> {
        let content = get_styled_content(ts);
        write!(io::stdout(), "{}", content)
    }

    fn render_syntax(&self, line: &[(syntect::highlighting::Style, &str)]) -> io::Result<()> {
        use crossterm::style::{Attribute, Color};
        use syntect::highlighting::FontStyle;

        for (style, text) in line {
            let mut content = crossterm::style::style(text).with(Color::Rgb {
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
            write!(io::stdout(), "{}", content)?;
        }

        Ok(())
    }
}

impl super::Printer for RichTextRenderer {
    fn new(args: args::ViewerArgs) -> anyhow::Result<Self> {
        use anyhow::Context;

        let mut theme_set = syntect::highlighting::ThemeSet::load_defaults();
        let theme_name = args.theme.as_deref().unwrap_or("base16-eighties.dark");
        let theme = theme_set
            .themes
            .remove(theme_name)
            .with_context(|| format!("Could not find theme {}", theme_name))?;
        Ok(Self {
            line_length: viewer::get_line_length(),
            highlight: !args.no_syntax_highlight,
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme,
        })
    }

    fn print_title(&self, left: &str, middle: &str, right: &str) -> io::Result<()> {
        write!(io::stdout(), "{}", crossterm::style::Attribute::Bold)?;
        super::print_title(self.line_length, left, middle, right)?;
        writeln!(io::stdout(), "{}", crossterm::style::Attribute::Reset)
    }

    fn print_html(&self, indent: usize, s: &doc::Text, _show_links: bool) -> io::Result<()> {
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
                    self.render_string(ts)?;
                }
            }
            writeln!(io::stdout())?;
        }
        Ok(())
    }

    fn print_code(&self, indent: usize, code: &doc::Text) -> io::Result<()> {
        if self.highlight {
            let syntax = self.syntax_set.find_syntax_by_extension("rs").unwrap();
            let mut h = syntect::easy::HighlightLines::new(syntax, &self.theme);

            for line in syntect::util::LinesWithEndings::from(&code.plain) {
                let ranges = h.highlight(line, &self.syntax_set);
                write!(io::stdout(), "{}", " ".repeat(indent))?;
                self.render_syntax(&ranges)?;
            }
            writeln!(io::stdout())?;
        } else {
            self.print_html(indent, code, false)?;
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
