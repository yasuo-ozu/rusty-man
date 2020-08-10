// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod text;

use std::cmp;
use std::fmt;
use std::io;

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
        "plain" => Box::new(text::TextViewer::with_plain_text()),
        "rich" => Box::new(text::TextViewer::with_rich_text()),
        _ => anyhow::bail!("The viewer {} is not supported", s),
    };
    Ok(viewer)
}

pub fn get_default() -> Box<dyn Viewer> {
    use crossterm::tty::IsTty;

    if io::stdout().is_tty() {
        Box::new(text::TextViewer::with_rich_text())
    } else {
        Box::new(text::TextViewer::with_plain_text())
    }
}

pub fn get_line_length() -> usize {
    if let Ok((cols, _)) = crossterm::terminal::size() {
        cmp::min(cols.into(), 100)
    } else {
        100
    }
}
