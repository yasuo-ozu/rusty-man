// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::cmp;
use std::iter;
use std::rc;

use cursive::{event, theme, utils::markup};
use html2text::render::text_renderer;

use crate::viewer::utils;

pub struct HtmlView {
    render_tree: html2text::RenderTree,
    highlighter: Option<utils::Highlighter>,
    max_width: usize,
    rendered_html: Option<RenderedHtml>,
    focus: Option<usize>,
    on_link: Option<rc::Rc<dyn Fn(&mut cursive::Cursive, String)>>,
}

impl HtmlView {
    pub fn new(html: &str, highlighter: Option<utils::Highlighter>, max_width: usize) -> HtmlView {
        HtmlView {
            render_tree: html2text::parse(html.as_bytes()),
            highlighter,
            max_width,
            rendered_html: None,
            focus: None,
            on_link: None,
        }
    }

    pub fn set_on_link<F>(&mut self, cb: F)
    where
        F: Fn(&mut cursive::Cursive, String) + 'static,
    {
        self.on_link = Some(rc::Rc::new(cb));
    }

    fn render(&self, width: usize) -> RenderedHtml {
        let mut rendered_html = RenderedHtml::new(width);
        let decorator = utils::RichDecorator::new(show_link, utils::LinkMode::Annotate);
        let raw_lines = self
            .render_tree
            .clone()
            .render(width, decorator)
            .into_lines();
        let highlighted_lines = utils::highlight_html(&raw_lines, self.highlighter.as_ref());
        for (y, line) in highlighted_lines.enumerate() {
            rendered_html.push_line(y, line);
        }
        rendered_html
    }

    fn update(&mut self, constraint: cursive::XY<usize>) -> cursive::XY<usize> {
        let width = cmp::min(self.max_width, constraint.x);

        // If we already have rendered the tree with the same width, we can reuse the cached data.
        if let Some(rendered_html) = &self.rendered_html {
            if rendered_html.width == width {
                return rendered_html.size;
            }
        }

        let rendered_html = self.render(width);

        // Due to changed wrapping, the link count may have changed.  So we have to make sure that
        // our focus is still valid.
        if let Some(focus) = self.focus {
            // TODO: Ideally, we would also want to adjust the focus if a previous link was
            // re-wrapped.
            if focus >= rendered_html.links.len() {
                self.focus = Some(rendered_html.links.len() - 1);
            }
        }

        let size = rendered_html.size;
        self.rendered_html = Some(rendered_html);
        size
    }
}

impl cursive::View for HtmlView {
    fn draw(&self, printer: &cursive::Printer) {
        let lines = &self
            .rendered_html
            .as_ref()
            .expect("layout not called before draw")
            .lines;
        for (y, line) in lines.iter().enumerate() {
            let mut x = 0;
            for element in line {
                let mut style = element.style;
                if element.link_idx == self.focus && printer.focused {
                    style = style.combine(theme::PaletteColor::Highlight);
                }
                printer.with_style(style, |printer| printer.print((x, y), &element.text));
                x += element.text.len();
            }
        }
    }

    fn layout(&mut self, constraint: cursive::XY<usize>) {
        self.update(constraint);
    }

    fn required_size(&mut self, constraint: cursive::XY<usize>) -> cursive::XY<usize> {
        self.update(constraint)
    }

    fn take_focus(&mut self, direction: cursive::direction::Direction) -> bool {
        let link_count = self
            .rendered_html
            .as_ref()
            .map(|html| html.links.len())
            .unwrap_or_default();
        if link_count > 0 {
            use cursive::direction::{Absolute, Direction, Relative};
            let focus = match direction {
                Direction::Abs(abs) => match abs {
                    Absolute::Up | Absolute::Left | Absolute::None => 0,
                    Absolute::Down | Absolute::Right => link_count - 1,
                },
                Direction::Rel(rel) => match rel {
                    Relative::Front => 0,
                    Relative::Back => link_count - 1,
                },
            };
            self.focus = Some(focus);
            true
        } else {
            false
        }
    }

    fn on_event(&mut self, event: event::Event) -> event::EventResult {
        use event::{Event, EventResult, Key};

        let links = if let Some(rendered_html) = &self.rendered_html {
            if rendered_html.links.is_empty() {
                return EventResult::Ignored;
            } else {
                &rendered_html.links
            }
        } else {
            return EventResult::Ignored;
        };
        let focus = if let Some(focus) = self.focus {
            focus
        } else {
            return EventResult::Ignored;
        };

        match event {
            Event::Key(Key::Left) => {
                if focus == 0 {
                    EventResult::Ignored
                } else if links[focus].position.y == links[focus - 1].position.y {
                    self.focus = Some(focus - 1);
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            Event::Key(Key::Up) => {
                let y = links[focus].position.y;
                let next_focus = links[..focus]
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, link)| link.position.y < y)
                    .map(|(idx, _)| idx);
                match next_focus {
                    Some(focus) => {
                        self.focus = Some(focus);
                        EventResult::Consumed(None)
                    }
                    None => EventResult::Ignored,
                }
            }
            Event::Key(Key::Down) => {
                let y = links[focus].position.y;
                let next_focus = links
                    .iter()
                    .enumerate()
                    .skip(focus)
                    .find(|(_, link)| link.position.y > y)
                    .map(|(idx, _)| idx);
                match next_focus {
                    Some(focus) => {
                        self.focus = Some(focus);
                        EventResult::Consumed(None)
                    }
                    None => EventResult::Ignored,
                }
            }
            Event::Key(Key::Right) => {
                if focus + 1 >= links.len() {
                    EventResult::Ignored
                } else if links[focus].position.y == links[focus + 1].position.y {
                    self.focus = Some(focus + 1);
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            Event::Key(Key::Enter) => {
                let link = links[focus].target.clone();
                let cb = self
                    .on_link
                    .clone()
                    .map(|cb| event::Callback::from_fn(move |s| cb(s, link.clone())));
                EventResult::Consumed(cb)
            }
            _ => EventResult::Ignored,
        }
    }

    fn important_area(&self, _: cursive::XY<usize>) -> cursive::Rect {
        if let Some((focus, rendered_html)) = self.focus.zip(self.rendered_html.as_ref()) {
            let origin = rendered_html.links[focus].position;
            cursive::Rect::from_size(origin, (rendered_html.links[focus].width, 1))
        } else {
            cursive::Rect::from((0, 0))
        }
    }
}

fn show_link(url: &str) -> bool {
    // We don’t want to show fragment links as we cannot jump to HTML elements by ID
    !url.starts_with('#')
}

#[derive(Clone, Debug)]
struct HtmlElement {
    text: String,
    style: theme::Style,
    link_idx: Option<usize>,
}

#[derive(Clone, Debug)]
struct Link {
    position: cursive::XY<usize>,
    target: String,
    width: usize,
}

#[derive(Clone, Debug)]
struct RenderedHtml {
    width: usize,
    size: cursive::XY<usize>,
    lines: Vec<Vec<HtmlElement>>,
    links: Vec<Link>,
}

impl RenderedHtml {
    pub fn new(width: usize) -> RenderedHtml {
        RenderedHtml {
            width,
            size: (0, 0).into(),
            lines: Vec::new(),
            links: Vec::new(),
        }
    }

    pub fn push_link(&mut self, link: Link) -> usize {
        self.links.push(link);
        self.links.len() - 1
    }

    pub fn push_line(&mut self, y: usize, elements: Vec<utils::HighlightedHtmlElement>) {
        let mut len = 0;
        let mut line = Vec::new();

        for element in elements {
            let element = match element {
                utils::HighlightedHtmlElement::RichString(ts) => {
                    let tag: Tag = ts.tag.iter().collect();
                    HtmlElement {
                        text: ts.s.clone(),
                        style: tag.style,
                        link_idx: tag.link_target.map(|target| {
                            self.push_link(Link {
                                position: (len, y).into(),
                                target,
                                width: ts.s.len(),
                            })
                        }),
                    }
                }
                utils::HighlightedHtmlElement::StyledString(s) => {
                    let s = utils::reset_background(s);
                    HtmlElement {
                        text: s.s.to_owned(),
                        style: s.style.map_or_else(Default::default, From::from),
                        link_idx: None,
                    }
                }
            };

            len += element.text.len();
            line.push(element);
        }

        self.lines.push(line);
        self.size = self.size.stack_vertical(&(len, 1).into());
    }
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
