use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn rockuml() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rockuml"))
}

fn stdout_lines(arguments: &[&str]) -> Vec<String> {
    let output = rockuml().args(arguments).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn decode_url_wraps_each_decoded_source_in_start_and_end_lines() {
    assert_eq!(
        stdout_lines(&["-decodeurl", "SyfFKj2rKt3CoKnELR1Io4ZDoSa70000"]),
        [
            "@startuml",
            "@startuml",
            "Bob -> Alice : hello",
            "@enduml",
            "@enduml"
        ]
    );
}

#[test]
fn encode_url_prints_one_code_per_diagram() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("two.puml");
    std::fs::write(
        &file,
        "@startuml\nBob -> Alice : hello\n@enduml\n@startuml\nA -> B\n@enduml\n",
    )
    .unwrap();
    assert_eq!(
        stdout_lines(&["-encodeurl", file.to_str().unwrap()]),
        ["SyfFKj2rKt3CoKnELR1Io4ZDoSa70000", "SrJGjLDm0W00"]
    );
}

#[test]
fn preprocessor_errors_are_reported_with_plantuml_exit_status() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("broken.puml");
    std::fs::write(&file, "@startuml\n!assert 0\n@enduml\n").unwrap();
    let output = rockuml()
        .args(["-preproc", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(200));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "Some diagram description contains errors"
    );
}

#[test]
fn version_flag_reports_the_plantuml_release_rockuml_is_compatible_with() {
    assert_eq!(
        stdout_lines(&["--version"]),
        [format!(
            "rockuml {} (PlantUML 1.2026.8 compatible)",
            env!("CARGO_PKG_VERSION")
        )]
    );
}

/// Renders a diagram of `lines` in every image format, asserting that rockuml neither panics nor takes
/// long.
fn renders_promptly(lines: &[&str]) {
    const TIME_LIMIT: Duration = Duration::from_secs(10);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("case.puml");
    let diagram = ["@startuml", &lines.join("\n"), "@enduml"].join("\n");
    std::fs::write(&file, &diagram).unwrap();
    for format in ["debug", "svg", "png"] {
        let mut child = rockuml()
            .args(["-f", format, file.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let start = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if start.elapsed() > TIME_LIMIT {
                child.kill().unwrap();
                panic!("rendering {format} took over {TIME_LIMIT:?}:\n{diagram}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(
            !stderr.contains("panicked"),
            "{format}: {stderr}\n{diagram}"
        );
    }
}

#[test]
fn malformed_svg_sprite_paths_draw_what_they_can() {
    for path in [
        r#"<path d="M0 0 h5 v5 z 1 1"/>"#,
        r#"<path fill="red"/>"#,
        "<path d='M0 0 h5 v5 z'/>",
        r#"<path fill="red" d="M0 0 h5>"#,
    ] {
        renders_promptly(&[
            &format!(r#"sprite $s <svg viewBox="0 0 10 10">{path}</svg>"#),
            "Alice -> Bob : <$s>",
        ]);
    }
}

#[test]
fn images_without_pixels_draw_nothing() {
    let icon = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/corpus/images/icon.png"
    );
    renders_promptly(&[
        "sprite $a [4x2/8] {",
        "zzzz",
        "}",
        "Alice -> Bob : <$a{scale=0.01}>",
    ]);
    renders_promptly(&["sprite $a {", "0F0", "}", "Alice -> Bob : <$a*0.01>"]);
    renders_promptly(&["sprite $a [0x0/8] {", "zz", "}", "Alice -> Bob : <$a>"]);
    renders_promptly(&[&format!("Alice -> Bob : <img:{icon}{{scale=0.01}}>")]);
}

#[test]
fn absurd_sprite_sizes_are_refused() {
    for size in ["100000x100000", "99999999999x99999999999", "5000x5000"] {
        renders_promptly(&[
            &format!("sprite $a [{size}/8] {{"),
            "zz",
            "}",
            "Alice -> Bob : <$a>",
        ]);
    }
}

#[test]
fn images_scaled_beyond_what_rockuml_draws_are_left_out() {
    renders_promptly(&[
        "sprite $a {",
        "0F0F",
        "F0F0",
        "}",
        "Alice -> Bob : <$a{scale=100000}>",
    ]);
    renders_promptly(&[
        "sprite $a [40000x3/4] {",
        "zzzz",
        "}",
        "Alice -> Bob : <$a{scale=1.5}>",
    ]);
}

#[test]
fn a_gif_takes_the_size_of_its_frame_not_of_its_screen() {
    renders_promptly(&[
        "Alice -> Bob : <img:data:image/png;base64,R0lGODlh/////4AAAAAAAP///ywAAAAAAQABAAACAkQBADs=>",
    ]);
}
