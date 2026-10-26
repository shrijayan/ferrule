//! The package's project files: `composer.json`, the tool configurations (php-cs-fixer's, which
//! `scripts/format` formats the package with, so it is part of the output's layout; `PHPStan`'s,
//! `PHPUnit`'s, phpactor's) and the scripts. All are imagekit-php's, with the package's names (see
//! NOTICE).

use super::Gen;
use crate::naming::{Casings, class_name, split_words};

/// Templates by published path; the first line of each says where it comes from and is not
/// emitted.
const TEMPLATES: [(&str, &str); 12] = [
    (
        ".php-cs-fixer.dist.php",
        include_str!("project/php-cs-fixer.dist.php.txt"),
    ),
    ("phpstan.dist.neon", include_str!("project/phpstan.dist.neon.txt")),
    ("phpunit.xml.dist", include_str!("project/phpunit.xml.dist.txt")),
    (".phpactor.json", include_str!("project/phpactor.json.txt")),
    (".gitignore", include_str!("project/gitignore.txt")),
    (".gitattributes", include_str!("project/gitattributes.txt")),
    ("scripts/bootstrap", include_str!("project/scripts-bootstrap.txt")),
    ("scripts/clean", include_str!("project/scripts-clean.txt")),
    ("scripts/format", include_str!("project/scripts-format.txt")),
    ("scripts/lint", include_str!("project/scripts-lint.txt")),
    ("scripts/test", include_str!("project/scripts-test.txt")),
    ("composer.json", include_str!("project/composer.json.txt")),
];

/// A template without its first line.
#[must_use]
pub fn template(text: &str) -> &str {
    text.split_once('\n').map_or("", |(_, rest)| rest)
}

/// The API's name in words as Stainless titles it: `ImageKit` -> `Image Kit`.
#[must_use]
pub fn title(g: &Gen<'_>) -> String {
    let name = g
        .client
        .settings
        .as_ref()
        .map_or_else(|| g.ns.clone(), |s| s.name.written.clone());
    split_words(&name)
        .iter()
        .map(|w| class_name(&[w], Casings(&g.client.casings)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The project files.
#[must_use]
pub fn project_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let title = title(g);
    let ns_json = g.ns.replace('\\', "\\\\");
    TEMPLATES
        .iter()
        .map(|(path, text)| {
            let text = template(text)
                .replace("__NS_JSON__", &ns_json)
                .replace("__TITLE__", &title)
                .replace("__COMPOSER__", &g.target.composer_name)
                // The webhook helpers' dependency; webhooks are not in the model (gap list).
                .replace("__WEBHOOKS__", "");
            ((*path).to_owned(), text)
        })
        .collect()
}
