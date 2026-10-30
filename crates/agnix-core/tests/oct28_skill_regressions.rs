//! Reproductions from agent-sh/agnix#1738 through the real validation pipeline.
use agnix_core::{LintConfig, validate_project};
use std::{fs, path::Path};

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const SKILL: &str =
    "---\nname: demo\ndescription: Lints demo files. Use when asked to lint a demo.\n---\n";

#[test]
fn as_015_ignored_payloads_do_not_count_toward_upload_size() {
    for (payload, ignore_path, ignore) in [
        (
            "node_modules/pkg/blob.bin",
            "skills/demo/.gitignore",
            "node_modules/\n",
        ),
        ("cache/blob.bin", "skills/demo/.gitignore", "cache/\n"),
        ("cache/blob.bin", "skills/demo/.gitignore", "/cache/\n"),
        ("cache/blob.bin", "skills/demo/.ignore", "/cache/\n"),
        ("cache/blob.bin", ".gitignore", "skills/demo/cache/\n"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        write(temp.path(), "skills/demo/SKILL.md", SKILL);
        write(temp.path(), ignore_path, ignore);
        let payload_path = temp.path().join("skills/demo").join(payload);
        fs::create_dir_all(payload_path.parent().unwrap()).unwrap();
        fs::File::create(&payload_path)
            .unwrap()
            .set_len(32_000_000)
            .unwrap();
        let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
        assert!(
            result.diagnostics.iter().all(|d| d.rule != "AS-015"),
            "{payload}: {:?}",
            result.diagnostics
        );
        // A real bundled payload still exceeds the limit.
        fs::remove_file(temp.path().join(ignore_path)).unwrap();
        fs::rename(&payload_path, temp.path().join("skills/demo/bundled.bin")).unwrap();
        let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "AS-015"),
            "{:?}",
            result.diagnostics
        );
    }
}

#[test]
fn as_024_major_pin_advice_explicitly_requires_an_exact_version() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "SKILL.md",
        &format!("{SKILL}\nRun `npx eslint@9 --fix .` and `npx eslint@9.0.0 --fix .`.\n"),
    );
    let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
    let hits: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AS-024")
        .collect();
    assert_eq!(hits.len(), 1, "{:?}", result.diagnostics);
    assert!(hits[0].message.contains("exact version"), "{:?}", hits[0]);
}

#[test]
fn cc_sk_027_nested_command_migration_explains_the_name_change() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".claude/commands/frontend/component.md",
        "Render a component.\n",
    );
    let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
    let hit = result
        .diagnostics
        .iter()
        .find(|d| d.rule == "CC-SK-027")
        .unwrap();
    let suggestion = hit.suggestion.as_deref().unwrap();
    assert!(suggestion.contains("/frontend:component"), "{suggestion}");
    assert!(suggestion.contains("changes"), "{suggestion}");
    assert!(!suggestion.contains("keeps the same"), "{suggestion}");
}

#[test]
fn cc_sk_028_control_characters_are_scoped_to_injected_commands() {
    let temp = tempfile::tempdir().unwrap();
    for (body, expected) in [
        ("Now: !`date`\nA form feed\u{c} in prose.\n", false),
        ("Now: !`printf '\u{c}'`\n", true),
        ("```!\nprintf '\u{c}'\n```\n", true),
        ("```!\ndate\n```\nA form feed\u{c} in prose.\n", false),
        ("````markdown\n```!\nprintf '\u{c}'\n```\n````\n", false),
        ("    ```!\n    printf '\u{c}'\n    ```\n", false),
        (
            "``text\n    ```!\n    printf '\u{c}'\n    ```\ntext``\n",
            false,
        ),
        ("```!\nprintf '\u{c}'\n```\t\n", true),
        ("```!\nprintf '\u{c}'\n", false),
        ("`!` is prose.\n```bash\nprintf '\u{c}'\n```\n", false),
    ] {
        write(
            temp.path(),
            ".claude/skills/demo/SKILL.md",
            &format!("{SKILL}\n{body}"),
        );
        let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
        let hits: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-028" && d.message.contains("control character"))
            .collect();
        assert_eq!(
            !hits.is_empty(),
            expected,
            "{body:?}: {:?}",
            result.diagnostics
        );
        if expected {
            assert_eq!(hits[0].line, if body.starts_with("```!") { 7 } else { 6 });
        }
    }
}

#[test]
fn cc_sk_028_powershell_format_characters_only_affect_commands() {
    let temp = tempfile::tempdir().unwrap();
    for character in ['\u{feff}', '\u{180e}'] {
        for (body, executable) in [
            (format!("Now: !`echo '{character}'`\n"), true),
            (format!("```!\necho '{character}'\n```\n"), true),
            (format!("Now: !`date`\nProse {character} here.\n"), false),
        ] {
            for shell in ["bash", "powershell"] {
                let content =
                    SKILL.replace("---\nname:", &format!("---\nshell: {shell}\nname:")) + &body;
                write(temp.path(), ".claude/skills/demo/SKILL.md", &content);
                let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
                assert_eq!(
                    result.diagnostics.iter().any(|d| d.rule == "CC-SK-028"),
                    executable && shell == "powershell",
                    "{shell}, {body:?}: {:?}",
                    result.diagnostics
                );
            }
        }
    }
}

#[test]
fn as_015_matches_parent_and_global_ignore_scope() {
    const CHILD_ROOT: &str = "AGNIX_OCT28_IGNORE_CHILD_ROOT";
    if let Some(base) = std::env::var_os(CHILD_ROOT) {
        let base = std::path::PathBuf::from(base);
        let root = base.join("repo");
        fs::create_dir_all(root.join(".git/info")).unwrap();
        write(&root, "skills/demo/SKILL.md", SKILL);
        for name in ["parent.bin", "global.bin", "local.bin", "keep.bin"] {
            fs::File::create(root.join("skills/demo").join(name))
                .unwrap()
                .set_len(32_000_000)
                .unwrap();
        }
        write(
            &base,
            ".ignore",
            "repo/skills/demo/parent.bin\nrepo/skills/demo/keep.bin\n",
        );
        // Child negation overrides the ancestor .ignore.
        write(&root, ".ignore", "!skills/demo/keep.bin\n");
        write(&root, ".git/info/exclude", "skills/demo/local.bin\n");
        let global_ignore = ignore::gitignore::gitconfig_excludes_path()
            .expect("the fixture config must select a global ignore file");
        assert!(
            global_ignore.is_file(),
            "fixture global ignore path must exist: {global_ignore:?}"
        );
        let result = validate_project(&root, &LintConfig::default()).unwrap();
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "AS-015"),
            "visible/local payloads count: {:?}",
            result.diagnostics
        );
        fs::remove_file(root.join("skills/demo/keep.bin")).unwrap();
        let result = validate_project(&root, &LintConfig::default()).unwrap();
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "AS-015"),
            ".git/info/exclude must not hide the local payload: {:?}",
            result.diagnostics
        );
        fs::remove_file(root.join("skills/demo/local.bin")).unwrap();
        fs::File::create(root.join("skills/demo/keep.bin"))
            .unwrap()
            .set_len(32_000_000)
            .unwrap();
        let result = validate_project(&root, &LintConfig::default()).unwrap();
        assert!(
            result.diagnostics.iter().any(|d| d.rule == "AS-015"),
            "child .ignore negation must keep its payload visible: {:?}",
            result.diagnostics
        );
        fs::remove_file(root.join("skills/demo/keep.bin")).unwrap();
        fs::remove_file(root.join("skills/demo/global.bin")).unwrap();
        let result = validate_project(&root, &LintConfig::default()).unwrap();
        assert!(
            result.diagnostics.iter().all(|d| d.rule != "AS-015"),
            "ancestor .ignore payload does not count: {:?}",
            result.diagnostics
        );
        fs::remove_file(root.join("skills/demo/parent.bin")).unwrap();
        fs::File::create(root.join("skills/demo/global.bin"))
            .unwrap()
            .set_len(32_000_000)
            .unwrap();
        let result = validate_project(&root, &LintConfig::default()).unwrap();
        assert!(
            result.diagnostics.iter().all(|d| d.rule != "AS-015"),
            "global ignore payload does not count: {:?}",
            result.diagnostics
        );
        return;
    }
    // Isolate Git's home/config discovery without mutating the test process
    // environment or the user's global exclusions.
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let excludes = home.join("global-ignore");
    fs::write(&excludes, "global.bin\n").unwrap();
    fs::write(
        home.join(".gitconfig"),
        format!(
            "[core]\nexcludesFile = {}\n",
            excludes.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "as_015_matches_parent_and_global_ignore_scope",
            "--nocapture",
        ])
        .env(CHILD_ROOT, temp.path())
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("GIT_CONFIG_GLOBAL", home.join(".gitconfig"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
