// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

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

    fn print(&self, s: &str) {
        let lines = html2text::from_read_rich(s.as_bytes(), self.line_length);
        for line in lines {
            for element in line.iter() {
                if let text_renderer::TaggedLineElement::Str(ts) = element {
                    self.render_string(ts);
                }
            }
            println!();
        }
    }

    fn print_opt(&self, s: Option<&str>) {
        if let Some(s) = s {
            println!();
            self.print(s);
        }
    }

    fn print_heading(&self, s: &str, level: usize) {
        print!("{}{} ", termion::style::Bold, "#".repeat(level));
        self.print(s);
        print!("{}", termion::style::Reset);
    }

    fn render_string(&self, ts: &RichString) {
        let start_style = get_style(ts, get_start_style);
        let end_style = get_style(ts, get_end_style);
        print!("{}{}{}", start_style, ts.s, end_style);
    }
}

impl viewer::Viewer for RichViewer {
    fn open(&self, doc: &doc::Doc) -> anyhow::Result<()> {
        viewer::spawn_pager();

        self.print_heading(&doc.title, 1);
        self.print_opt(doc.definition.as_deref());
        self.print_opt(doc.description.as_deref());
        Ok(())
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
