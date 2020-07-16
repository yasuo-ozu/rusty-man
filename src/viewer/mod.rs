// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod rich;
mod text;

use std::fmt;
use std::io;

use crate::doc;

pub trait Viewer: fmt::Debug {
    fn open(&self, doc: &doc::Doc) -> anyhow::Result<()>;
}

pub fn get_viewer(s: &str) -> anyhow::Result<Box<dyn Viewer>> {
    match s.to_lowercase().as_ref() {
        "rich" => Ok(Box::new(rich::RichViewer::new())),
        "text" => Ok(Box::new(text::TextViewer::new())),
        _ => Err(anyhow::anyhow!("The viewer {} is not supported", s)),
    }
}

pub fn get_default() -> Box<dyn Viewer> {
    if termion::is_tty(&io::stdout()) {
        Box::new(rich::RichViewer::new())
    } else {
        Box::new(text::TextViewer::new())
    }
}
