# Notices

## rockuml

rockuml is a Rust port of [PlantUML](https://plantuml.com) 1.2026.8, made from the sources of PlantUML's
LGPL edition. PlantUML is (C) Copyright 2009-2024 Arnaud Roques and is distributed under the GNU Lesser
General Public License, version 3 or later.

rockuml is distributed under the same license, LGPL-3.0-or-later:

- [LICENSE](LICENSE) is the GNU Lesser General Public License, version 3;
- [COPYING](COPYING) is the GNU General Public License, version 3, which the LGPL builds on.

rockuml is an independent project. It is not affiliated with or endorsed by the PlantUML project.

## Smetana

[crates/smetana](crates/smetana) ports PlantUML's Smetana, Arnaud Roques' Java translation of the `dot`
layout of [Graphviz](https://graphviz.org) 2.38. Graphviz is Copyright (c) 2011 AT&T Intellectual Property
and its contributors. Like the original C program and Smetana, the crate is distributed under the
Eclipse Public License, version 1.0: see [crates/smetana/LICENSE](crates/smetana/LICENSE).

## Diagrams

The images rockuml makes belong to whoever wrote their source. rockuml's license does not cover them, as
PlantUML's does not cover the images it makes.

## Third-party content

The fonts, emoji, icons, themes and standard library rockuml carries are listed with their authors and
licenses in [THIRD-PARTY.md](THIRD-PARTY.md). The Rust crates compiled into the binaries are listed with
their license texts in `THIRD-PARTY-CRATES.md`, which `tools/third-party-licenses.py` writes into every
release.
