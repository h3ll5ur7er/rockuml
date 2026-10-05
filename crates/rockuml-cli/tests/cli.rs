use std::process::Command;

fn rockuml() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rockuml"))
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
