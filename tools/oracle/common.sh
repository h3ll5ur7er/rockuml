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

# A fixed locale keeps goldens independent of the machine that generates them.
reference_plantuml() {
	"$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en -Duser.country=US -jar "$reference_jar" "$@"
}

native_path() {
	echo "$1" | to_native_paths
}

# Compiles tools/oracle/<unit>/<Class>.java against the golden-model jar and runs it with the remaining arguments.
# A fixed locale and line separator keep its output the same on every machine.
run_dump() {
	local unit="${1%/*}" class="${1#*/}"
	shift
	[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"
	local classes="$oracle_dir/build/$unit"
	mkdir -p "$classes"
	local jar_path classes_path
	jar_path="$(native_path "$reference_jar")"
	classes_path="$(native_path "$classes")"
	"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$jar_path" -d "$classes_path" \
		"$(native_path "$oracle_dir/$unit/$class.java")"
	# java.exe on Windows wants ';' between classpath entries.
	local separator=":"
	[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
	"$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Dline.separator=$'\n' \
		-cp "$jar_path$separator$classes_path" "$class" "$@" 2> /dev/null
}
