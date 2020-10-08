// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod text;
mod tui;
mod utils;

use std::fmt;
use std::io;

use crate::args;
use crate::doc;
use crate::source;

pub trait Viewer: fmt::Debug {
    fn open(
        &self,
        sources: Vec<Box<dyn source::Source>>,
        args: args::ViewerArgs,
        doc: &doc::Doc,
    ) -> anyhow::Result<()>;

    fn open_examples(
        &self,
        sources: Vec<Box<dyn source::Source>>,
        args: args::ViewerArgs,
        doc: &doc::Doc,
        examples: Vec<doc::Example>,
    ) -> anyhow::Result<()>;
}

pub fn get_viewer(s: &str) -> anyhow::Result<Box<dyn Viewer>> {
    let viewer: Box<dyn Viewer> = match s.to_lowercase().as_ref() {
        "plain" => Box::new(text::TextViewer::new(text::TextMode::Plain)),
        "rich" => Box::new(text::TextViewer::new(text::TextMode::Rich)),
        "tui" => Box::new(tui::TuiViewer::new()),
        _ => anyhow::bail!("The viewer {} is not supported", s),
    };
    Ok(viewer)
}

pub fn get_default() -> Box<dyn Viewer> {
    let text_mode = if termion::is_tty(&io::stdout()) {
        text::TextMode::Rich
    } else {
        text::TextMode::Plain
    };
    Box::new(text::TextViewer::new(text_mode))
}
