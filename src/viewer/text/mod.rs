// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod plain;
mod rich;

use std::fmt;
use std::io;

use crate::doc;
use crate::viewer;

pub trait Printer: fmt::Debug {
    fn print_heading(&self, level: usize, s: &str) -> io::Result<()>;

    fn print_html(&self, s: &str) -> io::Result<()>;

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
        self.printer
            .print_heading(1, &format!("{} {}", doc.ty.name(), doc.name.as_ref()))?;
        self.print_opt(doc.definition.as_deref())?;
        self.print_opt(doc.description.as_deref())?;
        for (ty, groups) in &doc.groups {
            self.printer.println()?;
            self.printer.print_heading(2, ty.group_name())?;

            for group in groups {
                if let Some(title) = &group.title {
                    self.printer.println()?;
                    self.printer.print_heading(3, title)?;
                }

                if doc.ty == doc::ItemType::Module {
                    self.print_list(group.members.iter().map(|i| {
                        if let Some(description) = &i.description {
                            format!("{}<br/>{}", i.name.last(), description)
                        } else {
                            i.name.last().to_owned()
                        }
                    }))?;
                } else {
                    for member in &group.members {
                        self.printer.println()?;
                        self.printer.print_heading(4, member.name.last())?;
                        if let Some(definition) = &member.definition {
                            self.printer.print_html(definition)?;
                        }
                        if member.definition.is_some() && member.description.is_some() {
                            self.printer.println()?;
                        }
                        if let Some(description) = &member.description {
                            self.printer.print_html(description)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn print_opt(&self, s: Option<&str>) -> io::Result<()> {
        if let Some(s) = s {
            self.printer.println()?;
            self.printer.print_html(s)
        } else {
            Ok(())
        }
    }

    fn print_list<I, D>(&self, items: I) -> io::Result<()>
    where
        I: Iterator<Item = D>,
        D: fmt::Display,
    {
        let html = items
            .map(|i| format!("<li>{}</li>", i))
            .collect::<Vec<_>>()
            .join("");
        self.printer.print_html(&format!("<ul>{}</ul>", html))
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
