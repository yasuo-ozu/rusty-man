#!/bin/bash
# SPDX-FileCopyrightText: 2021 Robin Krahl <robin.krahl@ireas.org>
# SPDX-License-Identifier: MIT

set -euo pipefail

if [ $# -ne 1 ]
then
	echo "Usage: $0 <version>" >&2
	exit 1
fi

version=$1
tests_dir=$(dirname $(readlink -f $0))
manifest=$tests_dir/../Cargo.toml
doc_dir=$tests_dir/html/$version

if [ -e "$doc_dir" ]
then
	echo "Error: documentation for version $version already exists" >&2
	exit 1
fi

temp_dir=$(mktemp --directory)

echo "Generating documentation"

CARGO_TARGET_DIR="$temp_dir" cargo +$version doc --manifest-path "$manifest" --no-deps \
	--package anyhow --package kuchiki --package log --package rand_core:0.5.1
mkdir "$doc_dir"
cp "$temp_dir/doc/search-index.js" "$doc_dir"
cp -r "$temp_dir/doc/anyhow" "$doc_dir"
cp -r "$temp_dir/doc/kuchiki" "$doc_dir"
cp -r "$temp_dir/doc/log" "$doc_dir"
cp -r "$temp_dir/doc/rand_core" "$doc_dir"

rm -r "$temp_dir"
