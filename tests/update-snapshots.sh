#!/bin/bash
# SPDX-FileCopyrightText: 2021 Robin Krahl <robin.krahl@ireas.org>
# SPDX-License-Identifier: MIT

set -euo pipefail

if [ $# -ne 2 ]
then
	echo "Usage: $0 <old_version> <version>" >&2
	exit 1
fi

old_version=$1
version=$2
tests_dir=$(dirname $(readlink -f $0))
snapshot_dir=$tests_dir/snapshots

old_prefix=output__$old_version
new_prefix=output__$version

for old_snapshot in "$snapshot_dir/$old_prefix"*
do
	new_snapshot=$(basename "$old_snapshot")
	new_snapshot=$new_prefix${new_snapshot#$old_prefix}
	cp --verbose "$old_snapshot" "$snapshot_dir/$new_snapshot"
done
