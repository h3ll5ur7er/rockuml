//! One input file: its diagram blocks, the files their images go to, and writing them (PlantUML's
//! `SourceFileReader`, `SourceFileReaderCopyCat` and `PSystemUtils`).

use std::fs;
use std::path::{Component, Path, PathBuf};

use rockuml::diagram::Diagram;
use rockuml::preproc::{PreprocessedBlock, PreprocessorEnvironment, Source};

use crate::cli_flag::CliFlag;
use crate::console::Console;
use crate::crash::{self, Unrendered};
use crate::exit_status::ExitStatus;
use crate::file_format::FileFormat;
use crate::run::Settings;
use crate::system_host;

pub(crate) struct SourceFileReader<'a> {
    file: &'a Path,
    settings: &'a Settings,
    output: OutputDirectory,
    blocks: Vec<BlockUml>,
    /// The number the next unnamed image takes.
    cpt: usize,
}

struct BlockUml {
    preprocessed: PreprocessedBlock,
    diagram: Result<Box<dyn Diagram>, Unrendered>,
}

impl BlockUml {
    fn is_error(&self) -> bool {
        match &self.diagram {
            Ok(diagram) => diagram.is_error(),
            Err(unrendered) => matches!(unrendered, Unrendered::Crashed(_)),
        }
    }

    fn error_line(&self) -> Option<i32> {
        self.diagram
            .as_ref()
            .ok()
            .and_then(|diagram| diagram.error())
            .map(|error| error.line)
    }
}

/// Where a file's images go.
enum OutputDirectory {
    Regular(PathBuf),
    /// `-o dir$`: below `dir`, along the input file's own path.
    CopyCat(PathBuf),
}

impl<'a> SourceFileReader<'a> {
    pub(crate) fn new(file: &'a Path, settings: &'a Settings) -> Result<Self, String> {
        let blocks = crash::catch(|| preprocess_file(file, settings))
            .map_err(|message| format!("{}: crashed: {message}", file.display()))??
            .into_iter()
            .map(|preprocessed| BlockUml {
                diagram: crash::render(|| rockuml::diagram::create(&preprocessed, &settings.host)),
                preprocessed,
            })
            .collect();
        let output = output_directory(file, settings.options.output_dir());
        let (OutputDirectory::Regular(directory) | OutputDirectory::CopyCat(directory)) = &output;
        let _ = fs::create_dir_all(directory);
        Ok(Self {
            file,
            settings,
            output,
            blocks,
            cpt: 0,
        })
    }

    pub(crate) fn update_status(&self, status: &ExitStatus) {
        status.goes_has_files();
        for block in &self.blocks {
            status.goes_has_blocks();
            if block.is_error() {
                status.goes_has_errors();
            }
        }
    }

    pub(crate) fn has_error(&self) -> bool {
        self.blocks.iter().any(BlockUml::is_error)
    }

    /// Writes each block's preprocessed lines in the input's charset.
    pub(crate) fn extract_preprocessing_source(&mut self, console: &mut Console) {
        for index in 0..self.blocks.len() {
            let suggested = self.suggested_file(index);
            let text: String = self.blocks[index]
                .preprocessed
                .lines()
                .flat_map(|line| [line, crate::console::LINE_SEPARATOR])
                .collect();
            let file = suggested.file(0, FileFormat::Preproc);
            if let Err(error) = fs::write(&file, self.settings.charset.encode(&text)) {
                console.error(&format!(
                    "rockuml: cannot write {}: {error}",
                    file.display()
                ));
            }
        }
    }

    /// Writes the images of every block and reports the first error line, as `manageFileInternal` does.
    pub(crate) fn generate_images(&mut self, status: &ExitStatus, console: &mut Console) {
        if self.blocks.is_empty() {
            console.error(&format!("Warning: no image in {}", self.file.display()));
            return;
        }
        for index in 0..self.blocks.len() {
            self.export_block(index, status, console);
        }
        if let Some(line) = self.blocks.iter().find_map(BlockUml::error_line) {
            console.error(&format!(
                "Error line {} in file: {}",
                line + 1,
                self.file.display()
            ));
        }
    }

    fn export_block(&mut self, index: usize, status: &ExitStatus, console: &mut Console) {
        let suggested = self.suggested_file(index);
        let format = self.settings.format;
        let options = &self.settings.options;
        let diagram = match &self.blocks[index].diagram {
            Ok(diagram) => diagram,
            Err(unrendered) => {
                unrendered.report(
                    &suggested.file(0, format).display().to_string(),
                    status,
                    console,
                );
                return;
            }
        };
        if options.is_true(CliFlag::NoErrorImage) && diagram.is_error() {
            return;
        }
        let first = suggested.file(0, format);
        if options.is_true(CliFlag::CheckMetadata)
            && format.supports_metadata()
            && is_fresh(&first, diagram.as_ref(), format)
        {
            return;
        }
        let image_format = format
            .image_format()
            .expect("drawing formats export images");
        let pages = diagram.page_count();
        for page in 0..pages {
            let file = suggested.file(page, format);
            if !can_file_be_written(&file, options.is_true(CliFlag::Overwrite), console) {
                break;
            }
            let image = crash::render(|| {
                rockuml::diagram::export_with(
                    diagram.as_ref(),
                    page,
                    image_format,
                    self.settings.options.metadata(),
                    &self.settings.fonts,
                    &self.settings.host,
                )
            });
            let content = match image {
                Ok(_) if format == FileFormat::Null => Vec::new(),
                Ok(image) => image,
                Err(unrendered) => {
                    unrendered.report(&file.display().to_string(), status, console);
                    break;
                }
            };
            if let Err(error) = fs::write(&file, content) {
                console.error(&format!(
                    "rockuml: cannot write {}: {error}",
                    file.display()
                ));
            }
        }
        if pages > 1 {
            self.cpt += pages - 1;
        }
    }

    /// The file a block's images are named after (`getSuggestedFile`): the input's name, numbered, unless
    /// the block names itself; a name ending with `/` or naming a directory puts the input's name there.
    fn suggested_file(&mut self, index: usize) -> SuggestedFile {
        let new_name = if self
            .settings
            .options
            .is_true(CliFlag::IgnoreStartumlFilename)
        {
            None
        } else {
            self.blocks[index].preprocessed.output_name()
        };
        let file_name = file_name(self.file);
        let suggested = match (&self.output, new_name) {
            (OutputDirectory::Regular(directory), None) => {
                self.numbered(directory.join(&file_name))
            }
            (OutputDirectory::Regular(directory), Some(new_name)) => {
                match dir_if_directory(&new_name, directory) {
                    Some(target) => SuggestedFile::new(target.join(&file_name), 0),
                    None => SuggestedFile::new(java_file(directory, &new_name), 0),
                }
            }
            (OutputDirectory::CopyCat(directory), new_name) => {
                self.numbered(java_file(directory, &new_name.unwrap_or(file_name)))
            }
        };
        if let Some(parent) = suggested.output_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        suggested
    }

    fn numbered(&mut self, output_file: PathBuf) -> SuggestedFile {
        let suggested = SuggestedFile::new(output_file, self.cpt);
        self.cpt += 1;
        suggested
    }
}

/// An output file name and the number its first image takes (`SuggestedFile`).
struct SuggestedFile {
    output_file: PathBuf,
    initial_cpt: usize,
}

impl SuggestedFile {
    fn new(output_file: PathBuf, initial_cpt: usize) -> Self {
        Self {
            output_file,
            initial_cpt,
        }
    }

    fn file(&self, cpt: usize, format: FileFormat) -> PathBuf {
        let name = file_name(&self.output_file);
        self.output_file
            .with_file_name(format.change_name(&name, self.initial_cpt + cpt))
    }
}

/// The blocks of a file, preprocessed with the command line's definitions and configuration.
pub(crate) fn preprocess_file(
    file: &Path,
    settings: &Settings,
) -> Result<Vec<PreprocessedBlock>, String> {
    let bytes =
        fs::read(file).map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let text = settings.charset.decode(&bytes);
    let file_name = file_name(file);
    let source = Source {
        text: &text,
        description: &file_name,
        directory: std::path::absolute(file)
            .ok()
            .and_then(|absolute| absolute.parent().map(Path::to_path_buf))
            .unwrap_or_default(),
        environment: environment_of(file, &file_name, settings),
    };
    Ok(rockuml::preproc::preprocess(&source, &settings.host))
}

/// What `%filename()` and `%filedate()` report. `%dirpath()` stays empty: PlantUML only reveals the
/// directory under its INSECURE security profile.
fn environment_of(file: &Path, file_name: &str, settings: &Settings) -> PreprocessorEnvironment {
    let modified = fs::metadata(file)
        .and_then(|metadata| metadata.modified())
        .ok();
    PreprocessorEnvironment {
        filename: Some(file_name.to_owned()),
        filedate: modified.map(|time| {
            rockuml::preproc::java_date_string(
                system_host::millis_since_epoch(time),
                &settings.host,
            )
        }),
        defines: settings.options.defines(),
        config: settings.options.config().to_vec(),
        ..PreprocessorEnvironment::default()
    }
}

fn output_directory(file: &Path, requested: Option<&str>) -> OutputDirectory {
    let input_directory = || {
        std::path::absolute(file)
            .ok()
            .and_then(|absolute| absolute.parent().map(Path::to_path_buf))
            .unwrap_or_default()
    };
    match requested {
        None => OutputDirectory::Regular(input_directory()),
        Some(copy_cat) if copy_cat.ends_with('$') => {
            let root = std::path::absolute(&copy_cat[..copy_cat.len() - 1]).unwrap_or_default();
            OutputDirectory::CopyCat(root.join(file.parent().unwrap_or(Path::new(""))))
        }
        Some(directory) if Path::new(directory).is_absolute() => {
            OutputDirectory::Regular(PathBuf::from(directory))
        }
        Some(directory) => OutputDirectory::Regular(input_directory().join(directory)),
    }
}

/// Java's `new File(parent, child)`, which puts even an absolute `child` below `parent` (where `Path::join`
/// would replace `parent`): a block named `@startuml /x` writes into the output directory, not to `/x`.
fn java_file(parent: &Path, child: &str) -> PathBuf {
    let relative: PathBuf = Path::new(child)
        .components()
        .filter(|component| !matches!(component, Component::Prefix(_) | Component::RootDir))
        .collect();
    parent.join(relative)
}

/// The directory a block name such as `@startuml out/` names (`getDirIfDirectory`).
fn dir_if_directory(new_name: &str, output_directory: &Path) -> Option<PathBuf> {
    if let Some(name) = new_name.strip_suffix(['/', '\\']) {
        let directory = output_directory.join(name);
        if !directory.exists() {
            let _ = fs::create_dir_all(&directory);
        } else if !directory.is_dir() {
            return None;
        }
        return Some(directory);
    }
    let directory = output_directory.join(new_name);
    directory.is_dir().then_some(directory)
}

/// A read-only file is replaced only with `--overwrite` (`canFileBeWritten`).
fn can_file_be_written(file: &Path, overwrite: bool, console: &mut Console) -> bool {
    let Ok(metadata) = fs::metadata(file) else {
        return true;
    };
    if !metadata.permissions().readonly() {
        return true;
    }
    if overwrite {
        let mut permissions = metadata.permissions();
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "--overwrite asks for it"
        )]
        permissions.set_readonly(false);
        let _ = fs::set_permissions(file, permissions);
        let _ = fs::remove_file(file);
        return true;
    }
    let absolute = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    console.error(&format!("Cannot write to file {}", absolute.display()));
    false
}

/// Whether the existing image was made from the same source (`equalsMetadata`).
fn is_fresh(file: &Path, diagram: &dyn Diagram, format: FileFormat) -> bool {
    let Ok(existing) = fs::read(file) else {
        return false;
    };
    let metadata = diagram.source().metadata();
    if format == FileFormat::Png {
        return rockuml::metadata::from_png(&existing).as_deref() == Some(metadata.as_str());
    }
    let svg = String::from_utf8_lossy(&existing);
    let signature = rockuml::url_code::encode(&metadata);
    if let Some(start) = svg.rfind("<?plantuml-src ") {
        return svg[start + "<?plantuml-src ".len()..].starts_with(&format!("{signature}?>"));
    }
    svg.rfind("<!--SRC=[")
        .is_some_and(|start| svg[start + "<!--SRC=[".len()..].starts_with(&format!("{signature}]")))
}

pub(crate) fn file_name(file: &Path) -> String {
    file.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_names_stay_below_the_output_directory() {
        let out = Path::new("out");
        assert_eq!(java_file(out, "a/b"), Path::new("out/a/b"));
        assert_eq!(java_file(out, "/etc/x"), Path::new("out/etc/x"));
        if cfg!(windows) {
            assert_eq!(java_file(out, r"C:\Windows\x"), Path::new(r"out\Windows\x"));
        }
    }
}
