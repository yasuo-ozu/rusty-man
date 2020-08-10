// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod plain;
mod rich;

use std::fmt;
use std::io;
use std::marker;

use crate::doc;
use crate::viewer;

pub trait Printer: fmt::Debug + marker::Sized {
    fn new(args: crate::ViewerArgs) -> anyhow::Result<Self>;

    fn print_title(&self, left: &str, middle: &str, right: &str) -> io::Result<()>;

    fn print_heading(&self, indent: usize, level: usize, s: &str) -> io::Result<()>;

    fn print_html(&self, indent: usize, s: &doc::Text, show_links: bool) -> io::Result<()>;

    fn print_code(&self, indent: usize, code: &doc::Text) -> io::Result<()>;

    fn println(&self) -> io::Result<()>;
}

#[derive(Clone, Debug)]
pub struct TextViewer<P: Printer> {
    _printer: marker::PhantomData<P>,
}

#[derive(Clone, Debug)]
pub struct TextRenderer<P: Printer> {
    printer: P,
}

impl<P: Printer> TextViewer<P> {
    fn new() -> Self {
        TextViewer {
            _printer: Default::default(),
        }
    }

    fn exec<F>(&self, args: crate::ViewerArgs, op: F) -> anyhow::Result<()>
    where
        F: FnOnce(&TextRenderer<P>) -> io::Result<()>,
    {
        spawn_pager();

        let printer = P::new(args)?;
        let renderer = TextRenderer::new(printer);
        op(&renderer).or_else(ignore_pipe_error).map_err(Into::into)
    }
}

impl TextViewer<plain::PlainTextRenderer> {
    pub fn with_plain_text() -> Self {
        TextViewer::new()
    }
}

impl TextViewer<rich::RichTextRenderer> {
    pub fn with_rich_text() -> Self {
        TextViewer::new()
    }
}

impl<P: Printer> viewer::Viewer for TextViewer<P> {
    fn open(&self, args: crate::ViewerArgs, doc: &doc::Doc) -> anyhow::Result<()> {
        self.exec(args, |r| r.print_doc(doc))
    }

    fn open_examples(
        &self,
        args: crate::ViewerArgs,
        doc: &doc::Doc,
        examples: Vec<doc::Example>,
    ) -> anyhow::Result<()> {
        self.exec(args, |r| r.print_examples(doc, examples))
    }
}

impl<P: Printer> TextRenderer<P> {
    pub fn new(printer: P) -> Self {
        Self { printer }
    }

    fn print_doc(&self, doc: &doc::Doc) -> io::Result<()> {
        self.print_title(doc)?;

        if let Some(text) = &doc.definition {
            self.print_heading(1, "Synopsis")?;
            self.printer.print_code(6, text)?;
            self.printer.println()?;
        }

        if let Some(text) = &doc.description {
            self.print_heading(1, "Description")?;
            self.printer.print_html(6, text, false)?;
            self.printer.println()?;
        }

        for (ty, groups) in &doc.groups {
            self.print_heading(1, ty.group_name())?;

            for group in groups {
                if let Some(title) = &group.title {
                    self.print_heading(2, title)?;
                }

                for member in &group.members {
                    // TODO: use something link strip_prefix instead of last()
                    self.print_heading(3, member.name.last())?;
                    if let Some(definition) = &member.definition {
                        self.printer.print_code(12, definition)?;
                    }
                    if member.definition.is_some() && member.description.is_some() {
                        self.printer.println()?;
                    }
                    if let Some(description) = &member.description {
                        self.printer.print_html(12, description, true)?;
                    }
                    if member.definition.is_some() || member.description.is_some() {
                        self.printer.println()?;
                    }
                }
            }
        }
        Ok(())
    }

    fn print_examples(&self, doc: &doc::Doc, examples: Vec<doc::Example>) -> io::Result<()> {
        self.print_title(doc)?;
        self.print_heading(1, "Examples")?;

        let n = examples.len();
        for (i, example) in examples.iter().enumerate() {
            if n > 1 {
                self.print_heading(2, &format!("Example {} of {}", i + 1, n))?;
            }
            if let Some(description) = &example.description {
                self.printer.print_html(6, description, true)?;
                self.printer.println()?;
            }
            self.printer.print_code(6, &example.code)?;
            self.printer.println()?;
        }

        Ok(())
    }

    fn print_title(&self, doc: &doc::Doc) -> io::Result<()> {
        let title = format!("{} {}", doc.ty.name(), doc.name.as_ref());
        self.printer
            .print_title(doc.name.krate(), &title, "rusty-man")
    }

    fn print_heading(&self, level: usize, s: &str) -> io::Result<()> {
        let text = match level {
            1 => std::borrow::Cow::from(s.to_uppercase()),
            _ => std::borrow::Cow::from(s),
        };
        let indent = match level {
            1 => 0,
            2 => 3,
            _ => 6,
        };
        self.printer.print_heading(indent, level, text.as_ref())
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
