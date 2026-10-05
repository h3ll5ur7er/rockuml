# Shared settings for the oracle scripts; sourced, not executed.

oracle_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$oracle_dir/../.." && pwd)"
reference_jar="$oracle_dir/build/plantuml-ref.jar"
corpus_dir="$repo_root/tests/corpus"

die() {
	echo "error: $*" >&2
	exit 1
}

find_jdk_bin() {
	local java
	java="$(ls -d "$repo_root"/tools/jdk/*/bin 2>/dev/null | head -n 1)"
	[[ -n "$java" ]] || die "no portable JDK in tools/jdk (see tools/oracle/README.md)"
	echo "$java"
}

# javac reads @argfiles natively, so MSYS paths (/b/...) must become B:/... on Windows.
to_native_paths() {
	if command -v cygpath > /dev/null; then
		cygpath -m -f -
	else
		cat
	fi
}

jdk_bin="$(find_jdk_bin)"

reference_plantuml() {
	"$jdk_bin/java" -Djava.awt.headless=true -jar "$reference_jar" "$@"
}
