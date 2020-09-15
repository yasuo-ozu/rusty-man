// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod plain;
mod rich;

use std::io;

use crate::args;
use crate::doc;
use crate::viewer::{self, utils};

#[derive(Clone, Debug)]
pub struct TextViewer {
    mode: TextMode,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TextMode {
    Plain,
    Rich,
}

impl TextViewer {
    pub fn new(mode: TextMode) -> Self {
        TextViewer { mode }
    }

    fn exec<F>(&self, args: args::ViewerArgs, op: F) -> anyhow::Result<()>
    where
        F: FnOnce(Box<dyn utils::ManRenderer<Error = io::Error>>) -> io::Result<()>,
    {
        let viewer: Box<dyn utils::ManRenderer<Error = io::Error>> = match self.mode {
            TextMode::Plain => Box::new(plain::PlainTextRenderer::new(args)),
            TextMode::Rich => Box::new(rich::RichTextRenderer::new(args)?),
        };

        spawn_pager();
        op(viewer).or_else(ignore_pipe_error).map_err(Into::into)
    }
}

impl viewer::Viewer for TextViewer {
    fn open(&self, args: args::ViewerArgs, doc: &doc::Doc) -> anyhow::Result<()> {
        self.exec(args, |mut viewer| viewer.render_doc(doc))
    }

    fn open_examples(
        &self,
        args: args::ViewerArgs,
        doc: &doc::Doc,
        examples: Vec<doc::Example>,
    ) -> anyhow::Result<()> {
        self.exec(args, |mut viewer| viewer.render_examples(doc, &examples))
    }
}

pub fn spawn_pager() {
    pager::Pager::with_default_pager("less -cr").setup()
}

fn ignore_pipe_error(error: io::Error) -> io::Result<()> {
    // If the pager is terminated before we can write everything to stdout, we will receive a
    // BrokenPipe error.  But we don’t want to report this error to the user.  See also:
    // https://github.com/rust-lang/rust/issues/46016
    if error.kind() == io::ErrorKind::BrokenPipe {
        Ok(())
    } else {
        Err(error)
    }
}

pub fn print_title(line_length: usize, left: &str, middle: &str, right: &str) -> io::Result<()> {
    use io::Write;

    write!(io::stdout(), "{}", left)?;

    let mut idx = left.len();
    let middle_idx = line_length / 2;
    let offset = middle.len() / 2;

    let spacing = if idx + offset >= middle_idx {
        1
    } else {
        middle_idx - offset - idx
    };
    write!(io::stdout(), "{}{}", " ".repeat(spacing), middle)?;
    idx += middle.len() + spacing;

    let end_idx = line_length;
    let offset = right.len();
    let spacing = if idx + offset >= end_idx {
        1
    } else {
        end_idx - offset - idx
    };
    writeln!(io::stdout(), "{}{}", " ".repeat(spacing), right)
}
