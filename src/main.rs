// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod doc;
mod parser;
mod source;
mod viewer;

use std::path;

use structopt::StructOpt;

/// Command-line interface for rustdoc documentation
#[derive(Debug, StructOpt)]
struct Opt {
    /// The keyword to open the documentation for, e. g. `rand_core::RngCore`
    keyword: String,

    /// The sources to check for documentation generated by rustdoc
    ///
    /// Typically, this is the path of a directory containing the documentation for one or more
    /// crates in subdirectories.
    #[structopt(name = "source", short, long, number_of_values = 1)]
    source_paths: Vec<String>,

    /// The viewer for the rustdoc documentation
    #[structopt(long, parse(try_from_str = viewer::get_viewer))]
    viewer: Option<Box<dyn viewer::Viewer>>,

    /// Do not search the default documentation sources
    ///
    /// If this option is not set, rusty-man appends `/usr/share/doc/rust{,-doc}/html` and
    /// `target/doc` directory to the list of sources if they exist.
    #[structopt(long)]
    no_default_sources: bool,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let sources = load_sources(&opt.source_paths, !opt.no_default_sources)?;
    let doc = find_doc(&sources, &opt.keyword)?;
    let viewer = opt.viewer.unwrap_or_else(viewer::get_default);
    viewer.open(&doc)
}

const DEFAULT_SOURCES: &[&str] = &[
    "/usr/share/doc/rust/html",
    "/usr/share/doc/rust-doc/html",
    "./target/doc",
];

fn load_sources(
    sources: &[String],
    load_default_sources: bool,
) -> anyhow::Result<Vec<Box<dyn source::Source>>> {
    let mut vec: Vec<Box<dyn source::Source>> = Vec::new();

    if load_default_sources {
        for s in DEFAULT_SOURCES {
            let path: &path::Path = s.as_ref();
            if path.is_dir() {
                vec.push(source::get_source(path)?);
            }
        }
    }

    for s in sources {
        vec.push(source::get_source(s)?);
    }

    // The last source should be searched first --> reverse source vector
    vec.reverse();

    Ok(vec)
}

fn find_doc(sources: &[Box<dyn source::Source>], keyword: &str) -> anyhow::Result<doc::Doc> {
    use anyhow::Context;

    let parts: Vec<&str> = keyword.split("::").collect();
    let crate_ = find_crate(sources, parts[0])?;
    let item = crate_
        .find_item(&parts[1..])?
        .with_context(|| format!("Could not find the item {}", keyword))?;
    item.load_doc()
}

fn find_crate(sources: &[Box<dyn source::Source>], name: &str) -> anyhow::Result<doc::Crate> {
    use anyhow::Context;

    sources
        .iter()
        .filter_map(|s| s.find_crate(name))
        .next()
        .with_context(|| format!("Could not find the crate {}", name))
}

#[cfg(test)]
mod tests {
    use std::path;

    pub fn ensure_docs() -> path::PathBuf {
        let doc = path::PathBuf::from("./target/doc");
        assert!(
            doc.is_dir(),
            "You have to run `cargo doc` before running this test case."
        );
        doc
    }
}
