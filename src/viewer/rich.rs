// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use html2text::render::text_renderer;

use crate::doc;
use crate::viewer;

type RichString = text_renderer::TaggedString<Vec<text_renderer::RichAnnotation>>;

#[derive(Clone, Debug)]
pub struct RichViewer {
    line_length: usize,
}

impl RichViewer {
    pub fn new() -> Self {
        Self {
            line_length: viewer::get_line_length(),
        }
    }

    fn print_doc(&self, doc: &doc::Doc) -> io::Result<()> {
        self.print_heading(&doc.title, 1)?;
        self.print_opt(doc.definition.as_deref())?;
        self.print_opt(doc.description.as_deref())?;
        Ok(())
    }

    fn print(&self, s: &str) -> io::Result<()> {
        let lines = html2text::from_read_rich(s.as_bytes(), self.line_length);
        for line in lines {
            for element in line.iter() {
                if let text_renderer::TaggedLineElement::Str(ts) = element {
                    self.render_string(ts)?;
                }
            }
            writeln!(io::stdout())?;
        }
        Ok(())
    }

    fn print_opt(&self, s: Option<&str>) -> io::Result<()> {
        if let Some(s) = s {
            writeln!(io::stdout())?;
            self.print(s)
        } else {
            Ok(())
        }
    }

    fn print_heading(&self, s: &str, level: usize) -> io::Result<()> {
        let prefix = "#".repeat(level);
        write!(io::stdout(), "{}{} ", termion::style::Bold, prefix)?;
        self.print(s)?;
        write!(io::stdout(), "{}", termion::style::Reset)
    }

    fn render_string(&self, ts: &RichString) -> io::Result<()> {
        let start_style = get_style(ts, get_start_style);
        let end_style = get_style(ts, get_end_style);
        write!(io::stdout(), "{}{}{}", start_style, ts.s, end_style)
    }
}

impl viewer::Viewer for RichViewer {
    fn open(&self, doc: &doc::Doc) -> anyhow::Result<()> {
        viewer::spawn_pager();

        self.print_doc(doc)
            .or_else(ignore_pipe_error)
            .map_err(Into::into)
    }
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

fn get_style<F>(ts: &RichString, f: F) -> String
where
    F: Fn(&text_renderer::RichAnnotation) -> String,
{
    ts.tag.iter().map(f).collect::<Vec<_>>().join("")
}

fn get_start_style(annotation: &text_renderer::RichAnnotation) -> String {
    use termion::{color, style};
    use text_renderer::RichAnnotation;

    match annotation {
        RichAnnotation::Default => String::new(),
        RichAnnotation::Link(_) => style::Underline.to_string(),
        RichAnnotation::Image => String::new(),
        RichAnnotation::Emphasis => style::Italic.to_string(),
        RichAnnotation::Strong => style::Bold.to_string(),
        RichAnnotation::Code => color::Fg(color::LightYellow).to_string(),
        RichAnnotation::Preformat(_) => String::new(),
    }
}

fn get_end_style(annotation: &text_renderer::RichAnnotation) -> String {
    use termion::{color, style};
    use text_renderer::RichAnnotation;

    match annotation {
        RichAnnotation::Default => String::new(),
        RichAnnotation::Link(_) => style::NoUnderline.to_string(),
        RichAnnotation::Image => String::new(),
        RichAnnotation::Emphasis => style::NoItalic.to_string(),
        // TODO: investigate why NoBold does not work
        RichAnnotation::Strong => style::Reset.to_string(),
        RichAnnotation::Code => color::Fg(color::Reset).to_string(),
        RichAnnotation::Preformat(_) => String::new(),
    }
}
