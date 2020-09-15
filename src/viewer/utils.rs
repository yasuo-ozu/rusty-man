// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::cmp;

use anyhow::Context as _;

use crate::args;
use crate::doc;

/// A trait for viewer implementations that display the documentation in a man-like style.
pub trait ManRenderer {
    type Error: std::error::Error + Sized + Send;

    fn print_title(&mut self, left: &str, center: &str, right: &str) -> Result<(), Self::Error>;
    fn print_heading(&mut self, indent: u8, text: &str) -> Result<(), Self::Error>;
    fn print_code(&mut self, indent: u8, code: &doc::Code) -> Result<(), Self::Error>;
    fn print_text(&mut self, indent: u8, text: &doc::Text) -> Result<(), Self::Error>;
    fn println(&mut self) -> Result<(), Self::Error>;

    fn render_doc(&mut self, doc: &doc::Doc) -> Result<(), Self::Error> {
        print_title(self, doc)?;

        if let Some(text) = &doc.definition {
            print_heading(self, 1, "Synopsis")?;
            self.print_code(6, text)?;
            self.println()?;
        }

        if let Some(text) = &doc.description {
            print_heading(self, 1, "Description")?;
            self.print_text(6, text)?;
            self.println()?;
        }

        for (ty, groups) in &doc.groups {
            print_heading(self, 1, ty.group_name())?;

            for group in groups {
                if let Some(title) = &group.title {
                    print_heading(self, 2, title)?;
                }

                for member in &group.members {
                    // TODO: use something link strip_prefix instead of last()
                    print_heading(self, 3, member.name.last())?;
                    if let Some(definition) = &member.definition {
                        self.print_code(12, definition)?;
                    }
                    if member.definition.is_some() && member.description.is_some() {
                        self.println()?;
                    }
                    if let Some(description) = &member.description {
                        self.print_text(12, description)?;
                    }
                    if member.definition.is_some() || member.description.is_some() {
                        self.println()?;
                    }
                }
            }
        }

        Ok(())
    }

    fn render_examples(
        &mut self,
        doc: &doc::Doc,
        examples: &[doc::Example],
    ) -> Result<(), Self::Error> {
        print_title(self, doc)?;
        print_heading(self, 1, "Examples")?;

        let n = examples.len();
        for (i, example) in examples.iter().enumerate() {
            if n > 1 {
                print_heading(self, 2, &format!("Example {} of {}", i + 1, n))?;
            }
            if let Some(description) = &example.description {
                self.print_text(6, description)?;
                self.println()?;
            }
            self.print_code(6, &example.code)?;
            self.println()?;
        }

        Ok(())
    }
}

fn print_title<M: ManRenderer + ?Sized>(viewer: &mut M, doc: &doc::Doc) -> Result<(), M::Error> {
    let title = format!("{} {}", doc.ty.name(), doc.name);
    viewer.print_title(doc.name.krate(), &title, "rusty-man")
}

fn print_heading<M: ManRenderer + ?Sized>(
    viewer: &mut M,
    level: u8,
    text: &str,
) -> Result<(), M::Error> {
    let text = match level {
        1 => std::borrow::Cow::from(text.to_uppercase()),
        _ => std::borrow::Cow::from(text),
    };
    let indent = match level {
        1 => 0,
        2 => 3,
        _ => 6,
    };
    viewer.print_heading(indent, text.as_ref())
}

pub fn get_line_length(args: &args::ViewerArgs) -> usize {
    if let Some(width) = args.width {
        width
    } else if let Ok((cols, _)) = crossterm::terminal::size() {
        cmp::min(cols.into(), args.max_width)
    } else {
        args.max_width
    }
}

pub fn get_syntect_theme(args: &args::ViewerArgs) -> anyhow::Result<syntect::highlighting::Theme> {
    let mut theme_set = syntect::highlighting::ThemeSet::load_defaults();
    let theme_name = args.theme.as_deref().unwrap_or("base16-eighties.dark");
    theme_set
        .themes
        .remove(theme_name)
        .with_context(|| format!("Could not find theme {}", theme_name))
}
