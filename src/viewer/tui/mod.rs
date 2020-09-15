// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod views;

use std::convert;

use anyhow::Context as _;
use cursive::view::{Resizable as _, Scrollable as _};
use cursive::views::{Dialog, LinearLayout, PaddedView, Panel, TextView};
use cursive::{event, theme};

use crate::args;
use crate::doc;
use crate::source;
use crate::viewer::{self, utils, utils::ManRenderer as _};

use views::{CodeView, HtmlView};

#[derive(Clone, Debug)]
pub struct TuiViewer {}

impl TuiViewer {
    pub fn new() -> TuiViewer {
        TuiViewer {}
    }

    fn render<F>(
        &self,
        sources: Vec<Box<dyn source::Source>>,
        args: args::ViewerArgs,
        doc: &doc::Doc,
        f: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(&mut TuiManRenderer) -> Result<(), convert::Infallible>,
    {
        let mut s = create_cursive(sources, args)?;
        let mut renderer = context(&mut s).create_renderer(&doc);
        f(&mut renderer)?;
        let view = renderer.into_view();
        s.add_fullscreen_layer(view);
        s.run();
        Ok(())
    }
}

impl viewer::Viewer for TuiViewer {
    fn open(
        &self,
        sources: Vec<Box<dyn source::Source>>,
        args: args::ViewerArgs,
        doc: &doc::Doc,
    ) -> anyhow::Result<()> {
        self.render(sources, args, doc, |renderer| renderer.render_doc(doc))
    }

    fn open_examples(
        &self,
        sources: Vec<Box<dyn source::Source>>,
        args: args::ViewerArgs,
        doc: &doc::Doc,
        examples: Vec<doc::Example>,
    ) -> anyhow::Result<()> {
        self.render(sources, args, doc, |renderer| {
            renderer.render_examples(doc, &examples)
        })
    }
}

pub struct Context {
    pub sources: Vec<Box<dyn source::Source>>,
    pub args: args::ViewerArgs,
    pub highlighter: Option<utils::Highlighter>,
}

impl Context {
    pub fn new(
        sources: Vec<Box<dyn source::Source>>,
        args: args::ViewerArgs,
    ) -> anyhow::Result<Context> {
        let highlighter = utils::get_highlighter(&args)?;
        Ok(Context {
            sources,
            args,
            highlighter,
        })
    }

    pub fn create_renderer(&self, doc: &doc::Doc) -> TuiManRenderer {
        TuiManRenderer::new(
            doc,
            self.args.max_width.unwrap_or(100),
            self.highlighter.as_ref(),
        )
    }
}

pub struct TuiManRenderer<'s> {
    doc_name: doc::Fqn,
    doc_ty: doc::ItemType,
    layout: LinearLayout,
    max_width: usize,
    highlighter: Option<&'s utils::Highlighter>,
}

impl<'s> TuiManRenderer<'s> {
    pub fn new(
        doc: &doc::Doc,
        max_width: usize,
        highlighter: Option<&'s utils::Highlighter>,
    ) -> TuiManRenderer<'s> {
        TuiManRenderer {
            doc_name: doc.name.clone(),
            doc_ty: doc.ty,
            layout: LinearLayout::vertical(),
            max_width,
            highlighter,
        }
    }

    fn into_view(self) -> impl cursive::View {
        let title = format!("{} {}", self.doc_ty.name(), self.doc_name);
        let scroll = self.layout.scrollable().full_screen();
        Panel::new(scroll).title(title)
    }
}

impl<'s> utils::ManRenderer for TuiManRenderer<'s> {
    type Error = convert::Infallible;

    fn print_title(&mut self, _left: &str, _center: &str, _right: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    fn print_heading(&mut self, indent: u8, text: &str) -> Result<(), Self::Error> {
        let heading = TextView::new(text).effect(theme::Effect::Bold);
        self.layout.add_child(indent_view(indent, heading));
        Ok(())
    }

    fn print_code(&mut self, indent: u8, code: &doc::Code) -> Result<(), Self::Error> {
        if let Some(highlighter) = self.highlighter {
            let code = CodeView::new(&code.to_string(), highlighter);
            self.layout.add_child(indent_view(indent, code));
        } else {
            let text = TextView::new(code.to_string());
            self.layout.add_child(indent_view(indent, text));
        }
        Ok(())
    }

    fn print_text(&mut self, indent: u8, text: &doc::Text) -> Result<(), Self::Error> {
        let indent = usize::from(indent);
        let mut text = HtmlView::new(
            &text.html,
            self.highlighter.cloned(),
            self.max_width.saturating_sub(indent),
        );
        let doc_name = self.doc_name.clone();
        let doc_ty = self.doc_ty;
        text.set_on_link(move |s, link| handle_link(s, &doc_name.clone(), doc_ty, link));
        self.layout.add_child(indent_view(indent, text));
        Ok(())
    }

    fn println(&mut self) -> Result<(), Self::Error> {
        self.layout.add_child(TextView::new(" "));
        Ok(())
    }
}

fn indent_view<V>(indent: impl Into<usize>, view: V) -> PaddedView<V> {
    PaddedView::lrtb(indent.into(), 0, 0, 0, view)
}

fn create_cursive(
    sources: Vec<Box<dyn source::Source>>,
    args: args::ViewerArgs,
) -> anyhow::Result<cursive::Cursive> {
    let mut cursive = cursive::default();

    cursive.set_user_data(Context::new(sources, args)?);

    cursive.add_global_callback('q', |s| s.quit());
    cursive.add_global_callback(event::Key::Backspace, |s| {
        let screen = s.screen_mut();
        if screen.len() > 1 {
            screen.pop_layer();
        }
    });

    let mut theme = theme::Theme::default();
    theme.shadow = false;
    theme.palette[theme::PaletteColor::Background] = theme::Color::TerminalDefault;
    theme.palette[theme::PaletteColor::View] = theme::Color::TerminalDefault;
    theme.palette[theme::PaletteColor::Primary] = theme::Color::TerminalDefault;
    cursive.set_theme(theme);

    Ok(cursive)
}

fn context(s: &mut cursive::Cursive) -> &mut Context {
    s.user_data()
        .expect("Missing context in cursive application")
}

fn report_error(s: &mut cursive::Cursive, error: anyhow::Error) {
    let context: Vec<_> = error
        .chain()
        .skip(1)
        .map(|e| format!("    {}", e.to_string()))
        .collect();

    let mut msg = error.to_string();
    if !context.is_empty() {
        msg.push_str("\n\nContext:\n");
        msg.push_str(&context.join("\n"));
    }

    let dialog = Dialog::info(msg).title("Error");
    s.add_layer(dialog);
}

fn handle_link(s: &mut cursive::Cursive, doc_name: &doc::Fqn, doc_ty: doc::ItemType, link: String) {
    let result = resolve_link(doc_name, doc_ty, link).and_then(|link| open_link(s, link));
    if let Err(err) = result {
        report_error(s, err);
    }
}

fn find_doc(
    sources: &[Box<dyn source::Source>],
    ty: Option<doc::ItemType>,
    name: &doc::Fqn,
) -> anyhow::Result<doc::Doc> {
    for source in sources {
        if let Some(doc) = source.find_doc(name, ty)? {
            return Ok(doc);
        }
    }
    Err(anyhow::anyhow!(
        "Could not find documentation for item: {}",
        name
    ))
}

fn open_link(s: &mut cursive::Cursive, link: ResolvedLink) -> anyhow::Result<()> {
    match link {
        ResolvedLink::Doc(ty, name) => {
            let doc = find_doc(&context(s).sources, ty, &name)?;
            let mut renderer = context(s).create_renderer(&doc);
            renderer.render_doc(&doc).unwrap();
            let view = renderer.into_view();
            s.add_fullscreen_layer(view);
            Ok(())
        }
        ResolvedLink::External(link) => webbrowser::open(&link)
            .map(|_| {})
            .context("Failed to open web browser"),
    }
}

enum ResolvedLink {
    Doc(Option<doc::ItemType>, doc::Fqn),
    External(String),
}

fn resolve_link(
    doc_name: &doc::Fqn,
    doc_ty: doc::ItemType,
    link: String,
) -> anyhow::Result<ResolvedLink> {
    // TODO: support docs.rs and doc.rust-lang.org links
    match url::Url::parse(&link) {
        Ok(_) => Ok(ResolvedLink::External(link)),
        Err(url::ParseError::RelativeUrlWithoutBase) => resolve_doc_link(doc_name, doc_ty, &link)
            .with_context(|| format!("Could not parse relative link URL: {}", &link)),
        Err(e) => {
            Err(anyhow::Error::new(e).context(format!("Could not parse link URL: {}", &link)))
        }
    }
}

fn resolve_doc_link(
    doc_name: &doc::Fqn,
    doc_ty: doc::ItemType,
    link: &str,
) -> anyhow::Result<ResolvedLink> {
    // TODO: use a proper URL parser instead of manually parsing the URL
    let (link, fragment) = {
        let parts: Vec<_> = link.splitn(2, '#').collect();
        if parts.len() > 1 {
            (parts[0], Some(parts[1]))
        } else {
            (parts[0], None)
        }
    };
    let parts: Vec<_> = link
        .split('/')
        .filter(|s| !s.is_empty())
        .filter(|s| *s != ".")
        .collect();

    let (mut ty, mut name) = if doc_ty != doc::ItemType::Module && !parts.is_empty() {
        (None, doc_name.parent())
    } else {
        (Some(doc_ty), Some(doc_name.to_owned()))
    };

    for part in parts {
        // We support "..", "index.html", "<module>" and "<type>.<name>.html".
        match part {
            ".." => {
                ty = None;
                name = name.context("Exceeded root level")?.parent();
            }
            "index.html" => {}
            _ => {
                if let Some((part_ty, part_name)) = parse_url_part(part, Some(".html")) {
                    // part == "type.name.html"
                    ty = Some(part_ty.parse()?);
                    name = if let Some(name) = name {
                        Some(name.child(&part_name))
                    } else {
                        Some(part_name.to_owned().into())
                    };
                } else {
                    // part == "<module>"
                    ty = Some(doc::ItemType::Module);
                    name = if let Some(name) = name {
                        Some(name.child(&part))
                    } else {
                        Some(part.to_owned().into())
                    };
                }
            }
        }
    }

    if let Some(fragment) = fragment {
        // If the fragment is "<type>.:name>", we add it to the name, otherwise we ignore it
        // because it just points to some other element on the page.
        if let Some((fragment_ty, fragment_name)) = parse_url_part(fragment, None) {
            ty = Some(fragment_ty.parse()?);
            name = if let Some(name) = name {
                Some(name.child(&fragment_name))
            } else {
                Some(fragment_name.to_owned().into())
            };
        }
    }

    Ok(ResolvedLink::Doc(
        ty,
        name.context("Cannot handle link to root")?,
    ))
}

fn parse_url_part<'s>(s: &'s str, suffix: Option<&str>) -> Option<(&'s str, &'s str)> {
    let s = if let Some(suffix) = suffix {
        if s.ends_with(suffix) {
            &s[..s.len() - suffix.len()]
        } else {
            return None;
        }
    } else {
        s
    };
    let parts: Vec<_> = s.split('.').collect();
    if parts.len() == 2 {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}
