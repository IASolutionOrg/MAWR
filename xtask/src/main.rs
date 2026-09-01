use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const EXCLUDED_DIRECTORIES: &[&str] = &[".git", "target"];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next();

    if arguments.next().is_some() {
        return Err("expected exactly one command; run `cargo xtask help`".to_owned());
    }

    match command.as_deref() {
        Some("verify") => verify(),
        Some("docs") => check_documentation(workspace_root()),
        Some("help" | "--help" | "-h") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`; run `cargo xtask help`")),
    }
}

fn print_help() {
    println!("MAWR repository tasks");
    println!();
    println!("Usage: cargo xtask <command>");
    println!();
    println!("Commands:");
    println!("  verify  Run every implemented deterministic repository check");
    println!("  docs    Validate local links in Markdown documentation");
    println!("  help    Print this help");
}

fn verify() -> Result<(), String> {
    let root = workspace_root();

    run_cargo(
        &root,
        "workspace check",
        &["check", "--locked", "--workspace", "--all-targets"],
    )?;
    println!("==> core dependency boundary");
    check_core_dependency_boundary(&root)?;
    println!("==> native static engine boundary");
    check_native_static_boundary(&root)?;
    run_cargo(&root, "format check", &["fmt", "--all", "--check"])?;
    run_cargo(
        &root,
        "Clippy",
        &[
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_cargo(
        &root,
        "tests",
        &["test", "--locked", "--workspace", "--all-features"],
    )?;

    println!("==> documentation links");
    check_documentation(root)?;
    println!("all implemented checks passed");

    Ok(())
}

fn check_core_dependency_boundary(root: &Path) -> Result<(), String> {
    let arguments = [
        "tree",
        "--locked",
        "--package",
        "mawr-core",
        "--edges",
        "all",
        "--prefix",
        "none",
    ];
    let output = Command::new("cargo")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|error| format!("could not inspect the core dependency graph: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "`cargo {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let graph = String::from_utf8(output.stdout)
        .map_err(|_| "core dependency graph output was not UTF-8".to_owned())?;
    let nodes = graph.lines().filter(|line| !line.trim().is_empty()).count();
    if nodes != 1 || !graph.starts_with("mawr-core v") {
        return Err(format!(
            "mawr-core must have no normal, dev, or build dependencies; graph was:\n{graph}"
        ));
    }

    println!("mawr-core has no external dependencies");
    Ok(())
}

fn check_native_static_boundary(root: &Path) -> Result<(), String> {
    let arguments = [
        "tree",
        "--locked",
        "--package",
        "mawr-native-static",
        "--edges",
        "features",
        "--prefix",
        "none",
    ];
    let output = Command::new("cargo")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|error| {
            format!("could not inspect the native engine dependency graph: {error}")
        })?;
    if !output.status.success() {
        return Err(format!(
            "`cargo {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let graph = String::from_utf8(output.stdout)
        .map_err(|_| "native engine dependency graph output was not UTF-8".to_owned())?;
    let lowercase_graph = graph.to_ascii_lowercase();
    let forbidden_graph_entries = [
        "chromiumoxide v",
        "headless_chrome v",
        "fantoccini v",
        "playwright v",
        "thirtyfour v",
        "obscura v",
        "electron v",
        "reqwest feature \"blocking\"",
        "reqwest feature \"default\"",
        "reqwest feature \"http2\"",
        "reqwest feature \"system-proxy\"",
    ];
    if let Some(entry) = forbidden_graph_entries
        .iter()
        .find(|entry| lowercase_graph.contains(**entry))
    {
        return Err(format!(
            "native static engine dependency graph contains forbidden entry `{entry}`"
        ));
    }
    if !graph.lines().any(|line| line.starts_with("mawr-core v")) {
        return Err("native static engine must depend inward on mawr-core".to_owned());
    }

    let source_root = root.join("crates").join("mawr-native-static").join("src");
    let mut source_files = Vec::new();
    collect_files_with_extension(&source_root, OsStr::new("rs"), &mut source_files)?;
    let forbidden_process_apis = [
        "std::process",
        "tokio::process",
        "process::Command",
        "Command::new",
    ];
    for file in source_files {
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("could not read {}: {error}", file.display()))?;
        if let Some(api) = forbidden_process_apis
            .iter()
            .find(|api| source.contains(**api))
        {
            return Err(format!(
                "native static engine source {} contains prohibited process API `{api}`",
                file.display()
            ));
        }
    }
    println!("native static engine has no proxy, browser, or subprocess fallback");
    Ok(())
}

fn collect_files_with_extension(
    directory: &Path,
    extension: &OsStr,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "could not read an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if file_type.is_dir() {
            collect_files_with_extension(&path, extension, files)?;
        } else if file_type.is_file() && path.extension() == Some(extension) {
            files.push(path);
        }
    }
    Ok(())
}

fn run_cargo(root: &Path, label: &str, arguments: &[&str]) -> Result<(), String> {
    println!("==> {label}");
    let status = Command::new("cargo")
        .args(arguments)
        .current_dir(root)
        .status()
        .map_err(|error| format!("could not start `cargo {}`: {error}", arguments.join(" ")))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`cargo {}` failed with {status}",
            arguments.join(" ")
        ))
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be a direct child of the workspace root")
        .to_path_buf()
}

fn check_documentation(root: PathBuf) -> Result<(), String> {
    let mut markdown_files = Vec::new();
    collect_markdown_files(&root, &mut markdown_files)?;
    markdown_files.sort();

    let mut failures = Vec::new();
    for file in &markdown_files {
        check_file_links(file, &mut failures)?;
    }

    if failures.is_empty() {
        println!(
            "validated local links in {} Markdown files",
            markdown_files.len()
        );
        Ok(())
    } else {
        failures.sort();
        Err(format!(
            "documentation link validation failed:\n{}",
            failures.join("\n")
        ))
    }
}

fn collect_markdown_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "could not read an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;

        if file_type.is_dir() {
            if !is_excluded_directory(&path) {
                collect_markdown_files(&path, files)?;
            }
        } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_excluded_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| EXCLUDED_DIRECTORIES.contains(&name))
}

fn check_file_links(file: &Path, failures: &mut Vec<String>) -> Result<(), String> {
    let contents = fs::read_to_string(file)
        .map_err(|error| format!("could not read {}: {error}", file.display()))?;

    for (line_index, line) in contents.lines().enumerate() {
        for target in markdown_link_targets(line) {
            if should_skip_target(target) {
                continue;
            }

            let target = target
                .split('#')
                .next()
                .unwrap_or_default()
                .trim_matches(['<', '>']);
            if target.is_empty() {
                continue;
            }

            let resolved = file
                .parent()
                .expect("a Markdown file always has a parent")
                .join(target);
            if !resolved.exists() {
                failures.push(format!(
                    "{}:{}: missing local target `{target}`",
                    file.display(),
                    line_index + 1
                ));
            }
        }
    }

    Ok(())
}

fn markdown_link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut remainder = line;

    while let Some(start) = remainder.find("](") {
        remainder = &remainder[start + 2..];
        let Some(end) = remainder.find(')') else {
            break;
        };
        targets.push(&remainder[..end]);
        remainder = &remainder[end + 1..];
    }

    targets
}

fn should_skip_target(target: &str) -> bool {
    let target = target.trim();
    target.is_empty()
        || target.starts_with('#')
        || target.starts_with("https://")
        || target.starts_with("http://")
        || target.starts_with("mailto:")
        || target.starts_with("data:")
}

#[cfg(test)]
mod tests {
    use super::{markdown_link_targets, should_skip_target};

    #[test]
    fn extracts_multiple_markdown_targets() {
        assert_eq!(
            markdown_link_targets("[one](docs/ONE.md) and [two](TWO.md#part)"),
            ["docs/ONE.md", "TWO.md#part"]
        );
    }

    #[test]
    fn ignores_images_and_labels_but_returns_their_targets() {
        assert_eq!(
            markdown_link_targets("![diagram](image.png)"),
            ["image.png"]
        );
    }

    #[test]
    fn classifies_non_file_targets() {
        assert!(should_skip_target("https://example.com"));
        assert!(should_skip_target("#section"));
        assert!(!should_skip_target("../README.md"));
    }
}
