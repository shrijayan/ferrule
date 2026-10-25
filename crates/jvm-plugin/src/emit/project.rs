//! The Gradle and project files (`emit/templates/project/`): build scripts, the Gradle wrapper's
//! text files, `buildSrc`, scripts, workflows and the policy files, with the SDK's names.
//!
//! Not emitted: `gradle/wrapper/gradle-wrapper.jar` (binary), `README.md`, `.stats.yml` and
//! `CHANGELOG.md` (release-please's).

use crate::emit::Gen;
use crate::emit::tmpl::render;
use crate::meta::Flavor;

/// A project file: its path (with `ART` for the artifact and `ORG` for the organization), its
/// template, and the flavor it exists for (`None`: both).
struct Template {
    path: &'static str,
    text: &'static str,
    only: Option<Flavor>,
}

macro_rules! template {
    ($path:literal, $file:literal) => {
        Template {
            path: $path,
            text: include_str!(concat!("templates/project/", $file)),
            only: None,
        }
    };
    ($path:literal, $file:literal, $only:expr) => {
        Template {
            path: $path,
            text: include_str!(concat!("templates/project/", $file)),
            only: Some($only),
        }
    };
}

const TEMPLATES: &[Template] = &[
    template!(".devcontainer/Dockerfile", "_devcontainer__Dockerfile.tmpl"),
    template!(
        ".devcontainer/devcontainer.json",
        "_devcontainer__devcontainer_json.tmpl"
    ),
    template!(".gitattributes", "_gitattributes.tmpl", Flavor::Java),
    template!(".github/workflows/ci.yml", "_github__workflows__ci_yml.tmpl"),
    template!(
        ".github/workflows/publish-sonatype.yml",
        "_github__workflows__publish-sonatype_yml.tmpl"
    ),
    template!(
        ".github/workflows/release-doctor.yml",
        "_github__workflows__release-doctor_yml.tmpl"
    ),
    template!(".gitignore", "_gitignore.tmpl"),
    template!(".release-please-manifest.json", "_release-please-manifest_json.tmpl"),
    template!(
        "ART-client-okhttp/build.gradle.kts",
        "ART-client-okhttp__build_gradle_kts.tmpl"
    ),
    template!("ART-core/build.gradle.kts", "ART-core__build_gradle_kts.tmpl"),
    template!("ART-example/build.gradle.kts", "ART-example__build_gradle_kts.tmpl"),
    template!("ART-lib/.keep", "ART-lib___keep.tmpl"),
    template!(
        "ART-proguard-test/build.gradle.kts",
        "ART-proguard-test__build_gradle_kts.tmpl"
    ),
    template!("ART-proguard-test/test.pro", "ART-proguard-test__test_pro.tmpl"),
    template!("ART/build.gradle.kts", "ART__build_gradle_kts.tmpl"),
    template!("LICENSE", "LICENSE.tmpl"),
    template!("SECURITY.md", "SECURITY_md.tmpl"),
    template!("bin/check-release-environment", "bin__check-release-environment.tmpl"),
    template!("build.gradle.kts", "build_gradle_kts.tmpl"),
    template!("buildSrc/build.gradle.kts", "buildSrc__build_gradle_kts.tmpl"),
    template!(
        "buildSrc/src/main/kotlin/ORG.java.gradle.kts",
        "buildSrc__src__main__kotlin__ORG_java_gradle_kts.tmpl"
    ),
    template!(
        "buildSrc/src/main/kotlin/ORG.kotlin.gradle.kts",
        "buildSrc__src__main__kotlin__ORG_kotlin_gradle_kts.tmpl"
    ),
    template!(
        "buildSrc/src/main/kotlin/ORG.publish.gradle.kts",
        "buildSrc__src__main__kotlin__ORG_publish_gradle_kts.tmpl"
    ),
    template!("examples/.keep", "examples___keep.tmpl", Flavor::Java),
    template!("gradle.properties", "gradle_properties.tmpl"),
    template!(
        "gradle/wrapper/gradle-wrapper.properties",
        "gradle__wrapper__gradle-wrapper_properties.tmpl"
    ),
    template!("gradlew", "gradlew.tmpl"),
    template!("gradlew.bat", "gradlew_bat.tmpl"),
    template!("release-please-config.json", "release-please-config_json.tmpl"),
    template!("scripts/build", "scripts__build.tmpl"),
    template!("scripts/fast-format", "scripts__fast-format.tmpl", Flavor::Java),
    template!("scripts/format", "scripts__format.tmpl"),
    template!("scripts/java-format", "scripts__java-format.tmpl", Flavor::Java),
    template!("scripts/kotlin-format", "scripts__kotlin-format.tmpl", Flavor::Java),
    template!("scripts/lint", "scripts__lint.tmpl"),
    template!("scripts/mock", "scripts__mock.tmpl"),
    template!("scripts/test", "scripts__test.tmpl"),
    template!("settings.gradle.kts", "settings_gradle_kts.tmpl"),
];

/// The paths of the project files that are executable scripts.
pub const EXECUTABLES: &[&str] = &[
    "bin/check-release-environment",
    "gradlew",
    "scripts/build",
    "scripts/fast-format",
    "scripts/format",
    "scripts/java-format",
    "scripts/kotlin-format",
    "scripts/lint",
    "scripts/mock",
    "scripts/test",
];

/// `text` wrapped at 80 columns, lines joined by a Kotlin string's `\n`.
fn wrap80(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= 80 => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines.join("\\n")
}

/// The organization's name as Gradle plugin ids spell it: `onebusaway-sdk`, `image-kit`.
#[must_use]
pub fn org_id(g: &Gen<'_>) -> String {
    let written = g
        .client
        .settings
        .as_ref()
        .map_or_else(|| g.client.package.name.clone(), |s| s.name.written.clone());
    crate::naming::split_words(&written)
        .iter()
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// Every project file of the flavor.
#[must_use]
pub fn project_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let (api_title, api_description) = g
        .client
        .description
        .split_once("\n\n")
        .map_or((g.client.description.as_str(), ""), |(t, d)| (t, d));
    let org = org_id(g);
    let env_prefix = g
        .client
        .settings
        .as_ref()
        .map_or_else(String::new, |s| s.env_prefix.clone());
    let year = g.target.year.to_string();
    let description = wrap80(api_description);
    let values = [
        ("ARTIFACT", g.target.artifact.as_str()),
        ("REPO", g.target.repo.as_str()),
        ("BRANCH", g.target.branch.as_str()),
        ("PKG", g.root()),
        ("CLIENT", g.client_name()),
        ("ENV_PREFIX", env_prefix.as_str()),
        ("ORG", org.as_str()),
        ("TITLE", g.target.title.as_str()),
        ("VERSION", g.target.version.as_str()),
        ("YEAR", year.as_str()),
        ("API_TITLE", api_title),
        ("API_DESCRIPTION", description.as_str()),
        ("DOCS_URL", g.target.docs_url.as_str()),
        ("CONTACT", g.target.contact.as_str()),
    ];
    let mut out = Vec::new();
    for t in TEMPLATES {
        if t.only.is_some_and(|f| f != g.target.flavor) {
            continue;
        }
        let path = t.path.replace("ART", &g.target.artifact).replace("ORG", &org);
        out.push((path, render(t.text, g.target.flavor, &values, &[])));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_publish_descriptions_at_80_columns() {
        assert_eq!(
            wrap80("The OneBusAway REST API. For use with servers like https://api.pugetsound.onebusaway.org"),
            "The OneBusAway REST API. For use with servers like\\nhttps://api.pugetsound.onebusaway.org"
        );
    }
}
