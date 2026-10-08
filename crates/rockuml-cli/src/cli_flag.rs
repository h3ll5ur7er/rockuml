//! PlantUML's command-line flags (`CliFlag`): every spelling, how each takes its value, its help text, and
//! whether rockuml offers it.

/// How a flag takes its value (`Arity`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arity {
    /// No value, e.g. `-verbose`.
    UnaryBoolean,
    /// No value, and the flag does something instead of rendering, e.g. `-help`.
    UnaryImmediateAction,
    /// An optional `:value` suffix, e.g. `-ftp` or `-ftp:8080`.
    UnaryOptionalColon,
    /// A key or key/value right after the flag or as the next argument, e.g. `-DKEY=VALUE` or `-D KEY`.
    UnaryInlineKeyOrKeyValue,
    /// The next argument, e.g. `-o out`.
    BinaryNextArgumentValue,
}

/// What rockuml does with a flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Support {
    Ported,
    /// Accepted without effect: what it changes in PlantUML cannot show in rockuml (Java or Graphviz settings).
    Ignored,
    NotPorted,
    /// Left out of rockuml on purpose (PLAN.md, tier "Drop"): GUI, servers, statistics, clipboard, Graphviz.
    Dropped,
}

/// A flag's line in `--help` (`CliFlagDoc`).
pub(crate) struct CliFlagDoc {
    pub value: &'static str,
    /// `--help` lists level 0, `--help-more` levels 0 and 1.
    pub level: i32,
    pub usage: &'static str,
    /// The heading of the group of flags this one starts.
    pub new_group: &'static str,
}

const fn doc(value: &'static str, level: i32) -> CliFlagDoc {
    CliFlagDoc {
        value,
        level,
        usage: "",
        new_group: "",
    }
}

const fn doc_with_usage(value: &'static str, level: i32, usage: &'static str) -> CliFlagDoc {
    CliFlagDoc {
        value,
        level,
        usage,
        new_group: "",
    }
}

const fn doc_in_new_group(
    value: &'static str,
    level: i32,
    usage: &'static str,
    new_group: &'static str,
) -> CliFlagDoc {
    CliFlagDoc {
        value,
        level,
        usage,
        new_group,
    }
}

/// Documented flags without a level only show in neither help.
const UNLISTED: i32 = 999;

macro_rules! cli_flags {
    ($($name:ident: $flag:literal [$($alias:literal),*] $arity:ident $support:ident $doc:expr, $default:expr;)*) => {
        /// In PlantUML's order, which decides which flag an argument is when several match.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) enum CliFlag {
            $($name,)*
        }

        impl CliFlag {
            pub(crate) const VALUES: &'static [CliFlag] = &[$(CliFlag::$name,)*];

            fn definition(self) -> Definition {
                match self {
                    $(CliFlag::$name => Definition {
                        flag: $flag,
                        aliases: &[$($alias),*],
                        arity: Arity::$arity,
                        support: Support::$support,
                        // A flag's doc is a `CliFlagDoc`, or `None` where PlantUML's is null.
                        doc: $doc.into(),
                        default_value: $default,
                    },)*
                }
            }
        }
    };
}

struct Definition {
    flag: &'static str,
    aliases: &'static [&'static str],
    arity: Arity,
    support: Support,
    doc: Option<CliFlagDoc>,
    default_value: Option<&'static str>,
}

cli_flags! {
    // General
    Help: "--help" ["-h", "-?", "-help"] UnaryImmediateAction Ported
        doc("Show help and usage information", 0), None;
    HelpMore: "--help-more" ["-h:more", "-help:more"] UnaryImmediateAction Ported
        doc("Show extended help (advanced options)", 0), None;
    Version: "--version" ["-version"] UnaryImmediateAction Ported
        doc("Show the rockuml version and the PlantUML release it is compatible with", 0), None;
    Author: "--author" ["--about", "-authors", "-about"] UnaryImmediateAction Ported
        doc("Show information about the authors", 0), None;
    Gui: "--gui" ["-gui"] UnaryBoolean Dropped
        doc("Launch the graphical user interface", 0), None;
    DarkMode: "--dark-mode" ["-darkmode"] UnaryBoolean Dropped
        doc("Render diagrams in dark mode", 0), None;
    Verbose: "--verbose" ["-v", "-verbose"] UnaryBoolean NotPorted
        doc("Enable verbose logging", 0), None;
    Duration: "--duration" ["-duration"] UnaryBoolean Ported
        doc("Print total processing time", 0), None;
    Progress: "--progress-bar" ["-progress"] UnaryBoolean Dropped
        doc("Show a textual progress bar", 0), None;
    Splash: "--splash-screen" ["-splash"] UnaryBoolean Dropped
        doc("Show splash screen with progress bar", 0), None;
    TestDot: "--check-graphviz" ["-testdot"] UnaryImmediateAction Dropped
        doc("Check Graphviz installation", 0), None;
    Picoweb: "--http-server" ["-picoweb"] UnaryOptionalColon Dropped
        doc_with_usage("Start internal HTTP server for rendering (default port : 8080)", 0, "--http-server[:<port>]"), None;
    // Input & preprocessing
    Exclude: "--exclude" ["-x", "-exclude"] BinaryNextArgumentValue Ported
        doc_in_new_group("Exclude input files matching the given pattern", 1, "--exclude <pattern>", "Input & preprocessing"), None;
    Pipe: "--pipe" ["-p", "-pipe"] UnaryBoolean Ported
        doc("Read source from stdin, write result to stdout", 0), None;
    Pipemap: "-pipemap" [] UnaryBoolean NotPorted None, None;
    Pipedelimitor: "-pipedelimitor" [] BinaryNextArgumentValue Ported None, None;
    Pipenostderr: "-pipenostderr" [] UnaryBoolean Ported None, None;
    PipeImageIndex: "--pipe-image-index" ["-pipeimageindex"] BinaryNextArgumentValue Ported
        doc("Generate the Nth image with pipe option", UNLISTED), Some("0");
    Define: "-D" [] UnaryInlineKeyOrKeyValue Ported
        doc_with_usage("Define a preprocessing variable (equivalent to '!define <var> <value>')", 0, "-D, --define <VAR>=<value>"), None;
    DefineLong: "--define" [] BinaryNextArgumentValue Ported None, None;
    Include: "-I" [] UnaryInlineKeyOrKeyValue Ported
        doc_with_usage("Include external file (as with '!include <file>')", 1, "-I, --include <file>"), None;
    IncludeLong: "--include" [] BinaryNextArgumentValue Ported None, None;
    Pragma: "-P" [] UnaryInlineKeyOrKeyValue Ported
        doc_with_usage("Set pragma (equivalent to '!pragma <key> <value>')", 1, "-P, --pragma <key>=<value>"), None;
    PragmaLong: "--pragma" [] BinaryNextArgumentValue Ported None, None;
    Skinparam: "-S" [] UnaryInlineKeyOrKeyValue Ported
        doc_with_usage("Set skin parameter (equivalent to 'skinparam <key> <value>')", 1, "--skinparam <key>=<value>"), None;
    SkinparamLong: "--skinparam" [] BinaryNextArgumentValue Ported None, None;
    Theme: "--theme" ["-theme"] BinaryNextArgumentValue Ported
        doc_with_usage("Apply a theme", 1, "--theme <name>"), None;
    Config: "--config" ["-config"] BinaryNextArgumentValue Ported
        doc_with_usage("Specify configuration file", 1, "--config <file>"), None;
    Charset: "--charset" ["-charset"] BinaryNextArgumentValue Ported
        doc_with_usage("Use a specific input charset", 1, "--charset <name>"), Some("UTF-8");
    // Execution control
    CheckOnly: "--check-syntax" ["--syntax-check", "-checkonly"] UnaryBoolean Ported
        doc_in_new_group("Check diagram syntax without generating images", 0, "", "Execution control"), None;
    FailFast: "--stop-on-error" ["-failfast"] UnaryBoolean Ported
        doc("Stop at the first syntax error", 0), None;
    FailFast2: "--check-before-run" ["-failfast2"] UnaryBoolean Ported
        doc("Pre-check syntax of all inputs and stop faster on error", 0), None;
    NoErrorImage: "--no-error-image" ["-noerror"] UnaryBoolean Ported
        doc("Do not generate error images for diagrams with syntax errors", 0), None;
    Timeout: "--graphviz-timeout" ["-timeout"] BinaryNextArgumentValue Ignored
        doc_with_usage("Set Graphviz processing timeout (in seconds)", 1, "--graphviz-timeout <seconds>"), None;
    IgnoreStartumlFilename: "--ignore-startuml-filename" [] UnaryBoolean Ported
        doc("Ignore '@startuml <name>' and always derive output filenames from input files", 1), None;
    NbThread: "--threads" ["-nbthread"] BinaryNextArgumentValue Ported
        doc_with_usage("Use <n> threads for processing  (auto = available processors)", 1, "--threads <n|auto>"), None;
    Loop: "--loop" [] BinaryNextArgumentValue Ported
        doc_with_usage("Runs the generation process <n> times, useful for performance testing", 1, "--loop <n>"), Some("1");
    // Metadata & assets
    RetrieveMetadata: "--extract-source" ["-metadata"] UnaryBoolean Ported
        doc_in_new_group("Extract embedded PlantUML source from PNG or SVG metadata", 0, "", "Metadata & assets"), None;
    NoMetadata: "--disable-metadata" ["-nometadata"] UnaryBoolean Ported
        doc("Do not include metadata in generated files", 1), None;
    CheckMetadata: "--skip-fresh" ["-checkmetadata"] UnaryBoolean Ported
        doc("Skip PNG/SVG files that are already up-to-date (using metadata)", 0), None;
    EncodeSprite: "--encode-sprite" ["--sprite", "-sprite", "-encodesprite"] UnaryBoolean Ported
        doc_with_usage("Encode a sprite definition from an image file", 0, "--sprite <4|8|16> <file>"), None;
    ComputeUrl: "--encode-url" ["--compute-url", "-computeurl", "-encodeurl"] UnaryBoolean Ported
        doc("Generate an encoded PlantUML URL from a source file", 1), None;
    DecodeUrl: "--decode-url" ["-decodeurl"] UnaryBoolean Ported
        doc_with_usage("Decode a PlantUML encoded URL back to its source", 1, "--decode-url <string>"), None;
    Language: "--list-keywords" ["-language"] UnaryImmediateAction NotPorted
        doc("Print the list of PlantUML language keywords", 1), None;
    GraphvizDot: "--dot-path" ["-graphvizdot", "-graphviz_dot"] BinaryNextArgumentValue Dropped
        doc_with_usage("Specify the path to the Graphviz 'dot' executable", 1, "--dot-path <path-to-dot-exe>"), None;
    Ftp: "--ftp-server" ["-ftp"] UnaryOptionalColon Dropped
        doc("Start a local FTP server for diagram rendering (rarely used)", 1), None;
    // Output control
    OutputDir: "--output-dir" ["-o", "output_dir", "-output", "-odir"] BinaryNextArgumentValue Ported
        doc_in_new_group("Generate output files in the specified directory", 1, "--output-dir <dir>", "Output control"), None;
    Overwrite: "--overwrite" ["--force-overwrite", "-overwrite"] UnaryBoolean Ported
        doc("Allow overwriting of read-only output files", 1), None;
    // Other
    Clipboard: "--clipboard" [] UnaryImmediateAction Dropped None, None;
    Clipboardloop: "--clipboardloop" [] UnaryImmediateAction Dropped None, None;
    DebugSvek: "-debugsvek" ["-debug_svek"] UnaryBoolean NotPorted
        doc("Generate intermediate Svek files", UNLISTED), None;
    FileDir: "-filedir" [] BinaryNextArgumentValue Ported
        doc("Pretend input files are located in given directory", UNLISTED), None;
    Filename: "-filename" [] BinaryNextArgumentValue Ported
        doc("Override %filename% variable", UNLISTED), None;
    Headless: "-headless" [] UnaryBoolean Ignored None, None;
    PrintFonts: "-printfonts" [] UnaryImmediateAction NotPorted
        doc("List fonts available on your system", UNLISTED), None;
    StdLib: "-stdlib" [] UnaryImmediateAction NotPorted
        doc("Print standard library information", UNLISTED), None;
    Stdrpt: "-stdrpt" [] UnaryOptionalColon NotPorted None, None;
    Syntax: "-syntax" [] UnaryBoolean NotPorted
        doc("Report syntax errors from stdin without generating images", UNLISTED), None;
    License: "-license" ["-licence"] UnaryImmediateAction Ported None, None;
    Word: "-word" [] UnaryBoolean Dropped None, None;
    UseSeparatorMinus: "-useseparatorminus" [] UnaryBoolean NotPorted None, None;
    // Output format
    Format: "--format" ["-f"] BinaryNextArgumentValue Ported
        doc_in_new_group("Set the output format for generated diagrams\n(png, svg, svg-deterministic, debug, preproc, null)", 0, "-f, --format <name>", "Output format (choose one)"), None;
    TEps: "--eps" ["-teps", "-eps"] UnaryBoolean NotPorted
        doc_in_new_group("Generate images in EPS format", 0, "", "Available formats"), None;
    TEpsText: "--teps:text" ["-teps:text", "-eps:text"] UnaryBoolean NotPorted None, None;
    THtml: "--html" ["-thtml", "-html"] UnaryBoolean NotPorted
        doc("Generate HTML files for class diagrams", 1), None;
    TLatex: "--latex" ["-tlatex", "-latex"] UnaryBoolean NotPorted
        doc("Generate LaTeX/TikZ output", 0), None;
    TLatexNopreamble: "--latex-nopreamble" ["-tlatex:nopreamble", "-latex:nopreamble"] UnaryBoolean NotPorted
        doc("Generate LaTeX/TikZ output without preamble", 1), None;
    Obfuscate: "--obfuscate" ["-cypher"] UnaryBoolean NotPorted
        doc("Replace text in diagrams with obfuscated strings to share diagrams safely", 0), None;
    TPdf: "--pdf" ["-tpdf", "-pdf"] UnaryBoolean NotPorted
        doc("Generate PDF images", 1), None;
    TPng: "--png" ["-tpng", "-png"] UnaryBoolean Ported
        doc("Generate PNG images (default)", 0), None;
    Preprocess: "--preproc" ["-preproc"] UnaryBoolean Ported
        doc("Generate the preprocessed source after applying !include, !define... (no rendering)", 0), None;
    TScxml: "--scxml" ["-tscxml"] UnaryBoolean NotPorted
        doc("Generate SCXML files for state diagrams", 1), None;
    TSvg: "--svg" ["-tsvg", "-svg"] UnaryBoolean Ported
        doc("Generate SVG images", 0), None;
    TNull: "--null" [] UnaryBoolean Ported None, None;
    TTxt: "--txt" ["-ttxt", "-txt"] UnaryBoolean NotPorted
        doc("Generate ASCII art diagrams", 0), None;
    TUtxt: "--utxt" ["-tutxt", "-utxt"] UnaryBoolean NotPorted
        doc("Generate ASCII art diagrams using Unicode characters", 0), None;
    TVdx: "--vdx" ["-tvdx", "-vdx"] UnaryBoolean NotPorted
        doc("Generate VDX files", 1), None;
    TXmi: "--xmi" ["-txmi", "-xmi"] UnaryBoolean NotPorted
        doc("Generate XMI files for class diagrams", 1), None;
    TXmiArgo: "--xmi:argo" ["-xmi:argo"] UnaryBoolean NotPorted None, None;
    TXmiCustom: "--xmi:custom" ["-xmi:custom"] UnaryBoolean NotPorted None, None;
    TXmiScript: "--xmi:script" ["-xmi:script"] UnaryBoolean NotPorted None, None;
    TXmiStar: "--xmi:star" ["-xmi:star"] UnaryBoolean NotPorted None, None;
    TBase64: "--base64" [] UnaryBoolean NotPorted None, None;
    TBraille: "--braille" [] UnaryBoolean NotPorted None, None;
    // Statistics
    DisableStats: "--disable-stats" ["-disablestats"] UnaryBoolean Ignored
        doc_in_new_group("Disable statistics collection (default behavior)", 1, "", "Statistics"), None;
    EnableStats: "--enable-stats" ["-enablestats"] UnaryBoolean Dropped
        doc("Enable statistics collection", 1), None;
    Dumphtmlstats: "--export-stats-html" [] UnaryImmediateAction Dropped
        doc("Export collected statistics to an HTML report and exit", 1), None;
    Dumpstats: "--export-stats" [] UnaryImmediateAction Dropped
        doc("Export collected statistics to a text report and exit", 1), None;
    HtmlStats: "--html-stats" ["-htmlstats"] UnaryBoolean Dropped
        doc("Output general statistics in HTML format", 1), None;
    XmlStats: "--xml-stats" ["-xmlstats"] UnaryBoolean Dropped
        doc("Output general statistics in XML format", 1), None;
    RealtimeStats: "--realtime-stats" ["-realtimestats"] UnaryBoolean Dropped
        doc("Generate statistics in real time during processing", 1), None;
    LoopStats: "--loop-stats" ["-loopstats"] UnaryImmediateAction Dropped
        doc("Continuously print usage statistics during execution", 1), None;
    // rockuml's own
    Font: "--font" ["-font"] BinaryNextArgumentValue Ported
        doc_in_new_group("Also measure and draw text with this font file, or the fonts in this directory", 0, "--font <file|dir>", "Fonts"), None;
}

impl CliFlag {
    pub(crate) fn flag(self) -> &'static str {
        self.definition().flag
    }

    pub(crate) fn arity(self) -> Arity {
        self.definition().arity
    }

    pub(crate) fn support(self) -> Support {
        self.definition().support
    }

    pub(crate) fn doc(self) -> Option<CliFlagDoc> {
        self.definition().doc
    }

    pub(crate) fn default_value(self) -> Option<&'static str> {
        self.definition().default_value
    }

    /// Whether `argument` is this flag. Single letters taking a key (`-D`, `-I`, `-P`, `-S`) are a
    /// case-sensitive prefix; flags with an optional `:value` a case-insensitive prefix; all others the
    /// whole argument, ignoring case.
    pub(crate) fn matches(self, argument: &str) -> bool {
        let definition = self.definition();
        let mut spellings =
            std::iter::once(definition.flag).chain(definition.aliases.iter().copied());
        match definition.arity {
            Arity::UnaryInlineKeyOrKeyValue => {
                spellings.any(|spelling| argument.starts_with(spelling))
            }
            Arity::UnaryOptionalColon => spellings.any(|spelling| {
                argument
                    .get(..spelling.len())
                    .is_some_and(|start| start.eq_ignore_ascii_case(spelling))
            }),
            _ => spellings.any(|spelling| argument.eq_ignore_ascii_case(spelling)),
        }
    }

    /// The flag an argument is: the first in PlantUML's order that matches. A lone `-P` is the pragma flag,
    /// as the help documents, although PlantUML finds `-p` (pipe) first.
    pub(crate) fn of(argument: &str) -> Option<CliFlag> {
        let mut flags = Self::VALUES.iter().copied();
        flags
            .clone()
            .find(|flag| flag.arity() == Arity::UnaryInlineKeyOrKeyValue && flag.flag() == argument)
            .or_else(|| flags.find(|flag| flag.matches(argument)))
    }

    /// The first column of the flag's help line (`getUsage`).
    pub(crate) fn usage(self) -> String {
        let definition = self.definition();
        if let Some(usage) = definition
            .doc
            .as_ref()
            .map(|doc| doc.usage)
            .filter(|usage| !usage.is_empty())
        {
            return if usage.starts_with("--") {
                format!("     {usage}")
            } else {
                format!(" {usage}")
            };
        }
        if let Some(short) = definition.aliases.iter().find(|alias| alias.len() == 2) {
            return format!(" {short}, {}", definition.flag);
        }
        format!("     {}", definition.flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_word_flags_ignore_case() {
        assert_eq!(CliFlag::of("-TSVG"), Some(CliFlag::TSvg));
        assert_eq!(CliFlag::of("-pipeNoStderr"), Some(CliFlag::Pipenostderr));
        assert_eq!(CliFlag::of("--svg"), Some(CliFlag::TSvg));
        assert_eq!(CliFlag::of("-tsvgx"), None);
    }

    #[test]
    fn key_flags_are_case_sensitive_prefixes() {
        assert_eq!(CliFlag::of("-DNAME=value"), Some(CliFlag::Define));
        assert_eq!(CliFlag::of("-Ipath"), Some(CliFlag::Include));
        assert_eq!(CliFlag::of("-Pteoz=true"), Some(CliFlag::Pragma));
        assert_eq!(CliFlag::of("-P"), Some(CliFlag::Pragma));
        assert_eq!(CliFlag::of("-p"), Some(CliFlag::Pipe));
        assert_eq!(CliFlag::of("-debugsvek"), Some(CliFlag::DebugSvek));
        // -S comes before --svg, so PlantUML reads -SVG as a skin parameter.
        assert_eq!(CliFlag::of("-SVG"), Some(CliFlag::Skinparam));
    }

    #[test]
    fn optional_colon_flags_are_case_insensitive_prefixes() {
        assert_eq!(CliFlag::of("-stdrpt:1"), Some(CliFlag::Stdrpt));
        assert_eq!(CliFlag::of("--HTTP-Server:8080"), Some(CliFlag::Picoweb));
    }

    #[test]
    fn usage_shows_a_short_alias_or_indents_long_flags() {
        assert_eq!(CliFlag::Help.usage(), " -h, --help");
        assert_eq!(CliFlag::Version.usage(), "     --version");
        assert_eq!(CliFlag::Theme.usage(), "     --theme <name>");
        assert_eq!(CliFlag::Define.usage(), " -D, --define <VAR>=<value>");
    }
}
