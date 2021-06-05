// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

#[path = "../src/test_utils.rs"]
mod test_utils;

use std::env;
use std::path;
use std::process;

use assert_cmd::prelude::*;

use test_utils::with_rustdoc;

fn run(path: impl AsRef<path::Path>, args: &[&str]) -> assert_cmd::assert::Assert {
    process::Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .args(&["--no-default-sources", "--source"])
        .arg(path.as_ref())
        .args(&["--viewer", "plain"])
        .args(&["--width", "100"])
        .args(args)
        .assert()
}

fn get_stdout(path: impl AsRef<path::Path>, args: &[&str]) -> String {
    let cmd = run(path, args).success().stderr("");
    String::from_utf8(cmd.get_output().stdout.clone()).unwrap()
}

macro_rules! generate_run {
    ($name:ident $version:literal $args:expr) => {
        #[test]
        fn $name() {
            with_rustdoc($version, |version, path| {
                insta::assert_snapshot!(
                    format!("{}_{}", version, std::stringify!($name)),
                    get_stdout(path, $args)
                );
            });
        }
    };
}

macro_rules! assert_doc {
    ($( $name:ident($version:literal): $item:literal ),* $(,)? ) => {
        $(
            generate_run!($name $version &[$item]);
        )*
    };
}

macro_rules! assert_examples {
    ($( $name:ident($version:literal): $item:literal ),* $(,)? ) => {
        $(
            generate_run!($name $version &["-e", $item]);
        )*
    };
}

assert_doc![
    mod_anyhow("*"): "anyhow",
    macro_anyhow_anyhow("*"): "anyhow::anyhow",
    macro_anyhow_ensure("*"): "anyhow::ensure",
    struct_anyhow_error("<1.51.0"): "anyhow::Error",
    trait_anyhow_context("*"): "anyhow::Context",
    type_anyhow_result("*"): "anyhow::Result",
];

assert_doc![
    mod_log("*"): "log",
    macro_log_debug("*"): "log::debug",
    struct_log_metadata("*"): "log::Metadata",
    enum_log_level("*"): "log::Level",
    constant_log_static_max_level("*"): "log::STATIC_MAX_LEVEL",
    trait_log_log("*"): "log::Log",
    fn_log_logger("*"): "log::logger",
    fn_log_set_logger_racy("*"): "log::set_logger_racy",
];

assert_doc![
    mod_rand_core("*"): "rand_core",
    trait_rand_core_rngcore("*"): "rand_core::RngCore",
    trait_rand_core_seedablerng("*"): "rand_core::SeedableRng",
    struct_rand_core_block_blockrng("*"): "rand_core::block::BlockRng",
];

assert_examples![
    examples_mod_anyhow("*"): "anyhow",
    examples_mod_log(">1.40.0"): "log",
    examples_struct_rand_core_rngcore("*"): "rand_core::RngCore",
];
