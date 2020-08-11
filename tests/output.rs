// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::env;
use std::process;
use std::sync;

use assert_cmd::prelude::*;

static mut DIR: Option<tempfile::TempDir> = None;
static INIT: sync::Once = sync::Once::new();

fn run_cargo_doc() {
    INIT.call_once(|| {
        let dir = tempfile::tempdir().unwrap();
        process::Command::new(env::var_os("CARGO").unwrap())
            .arg("doc")
            .args(&["--package", "anyhow"])
            .args(&["--package", "log"])
            .args(&["--package", "rand_core"])
            .arg("--target-dir")
            .arg(dir.path())
            .output()
            .unwrap();
        unsafe {
            DIR = Some(dir);
        }
    });
}

fn run(args: &[&str]) -> assert_cmd::assert::Assert {
    run_cargo_doc();

    let dir = unsafe { DIR.as_ref().unwrap() };

    process::Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .args(&["--no-default-sources", "--source"])
        .arg(dir.path().join("doc"))
        .args(&["--viewer", "plain"])
        .args(&["--width", "100"])
        .args(args)
        .assert()
}

fn get_stdout(args: &[&str]) -> String {
    let cmd = run(args).success().stderr("");
    String::from_utf8(cmd.get_output().stdout.clone()).unwrap()
}

fn assert_doc(item: &str) {
    insta::assert_snapshot!(get_stdout(&[item]));
}

fn assert_examples(item: &str) {
    insta::assert_snapshot!(get_stdout(&["-e", item]));
}

macro_rules! assert_doc {
    ($( $( #[$attrs:meta] )* $name:ident: $item:literal ),* $(,)? ) => {
        $(
            #[test]
            $( #[$attrs] )*
            fn $name() {
                assert_doc($item);
            }
        )*
    };
}

macro_rules! assert_examples {
    ($( $( #[$attrs:meta] )* $name:ident: $item:literal ),* $(,)? ) => {
        $(
            #[test]
            $( #[$attrs] )*
            fn $name() {
                assert_examples($item);
            }
        )*
    };
}

assert_doc![
    test_mod_anyhow: "anyhow",
    test_macro_anyhow_anyhow: "anyhow::anyhow",
    test_macro_anyhow_ensure: "anyhow::ensure",
    #[ignore]
    test_struct_anyhow_error: "anyhow::Error",
    test_trait_anyhow_context: "anyhow::Context",
    test_type_anyhow_result: "anyhow::Result",
];

assert_doc![
    #[ignore]
    test_mod_log: "log",
    #[ignore]
    test_macro_log_debug: "log::debug",
    #[ignore]
    test_struct_log_metadata: "log::Metadata",
    test_enum_log_level: "log::Level",
    #[ignore]
    test_constant_log_static_max_level: "log::STATIC_MAX_LEVEL",
    test_trait_log_log: "log::Log",
    test_fn_log_logger: "log::logger",
    test_fn_log_set_logger_racy: "log::set_logger_racy",
];

assert_doc![
    test_mod_rand_core: "rand_core",
    test_trait_rand_core_rngcore: "rand_core::RngCore",
    test_trait_rand_core_seedablerng: "rand_core::SeedableRng",
    test_struct_rand_core_block_blockrng: "rand_core::block::BlockRng",
];

assert_examples![
    test_examples_mod_anyhow: "anyhow",
    #[ignore]
    test_examples_mod_log: "log",
    test_examples_struct_rand_core_rngcore: "rand_core::RngCore",
];
