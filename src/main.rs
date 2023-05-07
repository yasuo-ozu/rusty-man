// SPDX-FileCopyrightText: 2020-2021 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! rusty-man is a command-line viewer for documentation generated by `rustdoc`.
//!
//! rusty-man opens the documentation for a given keyword.  It performs these steps to find the
//! documentation for an item:
//! 1. The sources, currently only local directories, are loaded, see the `load_sources` function
//!    and the `source` module.  Per default, we look for documentation in the directory
//!    `share/doc/rust{,-doc}/html` relative to the Rust installation path (`rustc --print sysroot`
//!    or `usr`) and the `doc` directory relative to the Cargo target directory
//!    (`$CARGO_TARGET_DIR`, `$CARGO_BUILD_TARGET_DIR` or `./target`).
//! 2. We try to look up the given keyword in all available sources, see the `parser` and the
//!    `source` module for the lookup logic and the `doc` module for the loaded documentation.
//! 3. If we didn’t find a match in the previous step, we load the search index from the
//!    `search-index.js` file for all sources and try to find a matching item.  If we find one, we
//!    open the documentation for that item as in step 2.  See the `search_doc` function and the
//!    `index` module.
//!
//! If we found a documentation item, we use a viewer to open it – see the `viewer` module.
//! Currently, there are three viewer implementations:  `plain` converts the documentaion to plain
//! text, `rich` adds some formatting to it.  Both viewers pipe their output through a pager, if
//! available.  The third viewer, `tui`, provides an interactive interface for browsing the
//! documentation.
//!
//! The documentation is scraped from the HTML files generated by `rustdoc`.  See the `parser`
//! module for the scraping and the `doc::Doc` struct for the structure of the documentation items.
//! For details on the structure of the HTML files and the search index, you have to look at the
//! `html::render` module in the `librustdoc` source code.
//!
//! Note that the format of the search index changed in Rust 1.44.  We don’t support the old index
//! format.  As the format of the HTML files is not specified, rusty-man might not work with new
//! Rust versions that change the documentation format.

// We have to disable some clippy lints as our MSRV is 1.40:
#![allow(
    // slice::strip_suffix added in 1.51
    clippy::manual_strip,
)]

mod args;
mod doc;
mod index;
mod parser;
mod source;
#[cfg(test)]
mod test_utils;
mod viewer;

use std::env;
use std::io;
use std::path;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = args::Args::load()?;
    let sources = load_sources(&args.source_paths, !args.no_default_sources)?;
    let doc = if let Some(doc) = sources.find(&args.keyword, None)? {
        Some(doc)
    } else if !args.no_search {
        search_doc(&sources, &args.keyword)?
    } else {
        anyhow::bail!("Could not find documentation for {}", &args.keyword);
    };

    if let Some(doc) = doc {
        if args.open {
            if let Some(url) = doc.url.as_ref() {
                Ok(open::that(url)?)
            } else {
                anyhow::bail!("Cannot find html document");
            }
        } else {
            let viewer = args.viewer.unwrap_or_else(viewer::get_default);
            if args.examples {
                let examples = doc.find_examples()?;
                anyhow::ensure!(
                    !examples.is_empty(),
                    "Could not find examples for {}",
                    &args.keyword
                );
                viewer.open_examples(sources, args.viewer_args, &doc, examples)
            } else {
                viewer.open(sources, args.viewer_args, &doc)
            }
        }
    } else {
        // item selection cancelled by user
        Ok(())
    }
}

/// Load all sources given as a command-line argument and, if enabled, the default sources.
fn load_sources(sources: &[String], load_default_sources: bool) -> anyhow::Result<source::Sources> {
    let mut vec = Vec::new();

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

    Ok(source::Sources::new(vec))
}

fn get_default_sources() -> Vec<path::PathBuf> {
    let mut default_sources = Vec::new();

    if let Some(rustup_doc) = get_rustup_doc() {
        default_sources.push(rustup_doc)
    } else {
        let sysroot = get_sysroot().unwrap_or_else(|| path::PathBuf::from("/usr"));
        default_sources.push(sysroot.join("share/doc/rust/html"));
        default_sources.push(sysroot.join("share/doc/rust-doc/html"));
    }

    let mut target_dir = get_target_dir();
    target_dir.push("doc");
    default_sources.push(target_dir);

    default_sources
}

fn get_rustup_doc() -> Option<path::PathBuf> {
    use std::process::Command;
    let output = Command::new("rustup")
        .args(["doc", "--path"])
        .output()
        .ok()?;
    if output.status.success() {
        let mut ans: path::PathBuf = String::from_utf8(output.stdout).ok()?.parse().ok()?;
        if ans.pop() {
            Some(ans)
        } else {
            None
        }
    } else {
        None
    }
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

fn get_target_dir() -> path::PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .or_else(|| env::var_os("CARGO_BUILD_TARGET_DIR"))
        .map(From::from)
        .unwrap_or_else(|| "./target".into())
}

/// Use the search index to find the documentation for an item that partially matches the given
/// keyword.
fn search_doc(sources: &source::Sources, name: &doc::Name) -> anyhow::Result<Option<doc::Doc>> {
    if let Some(item) = search_item(sources, name)? {
        use anyhow::Context;

        let doc = sources
            .find(&item.name, Some(item.ty))?
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
    sources: &source::Sources,
    name: &doc::Name,
) -> anyhow::Result<Option<index::IndexItem>> {
    let items = sources.search(name)?;
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
    use std::io::Write;
    use std::str::FromStr;

    // If we are not on a TTY, we can’t ask the user to select an item --> abort
    anyhow::ensure!(
        termion::is_tty(&io::stdin()),
        "Found multiple matches for {}",
        name
    );

    println!("Found multiple matches for {} – select one of:", name);
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
    if let Ok(i) = usize::from_str(input.trim()) {
        Ok(items.get(i).map(Clone::clone))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::source;
    use crate::test_utils::{with_rustdoc, Format};

    #[test]
    fn test_find_doc() {
        with_rustdoc("*", Format::all(), |_, _, path| {
            let sources = source::Sources::new(vec![source::get_source(path).unwrap()]);

            assert!(sources
                .find(&"kuchiki".to_owned().into(), None)
                .unwrap()
                .is_some());
            assert!(sources
                .find(&"kuchiki::NodeRef".to_owned().into(), None)
                .unwrap()
                .is_some());
            assert!(sources
                .find(&"kuchiki::NodeDataRef::as_node".to_owned().into(), None)
                .unwrap()
                .is_some());
            assert!(sources
                .find(&"kuchiki::traits".to_owned().into(), None)
                .unwrap()
                .is_some());
            assert!(sources
                .find(&"kachiki".to_owned().into(), None)
                .unwrap()
                .is_none());
        });
    }
}
