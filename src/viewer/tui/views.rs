// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::cmp;
use std::iter;

use cursive::{event, theme, utils::markup};
use html2text::render::text_renderer;

use crate::viewer::utils;

pub struct HtmlRenderer {
    render_tree: html2text::RenderTree,
    highlighter: Option<utils::Highlighter>,
}

impl HtmlRenderer {
    pub fn new(html: &str, highlighter: Option<utils::Highlighter>) -> HtmlRenderer {
        HtmlRenderer {
            render_tree: html2text::parse(html.as_bytes()),
            highlighter,
        }
    }
}

impl cursive_markup::Renderer for HtmlRenderer {
    fn render(&self, constraint: cursive::XY<usize>) -> cursive_markup::RenderedDocument {
        let decorator = utils::RichDecorator::new(show_link, utils::LinkMode::Annotate);
        let raw_lines = self
            .render_tree
            .clone()
            .render(constraint.x, decorator)
            .into_lines();
        let highlighted_lines = utils::highlight_html(&raw_lines, self.highlighter.as_ref());
        let mut doc = cursive_markup::RenderedDocument::new(constraint);
        for line in highlighted_lines {
            doc.push_line(line.into_iter().map(From::from))
        }
        doc
    }
}

impl<'s> From<utils::HighlightedHtmlElement<'s>> for cursive_markup::Element {
    fn from(e: utils::HighlightedHtmlElement<'s>) -> cursive_markup::Element {
        match e {
            utils::HighlightedHtmlElement::RichString(ts) => {
                let tag: Tag = ts.tag.iter().collect();
                cursive_markup::Element::new(ts.s.clone(), tag.style, tag.link_target)
            }
            utils::HighlightedHtmlElement::StyledString(s) => {
                let s = utils::reset_background(s);
                cursive_markup::Element::styled(
                    s.s.to_owned(),
                    s.style.map_or_else(Default::default, From::from),
                )
            }
        }
    }
}

fn show_link(url: &str) -> bool {
    // We don’t want to show fragment links as we cannot jump to HTML elements by ID
    !url.starts_with('#')
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Tag {
    style: theme::Style,
    link_target: Option<String>,
}

impl<'a> iter::FromIterator<&'a text_renderer::RichAnnotation> for Tag {
    fn from_iter<I: IntoIterator<Item = &'a text_renderer::RichAnnotation>>(iter: I) -> Tag {
        let mut tag = Tag::default();
        for annotation in iter {
            if let text_renderer::RichAnnotation::Link(target) = annotation {
                tag.link_target = Some(target.clone());
            }
            if let Some(style) = get_rich_style(annotation) {
                tag.style = tag.style.combine(style);
            }
        }
        tag
    }
}

fn get_rich_style(annotation: &text_renderer::RichAnnotation) -> Option<theme::Style> {
    use text_renderer::RichAnnotation;

    match annotation {
        RichAnnotation::Default => None,
        RichAnnotation::Link(_) => Some(theme::Effect::Underline.into()),
        RichAnnotation::Image => None,
        RichAnnotation::Emphasis => Some(theme::Effect::Italic.into()),
        RichAnnotation::Strong => Some(theme::Effect::Bold.into()),
        RichAnnotation::Strikeout => Some(theme::Effect::Strikethrough.into()),
        RichAnnotation::Code => Some(theme::PaletteColor::Secondary.into()),
        RichAnnotation::Preformat(_) => None,
    }
}

pub struct LinkView {
    text: markup::StyledString,
    cb: event::Callback,
    is_focused: bool,
}

impl LinkView {
    pub fn new<F>(text: impl Into<markup::StyledString>, cb: F) -> LinkView
    where
        F: Fn(&mut cursive::Cursive) + 'static,
    {
        LinkView {
            text: text.into(),
            cb: event::Callback::from_fn(cb),
            is_focused: false,
        }
    }
}

impl cursive::View for LinkView {
    fn draw(&self, printer: &cursive::Printer) {
        let mut style = theme::Style::from(theme::Effect::Underline);
        if self.is_focused && printer.focused {
            style = style.combine(theme::PaletteColor::Highlight);
        };
        printer.with_style(style, |printer| {
            printer.print_styled((0, 0), (&self.text).into())
        });
    }

    fn required_size(&mut self, _constraint: cursive::XY<usize>) -> cursive::XY<usize> {
        (self.text.width(), 1).into()
    }

    fn take_focus(&mut self, _direction: cursive::direction::Direction) -> bool {
        self.is_focused = true;
        true
    }

    fn on_event(&mut self, event: event::Event) -> event::EventResult {
        if event == event::Event::Key(event::Key::Enter) {
            event::EventResult::Consumed(Some(self.cb.clone()))
        } else {
            event::EventResult::Ignored
        }
    }
}

pub struct CodeView {
    lines: Vec<markup::StyledString>,
    width: usize,
}

impl CodeView {
    pub fn new(code: &str, highlighter: &utils::Highlighter) -> CodeView {
        let mut lines = Vec::new();
        let mut width = 0;
        for line in highlighter.highlight(code) {
            let s = line
                .iter()
                .map(text_style::StyledStr::from)
                .map(utils::reset_background)
                .map(markup::StyledString::from)
                .fold(markup::StyledString::new(), |mut acc, s| {
                    acc.append(s);
                    acc
                });
            width = cmp::max(width, s.width());
            lines.push(s);
        }
        CodeView { lines, width }
    }
}

impl cursive::View for CodeView {
    fn draw(&self, printer: &cursive::Printer) {
        for (y, line) in self.lines.iter().enumerate() {
            printer.print_styled((0, y), line.into());
        }
    }

    fn required_size(&mut self, _constraint: cursive::XY<usize>) -> cursive::XY<usize> {
        (self.width, self.lines.len()).into()
    }
}
