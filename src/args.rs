// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::fs;
use std::path;

use merge::Merge;
use serde::Deserialize;
use structopt::StructOpt;

use crate::doc;
use crate::viewer;

/// Command-line viewer for rustdoc documentation
///
/// rusty-man reads the HTML documentation generated by rustdoc and displays a documentation item.
/// Make sure to run `cargo doc` before using rusty-man.  Per default, rusty-man looks up
/// documentation in the ./target/doc directory relative to the current working directory and in
/// the system documentation directories (share/doc/rust{,-doc}/html relative to the Rust sysroot,
/// see `rustc --print sysroot`, or /usr).  Use the -s/--source option if you want to read the
/// documentation from a different directory.
///
/// rusty-man tries to find an item that exactly matches the given keyword.  If it doesn’t find an
/// exact match, it reads the search indexes of all available sources and tries to find a partial
/// match.
#[derive(Debug, Default, Deserialize, Merge, StructOpt)]
#[serde(default)]
pub struct Args {
    /// The keyword to open the documentation for, e. g. `rand_core::RngCore`
    #[merge(skip)]
    #[serde(skip)]
    pub keyword: doc::Name,

    /// The sources to check for documentation generated by rustdoc
    ///
    /// Typically, this is the path of a directory containing the documentation for one or more
    /// crates in subdirectories.
    #[merge(strategy = merge::vec::prepend)]
    #[structopt(name = "source", short, long, number_of_values = 1)]
    pub source_paths: Vec<String>,

    /// The viewer for the rustdoc documentation (one of: plain, rich, tui)
    #[structopt(long, parse(try_from_str = viewer::get_viewer))]
    #[serde(deserialize_with = "deserialize_viewer")]
    pub viewer: Option<Box<dyn viewer::Viewer>>,

    /// Do not search the default documentation sources
    ///
    /// If this option is not set, rusty-man appends `$sysroot/share/doc/rust{,-doc}/html` and
    /// `$target/doc` to the list of sources if they exist.  `$sysroot` is the output of `rustc
    /// --print sysroot` or `/usr` if that command does not output a valid path.  `$target` is
    /// `$CARGO_TARGET_DIR`, `$CARGO_BUILD_TARGET_DIR` or `./target`.
    #[merge(strategy = merge::bool::overwrite_false)]
    #[structopt(long)]
    pub no_default_sources: bool,

    /// Open found page in web browser.
    #[merge(strategy = merge::bool::overwrite_false)]
    #[structopt(long)]
    pub open: bool,

    /// Do not read the search index if there is no exact match
    ///
    /// Per default, rusty-man reads the search indexes of all sources and tries to find matching
    /// items if there is no exact match for the keyword.  If this option is set, the search
    /// indexes are not read.
    #[merge(strategy = merge::bool::overwrite_false)]
    #[structopt(long)]
    pub no_search: bool,

    /// Show all examples for the item instead of opening the full documentation.
    #[merge(strategy = merge::bool::overwrite_false)]
    #[structopt(short, long)]
    pub examples: bool,

    /// The path to the configuration file to read
    ///
    /// Per default, rusty-man tries to read defaults for the command-line arguments from the
    /// config.toml file in the user configuration directory according to the XDG Base Directory
    /// Specification, i. e. ${XDG_USER_CONFIG}/rusty-man/config.toml, where ${XDG_USER_CONFIG}
    /// defaults to ${HOME}/.config.
    ///
    /// If this option is set, rusty-man reads the given configuration file instead.  If this
    /// option is set to "-", rusty-man does not read any configuration files.
    #[merge(skip)]
    #[structopt(short, long)]
    #[serde(skip)]
    pub config_file: Option<String>,

    #[structopt(flatten)]
    #[serde(flatten)]
    pub viewer_args: ViewerArgs,
}

#[derive(Debug, Default, Deserialize, Merge, StructOpt)]
#[serde(default)]
pub struct ViewerArgs {
    /// Disable syntax highlighting.
    ///
    /// Per default, rusty-man tries to highlight Rust code snippets in its output if the rich or
    /// tui viewer is selected.  If this option is set, it renders the HTML representation instead.
    #[merge(strategy = merge::bool::overwrite_false)]
    #[structopt(long)]
    pub no_syntax_highlight: bool,

    /// The color theme for syntax highlighting
    ///
    /// rusty-man includes these color themes: base16-ocean.dark, base16-eighties.dark,
    /// base16-mocha.dark, base16-ocean.light, InspiredGitHub, Solarized (dark), Solarized (light).
    /// Default value: base16-eighties.dark.
    #[structopt(long)]
    pub theme: Option<String>,

    /// The width of the text output
    ///
    /// Per default, rusty-man sets the width of the text output based on the width of the terminal
    /// with the maximum width given by --max-width.  If this option is set, it uses the given
    /// width instead.
    #[structopt(long)]
    pub width: Option<usize>,

    /// The maximum width of the text output
    ///
    /// Unless the --width option is set, rusty-man sets the width of the text output based on the
    /// width of the terminal with the maximum width set with this option.
    #[structopt(long)]
    pub max_width: Option<usize>,

    /// The pager to use for the plain and rich viewers.
    ///
    /// Per default, rusty-man uses the pager set in the PAGER environment variable, or less if
    /// this environment variable is not set.
    #[structopt(long)]
    pub pager: Option<String>,
}

impl Args {
    pub fn load() -> anyhow::Result<Args> {
        let mut args = Args::from_args();

        if let Some(config) = Args::load_config(args.config_file.as_deref())? {
            args.merge(config);
        }

        Ok(args)
    }

    fn load_config(file: Option<&str>) -> anyhow::Result<Option<Args>> {
        let path = if let Some(file) = file {
            if file == "-" {
                None
            } else {
                Some(path::PathBuf::from(file))
            }
        } else {
            let dirs = xdg::BaseDirectories::with_prefix("rusty-man")?;
            dirs.find_config_file("config.toml")
        };
        if let Some(path) = path {
            log::info!("Loading configuration file '{}'", path.display());
            let s = fs::read_to_string(path)?;
            toml::from_str(&s).map(Some).map_err(From::from)
        } else {
            Ok(None)
        }
    }
}

fn deserialize_viewer<'de, D>(d: D) -> Result<Option<Box<dyn viewer::Viewer>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s: Option<&str> = Deserialize::deserialize(d)?;
    s.map(|s| viewer::get_viewer(s).map_err(D::Error::custom))
        .transpose()
}
