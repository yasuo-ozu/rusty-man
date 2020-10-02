// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod text;
mod utils;

use std::fmt;

use crate::args;
use crate::doc;

pub trait Viewer: fmt::Debug {
    fn open(&self, args: args::ViewerArgs, doc: &doc::Doc) -> anyhow::Result<()>;

    fn open_examples(
        &self,
        args: args::ViewerArgs,
        doc: &doc::Doc,
        examples: Vec<doc::Example>,
    ) -> anyhow::Result<()>;
}

pub fn get_viewer(s: &str) -> anyhow::Result<Box<dyn Viewer>> {
    let viewer: Box<dyn Viewer> = match s.to_lowercase().as_ref() {
        "plain" => Box::new(text::TextViewer::new(text::TextMode::Plain)),
        "rich" => Box::new(text::TextViewer::new(text::TextMode::Rich)),
        _ => anyhow::bail!("The viewer {} is not supported", s),
    };
    Ok(viewer)
}

pub fn get_default() -> Box<dyn Viewer> {
    let text_mode = if atty::is(atty::Stream::Stdout) {
        text::TextMode::Rich
    } else {
        text::TextMode::Plain
    };
    Box::new(text::TextViewer::new(text_mode))
}
