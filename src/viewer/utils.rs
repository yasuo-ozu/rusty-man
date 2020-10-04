// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::cmp;

use anyhow::Context as _;
use html2text::render::text_renderer;

use crate::args;
use crate::doc;

pub type RichString = text_renderer::TaggedString<Vec<text_renderer::RichAnnotation>>;
pub type RichLine = text_renderer::TaggedLine<Vec<text_renderer::RichAnnotation>>;

/// A helper struct for syntax highlighting using syntect.
#[derive(Clone, Debug)]
pub struct Highlighter {
    pub syntax_set: syntect::parsing::SyntaxSet,
    pub theme: syntect::highlighting::Theme,
}

impl Highlighter {
    pub fn new(args: &args::ViewerArgs) -> anyhow::Result<Highlighter> {
        Ok(Highlighter {
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme: get_syntect_theme(args)?,
        })
    }

    pub fn highlight<'a, 's>(
        &'a self,
        s: &'s str,
    ) -> HighlightedLines<'s, 'a, 'a, syntect::util::LinesWithEndings<'s>> {
        HighlightedLines::new(
            syntect::util::LinesWithEndings::from(s),
            self.get_highlight_lines("rs"),
            &self.syntax_set,
        )
    }

    pub fn get_highlight_lines(&self, syntax: &str) -> syntect::easy::HighlightLines<'_> {
        let syntax = self.syntax_set.find_syntax_by_extension(syntax).unwrap();
        syntect::easy::HighlightLines::new(syntax, &self.theme)
    }
}

/// An iterator over lines highlighted using syntect.
pub struct HighlightedLines<'s, 'ss, 't, I: Iterator<Item = &'s str>> {
    iter: I,
    highlighter: syntect::easy::HighlightLines<'t>,
    syntax_set: &'ss syntect::parsing::SyntaxSet,
}

impl<'s, 'ss, 't, I: Iterator<Item = &'s str>> HighlightedLines<'s, 'ss, 't, I> {
    fn new(
        iter: I,
        highlighter: syntect::easy::HighlightLines<'t>,
        syntax_set: &'ss syntect::parsing::SyntaxSet,
    ) -> HighlightedLines<'s, 'ss, 't, I> {
        HighlightedLines {
            iter,
            highlighter,
            syntax_set,
        }
    }
}

impl<'s, 'ss, 't, I: Iterator<Item = &'s str>> Iterator for HighlightedLines<'s, 'ss, 't, I> {
    type Item = Vec<(syntect::highlighting::Style, &'s str)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|s| self.highlighter.highlight(s, &self.syntax_set))
    }
}

pub enum HighlightedHtmlElement<'s> {
    RichString(&'s RichString),
    StyledString(text_style::StyledStr<'s>),
}

impl<'s> From<&'s RichString> for HighlightedHtmlElement<'s> {
    fn from(s: &'s RichString) -> HighlightedHtmlElement<'s> {
        HighlightedHtmlElement::RichString(s)
    }
}

impl<'s> From<text_style::StyledStr<'s>> for HighlightedHtmlElement<'s> {
    fn from(s: text_style::StyledStr<'s>) -> HighlightedHtmlElement<'s> {
        HighlightedHtmlElement::StyledString(s)
    }
}

pub struct HighlightedHtml<'h, 's, I: Iterator<Item = &'s RichLine>> {
    iter: I,
    highlighter: Option<&'h Highlighter>,
    highlight_lines: Option<syntect::easy::HighlightLines<'h>>,
}

impl<'h, 's, I: Iterator<Item = &'s RichLine>> HighlightedHtml<'h, 's, I> {
    fn new(iter: I, highlighter: Option<&'h Highlighter>) -> HighlightedHtml<'h, 's, I> {
        HighlightedHtml {
            iter,
            highlighter,
            highlight_lines: None,
        }
    }

    fn get_highlighted_line(
        &mut self,
        highlighter: &'h Highlighter,
        line: &'s RichLine,
    ) -> Vec<HighlightedHtmlElement<'s>> {
        let mut elements = Vec::new();

        for ts in line.iter().filter_map(|tle| match tle {
            text_renderer::TaggedLineElement::Str(ts) => Some(ts),
            _ => None,
        }) {
            if is_pre(ts) {
                let h = self
                    .highlight_lines
                    .get_or_insert_with(|| highlighter.get_highlight_lines("rs"));

                // TODO: syntect expects a newline

                let strings = h.highlight(&ts.s, &highlighter.syntax_set);
                elements.extend(
                    strings
                        .iter()
                        .map(text_style::StyledStr::from)
                        .map(HighlightedHtmlElement::from),
                );
            } else {
                self.highlight_lines = None;
                elements.push(ts.into());
            }
        }

        elements
    }
}

impl<'h, 's, I: Iterator<Item = &'s RichLine>> Iterator for HighlightedHtml<'h, 's, I> {
    type Item = Vec<HighlightedHtmlElement<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(line) = self.iter.next() {
            let elements = if let Some(highlighter) = self.highlighter {
                self.get_highlighted_line(highlighter, line)
            } else {
                line.iter()
                    .filter_map(|tle| match tle {
                        text_renderer::TaggedLineElement::Str(ts) => Some(ts),
                        _ => None,
                    })
                    .map(From::from)
                    .collect()
            };
            Some(elements)
        } else {
            None
        }
    }
}

pub fn highlight_html<'h, 's, I, Iter>(
    iter: I,
    highlighter: Option<&'h Highlighter>,
) -> HighlightedHtml<'h, 's, Iter>
where
    I: IntoIterator<Item = Iter::Item, IntoIter = Iter>,
    Iter: Iterator<Item = &'s RichLine>,
{
    HighlightedHtml::new(iter.into_iter(), highlighter)
}

fn is_pre(ts: &RichString) -> bool {
    ts.tag.iter().any(|annotation| match annotation {
        text_renderer::RichAnnotation::Preformat(_) => true,
        _ => false,
    })
}

/// A trait for viewer implementations that display the documentation in a man-like style.
pub trait ManRenderer {
    type Error: std::error::Error + Sized + Send;

    fn print_title(&mut self, left: &str, center: &str, right: &str) -> Result<(), Self::Error>;
    fn print_heading(&mut self, indent: u8, text: &str) -> Result<(), Self::Error>;
    fn print_code(&mut self, indent: u8, code: &doc::Code) -> Result<(), Self::Error>;
    fn print_text(&mut self, indent: u8, text: &doc::Text) -> Result<(), Self::Error>;
    fn println(&mut self) -> Result<(), Self::Error>;

    fn render_doc(&mut self, doc: &doc::Doc) -> Result<(), Self::Error> {
        print_title(self, doc)?;

        if let Some(text) = &doc.definition {
            print_heading(self, 1, "Synopsis")?;
            self.print_code(6, text)?;
            self.println()?;
        }

        if let Some(text) = &doc.description {
            print_heading(self, 1, "Description")?;
            self.print_text(6, text)?;
            self.println()?;
        }

        for (ty, groups) in &doc.groups {
            print_heading(self, 1, ty.group_name())?;

            for group in groups {
                if let Some(title) = &group.title {
                    print_heading(self, 2, title)?;
                }

                for member in &group.members {
                    // TODO: use something link strip_prefix instead of last()
                    print_heading(self, 3, member.name.last())?;
                    if let Some(definition) = &member.definition {
                        self.print_code(12, definition)?;
                    }
                    if member.definition.is_some() && member.description.is_some() {
                        self.println()?;
                    }
                    if let Some(description) = &member.description {
                        self.print_text(12, description)?;
                    }
                    if member.definition.is_some() || member.description.is_some() {
                        self.println()?;
                    }
                }
            }
        }

        Ok(())
    }

    fn render_examples(
        &mut self,
        doc: &doc::Doc,
        examples: &[doc::Example],
    ) -> Result<(), Self::Error> {
        print_title(self, doc)?;
        print_heading(self, 1, "Examples")?;

        let n = examples.len();
        for (i, example) in examples.iter().enumerate() {
            if n > 1 {
                print_heading(self, 2, &format!("Example {} of {}", i + 1, n))?;
            }
            if let Some(description) = &example.description {
                self.print_text(6, description)?;
                self.println()?;
            }
            self.print_code(6, &example.code)?;
            self.println()?;
        }

        Ok(())
    }
}

fn print_title<M: ManRenderer + ?Sized>(viewer: &mut M, doc: &doc::Doc) -> Result<(), M::Error> {
    let title = format!("{} {}", doc.ty.name(), doc.name);
    viewer.print_title(doc.name.krate(), &title, "rusty-man")
}

fn print_heading<M: ManRenderer + ?Sized>(
    viewer: &mut M,
    level: u8,
    text: &str,
) -> Result<(), M::Error> {
    let text = match level {
        1 => std::borrow::Cow::from(text.to_uppercase()),
        _ => std::borrow::Cow::from(text),
    };
    let indent = match level {
        1 => 0,
        2 => 3,
        _ => 6,
    };
    viewer.print_heading(indent, text.as_ref())
}

pub fn get_line_length(args: &args::ViewerArgs) -> usize {
    if let Some(width) = args.width {
        width
    } else if let Some((terminal_size::Width(width), _)) = terminal_size::terminal_size() {
        cmp::min(width.into(), args.max_width.unwrap_or(100))
    } else {
        args.max_width.unwrap_or(100)
    }
}

pub fn get_highlighter(args: &args::ViewerArgs) -> anyhow::Result<Option<Highlighter>> {
    if args.no_syntax_highlight {
        Ok(None)
    } else {
        Highlighter::new(&args).map(Some)
    }
}

pub fn reset_background(mut s: text_style::StyledStr<'_>) -> text_style::StyledStr<'_> {
    s.style_mut().bg = None;
    s
}

fn get_syntect_theme(args: &args::ViewerArgs) -> anyhow::Result<syntect::highlighting::Theme> {
    let mut theme_set = syntect::highlighting::ThemeSet::load_defaults();
    let theme_name = args.theme.as_deref().unwrap_or("base16-eighties.dark");
    theme_set
        .themes
        .remove(theme_name)
        .with_context(|| format!("Could not find theme {}", theme_name))
}
