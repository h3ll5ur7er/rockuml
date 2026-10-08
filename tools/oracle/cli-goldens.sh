#!/usr/bin/env bash
# Records what the golden model's command line does in each scenario of tests/cli.
#
# A scenario is a directory holding
#   args     the arguments, one per line; <WORKDIR> stands for the directory the command runs in
#   input/   files copied into the empty directory the command runs in (optional)
#   stdin    the command's standard input (optional)
#   env      NAME=value lines added to the environment (optional)
# The run is recorded in its expected/ directory:
#   status   the exit status
#   stdout   standard output
#   stderr   standard error, the working directory written as <WORKDIR>
#   files/   every file the command created or changed
#
# Usage: cli-goldens.sh [tests/cli/<scenario> ...]   (default: every scenario)
set -euo pipefail

source "$(dirname "$0")/common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

classes="$oracle_dir/build/cli-goldens"
mkdir -p "$classes"
"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$(native_path "$reference_jar")" -d "$(native_path "$classes")" \
	"$(native_path "$oracle_dir/cli-goldens/CliGolden.java")"
# java.exe on Windows wants ';' between classpath entries.
separator=":"
[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
class_path="$(native_path "$reference_jar")$separator$(native_path "$classes")"

# Replaces the working directory, in each spelling Java may print it, by <WORKDIR>.
mask_workdir() {
	local file="$1" work="$2" text spelling
	text="$(cat "$file"; printf x)"
	text="${text%x}"
	local spellings=("$work" "$(native_path "$work")")
	command -v cygpath > /dev/null && spellings+=("$(cygpath -w "$work")")
	for spelling in "${spellings[@]}"; do
		text="${text//"$spelling"/<WORKDIR>}"
	done
	printf '%s' "$text" > "$file"
}

copy_changed_files() {
	local input="$1" work="$2" target="$3" path
	(cd "$work" && find . -type f -print) | sort | while read -r path; do
		path="${path#./}"
		if [[ -f "$input/$path" ]] && cmp -s "$input/$path" "$work/$path"; then
			continue
		fi
		mkdir -p "$target/$(dirname "$path")"
		cp "$work/$path" "$target/$path"
	done
}

generate() {
	local scenario expected work status
	scenario="$(cd "$1" && pwd)"
	expected="$scenario/expected"
	work="$(mktemp -d)"
	if [[ -d "$scenario/input" ]]; then
		cp -R "$scenario/input/." "$work/"
	fi
	local environment=()
	if [[ -f "$scenario/env" ]]; then
		mapfile -t environment < "$scenario/env"
	fi
	local stdin=/dev/null
	if [[ -f "$scenario/stdin" ]]; then
		stdin="$scenario/stdin"
	fi
	rm -rf "$expected"
	mkdir -p "$expected/files"
	set +e
	(cd "$work" && env "${environment[@]}" "$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en \
		-Duser.country=US -cp "$class_path" CliGolden "$(native_path "$scenario/args")" \
		< "$stdin" > "$expected/stdout" 2> "$expected/stderr")
	status=$?
	set -e
	echo "$status" > "$expected/status"
	mask_workdir "$expected/stderr" "$work"
	copy_changed_files "$scenario/input" "$work" "$expected/files"
	rm -rf "$work"
	echo "$(basename "$scenario"): exit status $status"
}

if [[ $# -eq 0 ]]; then
	set -- "$repo_root"/tests/cli/*/
fi
for scenario in "$@"; do
	generate "$scenario"
done
