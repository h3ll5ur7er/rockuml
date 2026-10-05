use std::process::Command;

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
fn version_flag_reports_the_plantuml_release_rockuml_is_compatible_with() {
    let output = rockuml().arg("--version").output().unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "rockuml {} (PlantUML 1.2026.8 compatible)\n",
            env!("CARGO_PKG_VERSION")
        )
    );
}
