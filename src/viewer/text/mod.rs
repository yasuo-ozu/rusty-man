// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod plain;
mod rich;

use std::fmt;
use std::io;

use crate::doc;
use crate::viewer;

pub trait Printer: fmt::Debug {
    fn print_title(&self, left: &str, middle: &str, right: &str) -> io::Result<()>;

    fn print_heading(&self, indent: usize, level: usize, s: &str) -> io::Result<()>;

    fn print_html(&self, indent: usize, s: &doc::Text, show_links: bool) -> io::Result<()>;

    fn print_code(&self, indent: usize, code: &doc::Text) -> io::Result<()>;

    fn println(&self) -> io::Result<()>;
}

#[derive(Clone, Debug)]
pub struct TextViewer<P: Printer> {
    printer: P,
}

impl<P: Printer> TextViewer<P> {
    pub fn new(printer: P) -> Self {
        Self { printer }
    }

    fn print_doc(&self, doc: &doc::Doc) -> io::Result<()> {
        self.print_title(doc)?;
        self.print_opt("SYNOPSIS", doc.definition.as_ref(), false)?;
        self.print_opt("DESCRIPTION", doc.description.as_ref(), true)?;
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
                        self.printer.print_html(12, definition, false)?;
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

    fn print_opt(&self, title: &str, s: Option<&doc::Text>, show_links: bool) -> io::Result<()> {
        if let Some(s) = s {
            self.print_heading(1, title)?;
            self.printer.print_html(6, s, show_links)?;
            self.printer.println()
        } else {
            Ok(())
        }
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

impl TextViewer<plain::PlainTextRenderer> {
    pub fn with_plain_text() -> Self {
        Self::new(plain::PlainTextRenderer::new())
    }
}

impl TextViewer<rich::RichTextRenderer> {
    pub fn with_rich_text() -> Self {
        Self::new(rich::RichTextRenderer::new())
    }
}

impl<P: Printer> viewer::Viewer for TextViewer<P> {
    fn open(&self, doc: &doc::Doc) -> anyhow::Result<()> {
        spawn_pager();

        self.print_doc(doc)
            .or_else(ignore_pipe_error)
            .map_err(Into::into)
    }

    fn open_examples(&self, doc: &doc::Doc, examples: Vec<doc::Example>) -> anyhow::Result<()> {
        spawn_pager();

        self.print_examples(doc, examples)
            .or_else(ignore_pipe_error)
            .map_err(Into::into)
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
