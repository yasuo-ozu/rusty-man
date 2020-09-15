// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::cmp;

use anyhow::Context as _;

use crate::args;

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
