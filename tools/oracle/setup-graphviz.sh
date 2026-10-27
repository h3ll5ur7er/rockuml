#!/usr/bin/env bash
# Unpacks Graphviz from the distribution's packages into tools/graphviz (git-ignored), without installing it,
# and prints the path of a `dot` that runs from there. Debian and Ubuntu only.
#
# The `@startdot` goldens and their parity cases run an external `dot`; every other golden must come from
# Smetana, which PlantUML uses only when it finds no `dot`, so Graphviz must never be installed where the
# oracle runs. Point GRAPHVIZ_DOT at the printed path instead:
#
#   GRAPHVIZ_DOT="$(bash tools/oracle/setup-graphviz.sh)" bash tools/oracle/generate-goldens.sh tests/corpus/dot/*.puml
#
# The `dot` it prints measures text with rockuml's Liberation fonts alone (graphviz-fonts.conf), so that
# layouts do not depend on the fonts a machine happens to have.
set -euo pipefail

source_dir="$(cd "$(dirname "$0")" && pwd)"
target="$(cd "$source_dir/../.." && pwd)/tools/graphviz"
dot="$target/dot"

is_installed() {
	dpkg-query -W -f='${Status}' "$1" 2> /dev/null | grep -q 'install ok installed'
}

# Built from the graphviz source package, whose own binary package carries no Source field.
is_graphviz_package() {
	local source
	source="$(apt-cache show --no-all-versions "$1" | sed -n 's/^Source: \([^ ]*\).*/\1/p')"
	[[ "${source:-$1}" == graphviz ]]
}

if [[ ! -x "$dot" ]]; then
	mkdir -p "$target/packages" "$target/root"
	# Graphviz's own packages always, so that `dot`, its libraries and its plugins all come from here; of the
	# others, those this machine lacks, the ones it has being found where they are.
	packages=()
	while read -r package; do
		if is_graphviz_package "$package" || ! is_installed "$package"; then
			packages+=("$package")
		fi
	done < <(apt-cache depends --recurse --no-recommends --no-suggests --no-conflicts --no-breaks \
		--no-replaces --no-enhances graphviz | grep '^[a-z0-9]' | sort -u)
	(cd "$target/packages" && apt-get download "${packages[@]}" > /dev/null)
	for deb in "$target"/packages/*.deb; do
		dpkg -x "$deb" "$target/root"
	done
	libgvc="$(find "$target/root" -name 'libgvc.so.6' | head -n 1)"
	if [[ ! -x "$target/root/usr/bin/dot" || -z "$libgvc" ]]; then
		echo "setup-graphviz.sh: the unpacked packages lack usr/bin/dot or libgvc.so.6" >&2
		exit 1
	fi
	libraries="$(dirname "$libgvc")"
	cat > "$dot" <<-EOF
		#!/bin/sh
		# Graphviz unpacked by tools/oracle/setup-graphviz.sh.
		export LD_LIBRARY_PATH="$libraries\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
		export GVBINDIR="$libraries/graphviz"
		export FONTCONFIG_FILE="$source_dir/graphviz-fonts.conf"
		exec "$target/root/usr/bin/dot" "\$@"
	EOF
	chmod +x "$dot"
	# Registers the plugins in GVBINDIR, as installing the package does.
	"$dot" -c
fi
echo "$dot"
