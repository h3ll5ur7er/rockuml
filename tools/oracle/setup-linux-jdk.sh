#!/usr/bin/env bash
# Makes a JDK on Linux measure text like Java on Windows, so that the .svg and .png goldens can be generated
# without Windows.
#
# Usage: bash tools/oracle/setup-linux-jdk.sh [JDK 21 home, by default the one `javac` belongs to]
#
# Copies the JDK to tools/jdk/linux and gives it a font configuration (lib/fontconfig.<distribution>.properties;
# Java ignores a generic fontconfig.properties on Linux) whose logical fonts are rockuml's embedded Liberation
# fonts, which have the metrics of Arial, Times New Roman and Courier New. On Windows the logical fonts fall back
# on fonts for other scripts, whose extents raise a composite font's ascent and descent (see rockuml's
# `LOGICAL_MIN_ASCENT`). A padding font with exactly those extents and no glyphs of its own stands in for them.
# The padding font is made from Liberation Sans with fontTools (`pip install fonttools`).
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
source_jdk="${1:-$(dirname "$(dirname "$(readlink -f "$(command -v javac)")")")}"
jdk="$repo_root/tools/jdk/linux"
fonts="$repo_root/crates/rockuml/assets/fonts"

[[ -x "$source_jdk/bin/javac" ]] || { echo "error: no JDK at $source_jdk" >&2; exit 1; }
distribution="$(. /etc/os-release && echo "${NAME%% *}")"

rm -rf "$jdk"
mkdir -p "$jdk"
# Debian's JDK links to files it does not ship (src.zip, an optional accessibility bridge).
cp -rL "$source_jdk"/. "$jdk" 2> /dev/null || true
[[ -x "$jdk/bin/java" ]] || { echo "error: copying $source_jdk failed" >&2; exit 1; }
mkdir -p "$jdk/lib/fonts"

python3 - "$fonts/LiberationSans-Regular.ttf" "$jdk/lib/fonts/RockumlAscentPad.ttf" << 'EOF'
import sys
from fontTools import subset
from fontTools.ttLib import TTFont

font = TTFont(sys.argv[1])
subsetter = subset.Subsetter(subset.Options())
subsetter.populate(unicodes=[0x20])
subsetter.subset(font)
# In 2048ths of the size, like rockuml's LOGICAL_MIN_ASCENT and LOGICAL_MIN_DESCENT.
ascent, descent = 2059, 450
hhea, os2 = font["hhea"], font["OS/2"]
hhea.ascent, hhea.descent, hhea.lineGap = ascent, -descent, 0
os2.sTypoAscender, os2.sTypoDescender, os2.sTypoLineGap = ascent, -descent, 0
os2.usWinAscent, os2.usWinDescent = ascent, descent
names = font["name"]
for record in list(names.names):
    if record.nameID in (1, 3, 4, 6, 16):
        name = "RockumlAscentPad" if record.nameID in (3, 6) else "Rockuml Ascent Pad"
        names.setName(name, record.nameID, record.platformID, record.platEncID, record.langID)
font.save(sys.argv[2])
EOF

{
	echo "version=1"
	for logical in serif:Serif sansserif:Sans monospaced:Mono dialog:Sans dialoginput:Mono; do
		for style in "plain:" "bold: Bold" "italic: Italic" "bolditalic: Bold Italic"; do
			echo "${logical%%:*}.${style%%:*}.latin-1=Liberation ${logical##*:}${style#*:}"
			echo "${logical%%:*}.${style%%:*}.pad=Rockuml Ascent Pad"
		done
	done
	echo "sequence.allfonts=latin-1,pad"
	for family in Serif Sans Mono; do
		for style in ":Regular" " Bold:Bold" " Italic:Italic" " Bold Italic:BoldItalic"; do
			name="Liberation $family${style%%:*}"
			echo "filename.${name// /_}=$fonts/Liberation$family-${style##*:}.ttf"
		done
	done
	echo "filename.Rockuml_Ascent_Pad=$jdk/lib/fonts/RockumlAscentPad.ttf"
} > "$jdk/lib/fontconfig.$distribution.properties"

echo "Set up $jdk"
