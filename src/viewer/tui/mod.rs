// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod views;

use std::convert;

use anyhow::Context as _;
use cursive::view::{Resizable as _, Scrollable as _};
use cursive::views::{Dialog, EditView, LinearLayout, PaddedView, Panel, SelectView, TextView};
use cursive::{event, theme, utils::markup};
use cursive_markup::MarkupView;

use crate::args;
use crate::doc;
use crate::index;
use crate::source;
use crate::viewer::{self, utils, utils::ManRenderer as _};

use views::{CodeView, HtmlRenderer, LinkView};

#[derive(Clone, Debug)]
pub struct TuiViewer {}

impl TuiViewer {
    pub fn new() -> TuiViewer {
        TuiViewer {}
    }

    fn render<F>(
        &self,
        sources: source::Sources,
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
        sources: source::Sources,
        args: args::ViewerArgs,
        doc: &doc::Doc,
    ) -> anyhow::Result<()> {
        self.render(sources, args, doc, |renderer| renderer.render_doc(doc))
    }

    fn open_examples(
        &self,
        sources: source::Sources,
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
    pub sources: source::Sources,
    pub args: args::ViewerArgs,
    pub highlighter: Option<utils::Highlighter>,
}

impl Context {
    pub fn new(sources: source::Sources, args: args::ViewerArgs) -> anyhow::Result<Context> {
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

    fn print_heading(
        &mut self,
        indent: u8,
        text: &str,
        link: Option<utils::DocLink>,
    ) -> Result<(), Self::Error> {
        let text = markup::StyledString::styled(text, theme::Effect::Bold);
        if let Some(link) = link {
            let heading = LinkView::new(text, move |s| {
                if let Err(err) = open_link(s, link.clone().into()) {
                    report_error(s, err);
                }
            });
            self.layout.add_child(indent_view(indent, heading));
        } else {
            let heading = TextView::new(text);
            self.layout.add_child(indent_view(indent, heading));
        }
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
        let renderer = HtmlRenderer::new(&text.html, self.highlighter.cloned());
        let mut view = MarkupView::with_renderer(renderer);
        view.set_maximum_width(self.max_width.saturating_sub(indent));
        let doc_name = self.doc_name.clone();
        let doc_ty = self.doc_ty;
        view.on_link_select(move |s, link| handle_link(s, &doc_name, doc_ty, link));
        self.layout.add_child(indent_view(indent, view));
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

fn create_backend() -> anyhow::Result<Box<dyn cursive::backend::Backend>> {
    let termion =
        cursive::backends::termion::Backend::init().context("Could not create termion backend")?;
    let buffered = cursive_buffered_backend::BufferedBackend::new(termion);
    Ok(Box::new(buffered))
}

fn create_cursive(
    sources: source::Sources,
    args: args::ViewerArgs,
) -> anyhow::Result<cursive::Cursive> {
    use cursive::event::{Event, Key};

    let mut cursive =
        cursive::Cursive::try_new(create_backend).context("Could not create Cursive instance")?;

    cursive.set_user_data(Context::new(sources, args)?);

    // vim-like keybindings
    cursive.add_global_callback('j', |s| s.on_event(Key::Down.into()));
    cursive.add_global_callback('k', |s| s.on_event(Key::Up.into()));
    cursive.add_global_callback('h', |s| s.on_event(Key::Left.into()));
    cursive.add_global_callback('l', |s| s.on_event(Key::Right.into()));
    cursive.add_global_callback('G', |s| s.on_event(Key::End.into()));
    cursive.add_global_callback('g', |s| s.on_event(Key::Home.into()));
    cursive.add_global_callback(Event::CtrlChar('f'), |s| s.on_event(Key::PageDown.into()));
    cursive.add_global_callback(Event::CtrlChar('b'), |s| s.on_event(Key::PageUp.into()));

    cursive.add_global_callback('q', |s| s.quit());
    cursive.add_global_callback(event::Key::Backspace, |s| {
        let screen = s.screen_mut();
        if screen.len() > 1 {
            screen.pop_layer();
        }
    });
    cursive.add_global_callback('o', open_doc_dialog);

    let mut theme = theme::Theme {
        shadow:  false,
        ..Default::default()
    };
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

fn with_report_error<F>(s: &mut cursive::Cursive, f: F)
where
    F: Fn(&mut cursive::Cursive) -> anyhow::Result<()>,
{
    if let Err(err) = f(s) {
        report_error(s, err);
    }
}

fn open_doc_dialog(s: &mut cursive::Cursive) {
    let mut edit_view = EditView::new();
    edit_view.set_on_submit(|s, val| {
        with_report_error(s, |s| {
            s.pop_layer();
            let sources = &context(s).sources;
            let name = doc::Name::from(val.to_owned());
            let mut doc = sources.find(&name, None)?;
            if doc.is_none() {
                let items = sources.search(&name)?;
                if items.len() > 1 {
                    select_doc_dialog(s, items);
                    return Ok(());
                } else if !items.is_empty() {
                    doc = sources.find(&items[0].name, Some(items[0].ty))?;
                }
            }
            if let Some(doc) = doc {
                open_doc(s, &doc);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Could not find documentation for {}", name))
            }
        });
    });
    let dialog = Dialog::around(edit_view.min_width(40)).title("Open documentation");
    s.add_layer(dialog);
}

fn select_doc_dialog(s: &mut cursive::Cursive, items: Vec<index::IndexItem>) {
    let mut select_view = SelectView::new();
    select_view.add_all(
        items
            .into_iter()
            .map(|item| (item.name.as_ref().to_owned(), item)),
    );
    select_view.set_on_submit(|s, item| {
        with_report_error(s, |s| {
            let doc = context(s).sources.find(&item.name, Some(item.ty))?;
            if let Some(doc) = doc {
                open_doc(s, &doc);
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Could not find documentation for {}",
                    item.name
                ))
            }
        });
    });
    let dialog = Dialog::around(select_view.scrollable()).title("Select documentation item");
    s.add_layer(dialog);
}

fn open_doc(s: &mut cursive::Cursive, doc: &doc::Doc) {
    let mut renderer = context(s).create_renderer(&doc);
    renderer.render_doc(&doc).unwrap();
    let view = renderer.into_view();
    s.add_fullscreen_layer(view);
}

fn handle_link(s: &mut cursive::Cursive, doc_name: &doc::Fqn, doc_ty: doc::ItemType, link: &str) {
    let result = resolve_link(doc_name, doc_ty, link).and_then(|link| open_link(s, link));
    if let Err(err) = result {
        report_error(s, err);
    }
}

fn open_link(s: &mut cursive::Cursive, link: ResolvedLink) -> anyhow::Result<()> {
    match link {
        ResolvedLink::Doc(ty, name) => {
            let doc = context(s)
                .sources
                .find(&name, ty)?
                .with_context(|| format!("Could not find documentation for item: {}", name))?;
            open_doc(s, &doc);
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

impl From<utils::DocLink> for ResolvedLink {
    fn from(link: utils::DocLink) -> ResolvedLink {
        ResolvedLink::Doc(link.ty, link.name)
    }
}

fn resolve_link(
    doc_name: &doc::Fqn,
    doc_ty: doc::ItemType,
    link: &str,
) -> anyhow::Result<ResolvedLink> {
    // TODO: support docs.rs and doc.rust-lang.org links
    match url::Url::parse(link) {
        Ok(_) => Ok(ResolvedLink::External(link.to_owned())),
        Err(url::ParseError::RelativeUrlWithoutBase) => resolve_doc_link(doc_name, doc_ty, link)
            .with_context(|| format!("Could not parse relative link URL: {}", link)),
        Err(e) => Err(anyhow::Error::new(e).context(format!("Could not parse link URL: {}", link))),
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
