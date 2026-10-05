#!/usr/bin/env bash
# Regenerates the golden outputs for every case in tests/corpus (or for the given .puml files).
# The goldens are written next to each case, exactly where PlantUML puts them, so rockuml
# can be run the same way and compared file by file.
set -euo pipefail

source "$(dirname "$0")/common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

golden_formats=(debug svg)
requested_cases=("$@")

list_cases() {
	if ((${#requested_cases[@]} > 0)); then
		printf '%s\0' "${requested_cases[@]}"
	else
		find "$corpus_dir" -name '*.puml' -print0
	fi
}

remove_stale_goldens() {
	local case_file stem extension
	while IFS= read -r -d '' case_file; do
		stem="${case_file%.puml}"
		for extension in preproc "${golden_formats[@]}"; do
			rm -f "$stem.$extension" "$stem"_[0-9][0-9][0-9]."$extension"
		done
	done
}

render_with_reference() {
	# Cases that are deliberate syntax errors make PlantUML exit non-zero; their error image is still the golden.
	list_cases | xargs -0 -n 200 "$oracle_dir/reference-plantuml.sh" "$@" > /dev/null 2>&1 || true
}

list_cases | remove_stale_goldens

echo "Preprocessed text..."
render_with_reference -preproc
for format in "${golden_formats[@]}"; do
	echo "Format $format..."
	render_with_reference -f "$format"
done

echo "Goldens generated for $(list_cases | tr -cd '\0' | wc -c) cases."
