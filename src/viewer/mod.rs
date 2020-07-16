// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod text;

use std::fmt;

use crate::doc;

pub trait Viewer: fmt::Debug {
    fn open(&self, doc: &doc::Doc) -> anyhow::Result<()>;
}

pub fn get_viewer(s: &str) -> anyhow::Result<Box<dyn Viewer>> {
    match s.to_lowercase().as_ref() {
        "text" => Ok(Box::new(text::TextViewer::new())),
        _ => Err(anyhow::anyhow!("The viewer {} is not supported", s)),
    }
}
