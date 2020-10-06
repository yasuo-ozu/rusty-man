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

#[derive(Clone, Debug, Default)]
pub struct DocLink {
    pub name: doc::Fqn,
    pub ty: Option<doc::ItemType>,
}

/// A trait for viewer implementations that display the documentation in a man-like style.
pub trait ManRenderer {
    type Error: std::error::Error + Sized + Send;

    fn print_title(&mut self, left: &str, center: &str, right: &str) -> Result<(), Self::Error>;
    fn print_heading(
        &mut self,
        indent: u8,
        text: &str,
        link: Option<DocLink>,
    ) -> Result<(), Self::Error>;
    fn print_code(&mut self, indent: u8, code: &doc::Code) -> Result<(), Self::Error>;
    fn print_text(&mut self, indent: u8, text: &doc::Text) -> Result<(), Self::Error>;
    fn println(&mut self) -> Result<(), Self::Error>;

    fn render_doc(&mut self, doc: &doc::Doc) -> Result<(), Self::Error> {
        print_title(self, doc)?;

        if let Some(text) = &doc.definition {
            print_heading(self, 1, "Synopsis", None)?;
            self.print_code(6, text)?;
            self.println()?;
        }

        if let Some(text) = &doc.description {
            print_heading(self, 1, "Description", None)?;
            self.print_text(6, text)?;
            self.println()?;
        }

        for (ty, groups) in &doc.groups {
            print_heading(self, 1, ty.group_name(), None)?;

            for group in groups {
                if let Some(title) = &group.title {
                    print_heading(self, 2, title, None)?;
                }

                for member in &group.members {
                    let link = if doc::ItemType::Module == doc.ty {
                        Some(DocLink {
                            name: member.name.clone(),
                            ty: Some(*ty),
                        })
                    } else {
                        None
                    };
                    // TODO: use something link strip_prefix instead of last()
                    print_heading(self, 3, member.name.last(), link)?;
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
        print_heading(self, 1, "Examples", None)?;

        let n = examples.len();
        for (i, example) in examples.iter().enumerate() {
            if n > 1 {
                print_heading(self, 2, &format!("Example {} of {}", i + 1, n), None)?;
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
    link: Option<DocLink>,
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
    viewer.print_heading(indent, text.as_ref(), link)
}

/// Link handling mode for the [`RichDecorator`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkMode {
    /// Annotate links with `RichAnnotation::Link`.
    Annotate,
    /// List links at the end of the block.
    List,
}

/// A decorator that generates rich text.
#[derive(Clone)]
pub struct RichDecorator {
    link_filter: fn(&str) -> bool,
    link_mode: LinkMode,
    ignore_next_link: bool,
    links: Vec<String>,
}

impl RichDecorator {
    pub fn new(link_filter: fn(&str) -> bool, link_mode: LinkMode) -> RichDecorator {
        RichDecorator {
            link_filter,
            link_mode,
            ignore_next_link: false,
            links: Vec::new(),
        }
    }
}

impl text_renderer::TextDecorator for RichDecorator {
    type Annotation = text_renderer::RichAnnotation;

    fn decorate_link_start(&mut self, url: &str) -> (String, Self::Annotation) {
        self.ignore_next_link = !(self.link_filter)(url);
        if self.ignore_next_link {
            (String::new(), text_renderer::RichAnnotation::Default)
        } else {
            let annotation = text_renderer::RichAnnotation::Link(url.to_owned());
            match self.link_mode {
                LinkMode::Annotate => (String::new(), annotation),
                LinkMode::List => {
                    self.links.push(url.to_owned());
                    ("[".to_owned(), annotation)
                }
            }
        }
    }

    fn decorate_link_end(&mut self) -> String {
        if self.ignore_next_link {
            String::new()
        } else {
            match self.link_mode {
                LinkMode::Annotate => String::new(),
                LinkMode::List => format!("][{}]", self.links.len() - 1),
            }
        }
    }

    fn decorate_em_start(&mut self) -> (String, Self::Annotation) {
        ("".to_string(), text_renderer::RichAnnotation::Emphasis)
    }

    fn decorate_em_end(&mut self) -> String {
        "".to_string()
    }

    fn decorate_strong_start(&mut self) -> (String, Self::Annotation) {
        ("".to_string(), text_renderer::RichAnnotation::Strong)
    }

    fn decorate_strong_end(&mut self) -> String {
        "".to_string()
    }

    fn decorate_strikeout_start(&mut self) -> (String, Self::Annotation) {
        ("".to_string(), text_renderer::RichAnnotation::Strikeout)
    }

    fn decorate_strikeout_end(&mut self) -> String {
        "".to_string()
    }

    fn decorate_code_start(&mut self) -> (String, Self::Annotation) {
        ("".to_string(), text_renderer::RichAnnotation::Code)
    }

    fn decorate_code_end(&mut self) -> String {
        "".to_string()
    }

    fn decorate_preformat_first(&mut self) -> Self::Annotation {
        text_renderer::RichAnnotation::Preformat(false)
    }

    fn decorate_preformat_cont(&mut self) -> Self::Annotation {
        text_renderer::RichAnnotation::Preformat(true)
    }

    fn decorate_image(&mut self, title: &str) -> (String, Self::Annotation) {
        (title.to_string(), text_renderer::RichAnnotation::Image)
    }

    fn finalise(self) -> Vec<text_renderer::TaggedLine<text_renderer::RichAnnotation>> {
        let mut lines = Vec::new();
        if self.link_mode == LinkMode::List {
            for (idx, link) in self.links.into_iter().enumerate() {
                let mut line = text_renderer::TaggedLine::new();
                line.push_str(text_renderer::TaggedString {
                    s: format!("[{}] ", idx),
                    tag: text_renderer::RichAnnotation::Default,
                });
                line.push_str(text_renderer::TaggedString {
                    s: link.clone(),
                    tag: text_renderer::RichAnnotation::Link(link),
                });
                lines.push(line);
            }
        }
        lines
    }

    fn make_subblock_decorator(&self) -> Self {
        RichDecorator::new(self.link_filter, self.link_mode)
    }
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
