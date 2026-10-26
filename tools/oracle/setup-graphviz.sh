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

if [[ ! -x "$dot" ]]; then
	mkdir -p "$target/packages" "$target/root"
	# The packages graphviz needs that this machine lacks: those it has are found where they are.
	packages=()
	while read -r package; do
		if ! dpkg-query -W -f='${Status}' "$package" 2> /dev/null | grep -q 'install ok installed'; then
			packages+=("$package")
		fi
	done < <(apt-cache depends --recurse --no-recommends --no-suggests --no-conflicts --no-breaks \
		--no-replaces --no-enhances graphviz | grep '^[a-z0-9]' | sort -u)
	(cd "$target/packages" && apt-get download "${packages[@]}" > /dev/null)
	for deb in "$target"/packages/*.deb; do
		dpkg -x "$deb" "$target/root"
	done
	libraries="$(dirname "$(find "$target/root" -name 'libgvc.so.6' | head -n 1)")"
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
