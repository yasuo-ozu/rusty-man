// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! rusty-man is a command-line viewer for documentation generated by `rustdoc`.
//!
//! rusty-man opens the documentation for a given keyword.  It performs these steps to find the
//! documentation for an item:
//! 1. The sources, currently only local directories, are loaded, see the `load_sources` funnction
//!    and the `source` module.  Per default, we look for documentation in the directory
//!    `share/doc/rust{,-doc}/html` relative to the Rust installation path (`rustc --print sysroot`
//!    or `usr`) and in `./target/doc`.
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
//! Currently, there are two viewer implementations:  `plain` converts the documentaion to plain
//! text, `rich` adds some formatting to it.  Both viewers pipe their output through a pager, if
//! available.
//!
//! The documentation is scraped from the HTML files generated by `rustdoc`.  See the `parser`
//! module for the scraping and the `doc::Doc` struct for the structure of the documentation items.
//! For details on the structure of the HTML files and the search index, you have to look at the
//! `html::render` module in the `librustdoc` source code.
//!
//! Note that the format of the search index changed in a recent Rust version (> 1.40 and <= 1.44).
//! We don’t support the old index format.  As the format of the HTML files is not specified,
//! rusty-man might not work with new Rust versions that change the documentation format.

mod args;
mod doc;
mod index;
mod parser;
mod source;
mod viewer;

use std::io;
use std::path;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = args::Args::load()?;
    let sources = load_sources(&args.source_paths, !args.no_default_sources)?;
    let doc = if let Some(doc) = find_doc(&sources, &args.keyword)? {
        Some(doc)
    } else if !args.no_search {
        search_doc(&sources, &args.keyword)?
    } else {
        anyhow::bail!("Could not find documentation for {}", &args.keyword);
    };

    if let Some(doc) = doc {
        let viewer = args.viewer.unwrap_or_else(viewer::get_default);
        if args.examples {
            let examples = doc.find_examples()?;
            anyhow::ensure!(
                !examples.is_empty(),
                "Could not find examples for {}",
                &args.keyword
            );
            viewer.open_examples(args.viewer_args, &doc, examples)
        } else {
            viewer.open(args.viewer_args, &doc)
        }
    } else {
        // item selection cancelled by user
        Ok(())
    }
}

/// Load all sources given as a command-line argument and, if enabled, the default sources.
fn load_sources(
    sources: &[String],
    load_default_sources: bool,
) -> anyhow::Result<Vec<Box<dyn source::Source>>> {
    let mut vec: Vec<Box<dyn source::Source>> = Vec::new();

    if load_default_sources {
        for path in get_default_sources() {
            if path.is_dir() {
                vec.push(source::get_source(&path)?);
            } else {
                log::info!(
                    "Ignoring default source '{}' because it does not exist",
                    path.display()
                );
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

fn get_default_sources() -> Vec<path::PathBuf> {
    let mut default_sources: Vec<path::PathBuf> = Vec::new();

    let sysroot = get_sysroot().unwrap_or_else(|| path::PathBuf::from("/usr"));
    default_sources.push(sysroot.join("share/doc/rust/html"));
    default_sources.push(sysroot.join("share/doc/rust-doc/html"));

    default_sources.push(path::PathBuf::from("./target/doc"));

    default_sources
}

fn get_sysroot() -> Option<path::PathBuf> {
    std::process::Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().into())
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
        log::info!("Could not find crate '{}'", fqn.krate());
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
        log::info!(
            "Could not find documentation for '{}' in the search index",
            name
        );
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
        log::info!("Search returned a single item: '{}'", &items[0].name);
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
    use crossterm::tty::IsTty;
    use std::io::Write;

    // If we are not on a TTY, we can’t ask the user to select an item --> abort
    anyhow::ensure!(io::stdin().is_tty(), "Found multiple matches for {}", name);

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
