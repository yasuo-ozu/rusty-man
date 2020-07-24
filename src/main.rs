// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! rusty-man is a command-line viewer for documentation generated by `rustdoc`.
//!
//! rusty-man opens the documentation for a given keyword.  It performs these steps to find the
//! documentation for an item:
//! 1. The sources, currently only local directories, are loaded, see the `load_sources` funnction
//!    and the `source` module.  Per default, we look for documentation in `/usr/share/doc` and in
//!    `./target/doc`.
//! 2. We split the keyword `{crate}::{item}` into the crate and the item and try to find the crate
//!    in one of the sources – see the `find_crate` function.
//! 3. If we found a crate, we look up the item in the `all.html` file of the crate and load the
//!    documentation linked there.  If we can’t find the item in the index, we check whether it is
//!    a module by trying to open the `{item}/index.html` file.  If this fails too, we check
//!    whether the item `{parent}::{member}` is a member of another type.  See the `find_doc`
//!    function and the `doc` module.
//! 4. If we didn’t find a match in the previous step, we load the search index from the
//!    `search-index.js` file for all sources and try to find a matching item.  If we find one, we
//!    open the documentation for that item as in step 3.  See the `search_doc` function and the
//!    `index` module.
//!
//! If we found a documentation item, we use a viewer to open it – see the `viewer` module.
//! Currently, there are two viewer implementations:  `PlainTextViewer` converts the documentaion
//! to plain text, `RichTextViewer` adds some formatting to it.  Both viewers pipe their output
//! through a pager, if available.
//!
//! The documentation is scraped from the HTML files generated by `rustdoc`.  See the `parser`
//! module for the scraping and the `doc::Doc` struct for the structure of the documentation items.
//! For details on the structure of the HTML files and the search index, you have to look at the
//! `html::render` module in the `librustdoc` source code.
//!
//! Note that the format of the search index changed in Rust 1.40.  We don’t support the old index
//! format.  As the format of the HTML files is not specified, rusty-man might not work with new
//! Rust versions that change the documentation format.

mod doc;
mod index;
mod parser;
mod source;
mod viewer;

use std::io;
use std::path;

use structopt::StructOpt;

/// Command-line viewer for rustdoc documentation
#[derive(Debug, StructOpt)]
struct Opt {
    /// The keyword to open the documentation for, e. g. `rand_core::RngCore`
    keyword: doc::Name,

    /// The sources to check for documentation generated by rustdoc
    ///
    /// Typically, this is the path of a directory containing the documentation for one or more
    /// crates in subdirectories.
    #[structopt(name = "source", short, long, number_of_values = 1)]
    source_paths: Vec<String>,

    /// The viewer for the rustdoc documentation (one of: plain, rich)
    #[structopt(long, parse(try_from_str = viewer::get_viewer))]
    viewer: Option<Box<dyn viewer::Viewer>>,

    /// Do not search the default documentation sources
    ///
    /// If this option is not set, rusty-man appends `/usr/share/doc/rust{,-doc}/html` and
    /// `target/doc` directory to the list of sources if they exist.
    #[structopt(long)]
    no_default_sources: bool,

    /// Do not read the search index if there is no exact match
    ///
    /// Per default, rusty-man reads the search indexes of all sources and tries to find matching
    /// items if there is no exact match for the keyword.  If this option is set, the search
    /// indexes are not read.
    #[structopt(long)]
    no_search: bool,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    let sources = load_sources(&opt.source_paths, !opt.no_default_sources)?;
    let doc = if let Some(doc) = find_doc(&sources, &opt.keyword)? {
        Some(doc)
    } else if !opt.no_search {
        search_doc(&sources, &opt.keyword)?
    } else {
        anyhow::bail!("Could not find documentation for {}", &opt.keyword);
    };

    if let Some(doc) = doc {
        let viewer = opt.viewer.unwrap_or_else(viewer::get_default);
        viewer.open(&doc)
    } else {
        // item selection cancelled by user
        Ok(())
    }
}

const DEFAULT_SOURCES: &[&str] = &[
    "/usr/share/doc/rust/html",
    "/usr/share/doc/rust-doc/html",
    "./target/doc",
];

/// Load all sources given as a command-line argument and, if enabled, the default sources.
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

/// Find the documentation for an item with the given name (exact matches only).
fn find_doc(
    sources: &[Box<dyn source::Source>],
    name: &doc::Name,
) -> anyhow::Result<Option<doc::Doc>> {
    let fqn: doc::Fqn = name.clone().into();
    if let Some(krate) = find_crate(sources, fqn.krate()) {
        krate
            .find_item(&fqn)?
            .or_else(|| krate.find_module(&fqn))
            .or_else(|| krate.find_member(&fqn))
            .map(|i| i.load_doc())
            .transpose()
    } else {
        Ok(None)
    }
}

/// Find the crate with the given name.
fn find_crate(sources: &[Box<dyn source::Source>], name: &str) -> Option<doc::Crate> {
    sources.iter().filter_map(|s| s.find_crate(name)).next()
}

/// Use the search index to find the documentation for an item that partially matches the given
/// keyword.
fn search_doc(
    sources: &[Box<dyn source::Source>],
    name: &doc::Name,
) -> anyhow::Result<Option<doc::Doc>> {
    if let Some(item) = search_item(sources, name)? {
        use anyhow::Context;

        let doc = find_doc(sources, &item.name)?
            .with_context(|| format!("Could not find documentation for {}", &item.name))?;
        Ok(Some(doc))
    } else {
        Ok(None)
    }
}

/// Use the search index to find an item that partially matches the given keyword.
fn search_item(
    sources: &[Box<dyn source::Source>],
    name: &doc::Name,
) -> anyhow::Result<Option<index::IndexItem>> {
    let indexes = sources
        .iter()
        .filter_map(|s| s.load_index().transpose())
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut items = indexes
        .iter()
        .map(|i| i.find(name))
        .collect::<Vec<_>>()
        .concat();
    items.sort_unstable();
    items.dedup();

    if items.is_empty() {
        Err(anyhow::anyhow!(
            "Could not find documentation for {}",
            &name
        ))
    } else if items.len() == 1 {
        Ok(Some(items[0].clone()))
    } else {
        select_item(&items, name)
    }
}

/// Let the user select an item from the given list of matches.
fn select_item(
    items: &[index::IndexItem],
    name: &doc::Name,
) -> anyhow::Result<Option<index::IndexItem>> {
    use std::io::Write;

    // If we are not on a TTY, we can’t ask the user to select an item --> abort
    anyhow::ensure!(
        termion::is_tty(&io::stdin()),
        "Found multiple matches for {}",
        name
    );

    println!("Found mulitple matches for {} – select one of:", name);
    println!();
    let width = (items.len() + 1).to_string().len();
    for (i, item) in items.iter().enumerate() {
        println!("[ {:width$} ] {}", i, &item, width = width);
    }
    println!();
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if let Ok(i) = usize::from_str_radix(input.trim(), 10) {
        Ok(items.get(i).map(Clone::clone))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use crate::source;

    pub fn ensure_docs() -> path::PathBuf {
        let doc = path::PathBuf::from("./target/doc");
        assert!(
            doc.is_dir(),
            "You have to run `cargo doc` before running this test case."
        );
        doc
    }

    #[test]
    fn test_find_doc() {
        let path = ensure_docs();
        let sources = vec![source::get_source(path).unwrap()];

        assert!(super::find_doc(&sources, &"kuchiki".to_owned().into())
            .unwrap()
            .is_some());
        assert!(
            super::find_doc(&sources, &"kuchiki::NodeRef".to_owned().into())
                .unwrap()
                .is_some()
        );
        assert!(
            super::find_doc(&sources, &"kuchiki::NodeDataRef::as_node".to_owned().into())
                .unwrap()
                .is_some()
        );
        assert!(
            super::find_doc(&sources, &"kuchiki::traits".to_owned().into())
                .unwrap()
                .is_some()
        );
        assert!(super::find_doc(&sources, &"kachiki".to_owned().into())
            .unwrap()
            .is_none());
    }
}
