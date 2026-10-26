//! The project files: the solution, the two projects and `Directory.Build.props`, `scripts/`,
//! and the tool configurations (`.editorconfig`, `.csharpierignore`, the csharpier tool
//! manifest, VS Code and devcontainer settings). The texts are imagekit-dotnet's with its names
//! as tokens (`src/emit/project/*.txt`).

use crate::emit::{Gen, template};

fn fill(g: &Gen<'_>, text: &str) -> String {
    let t = g.target;
    template(text)
        .replace("__REPO__", &t.repo)
        .replace("__DOCS__", &t.docs)
        .replace("__YEAR__", &t.year.to_string())
        .replace("__TITLE__", &t.title)
        .replace("__NS__", &t.ns)
        .replace("__VERSION__", &t.version)
        .replace("__LICENSE__", &t.license)
}

/// Every project file, by path from the SDK root.
#[must_use]
pub fn project_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let ns = &g.target.ns;
    let mut out = vec![
        // Visual Studio writes solution files with a byte order mark.
        (
            format!("{ns}.sln"),
            format!("\u{feff}{}", fill(g, include_str!("project/sln.txt"))),
        ),
        (
            format!("src/{ns}/{ns}.csproj"),
            fill(g, include_str!("project/csproj.txt")),
        ),
        (
            format!("src/{ns}.Tests/{ns}.Tests.csproj"),
            fill(g, include_str!("project/tests-csproj.txt")),
        ),
        (
            "src/Directory.Build.props".to_owned(),
            fill(g, include_str!("project/Directory.Build.props.txt")),
        ),
        // The SDK's C# files are CRLF, and so is the file that says so.
        (
            ".editorconfig".to_owned(),
            fill(g, include_str!("project/editorconfig.txt")).replace('\n', "\r\n"),
        ),
        (".gitignore".to_owned(), fill(g, include_str!("project/gitignore.txt"))),
        (
            ".csharpierignore".to_owned(),
            fill(g, include_str!("project/csharpierignore.txt")),
        ),
        (
            ".config/dotnet-tools.json".to_owned(),
            fill(g, include_str!("project/dotnet-tools.json.txt")),
        ),
        (
            ".vscode/extensions.json".to_owned(),
            fill(g, include_str!("project/vscode-extensions.json.txt")),
        ),
        (
            ".devcontainer/devcontainer.json".to_owned(),
            fill(g, include_str!("project/devcontainer.json.txt")),
        ),
        ("examples/.keep".to_owned(), String::new()),
    ];
    for (name, text) in [
        ("bootstrap", include_str!("project/scripts-bootstrap.txt")),
        ("build", include_str!("project/scripts-build.txt")),
        ("format", include_str!("project/scripts-format.txt")),
        ("lint", include_str!("project/scripts-lint.txt")),
        ("test", include_str!("project/scripts-test.txt")),
    ] {
        out.push((format!("scripts/{name}"), fill(g, text)));
    }
    out
}
