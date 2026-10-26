//! The plugin's own text templates (`emit/templates/`): files whose text is fixed but for names,
//! the flavor and a few generated slots.
//!
//! The first line of a template says where its text comes from and is not emitted. Then, in
//! order:
//! - whole lines `{{#java}}` / `{{#kotlin}}` ... `{{/java}}` / `{{/kotlin}}` keep the lines
//!   between them for that flavor only;
//! - a whole line `{{@NAME}}` is replaced by the slot `NAME`'s lines (none when it is empty);
//! - `{{java:TEXT}}` / `{{kotlin:TEXT}}` keep `TEXT` for that flavor only;
//! - `{{NAME}}` (upper-case letters, digits and `_`) is replaced by the value `NAME`; other
//!   `{{` text, such as GitHub Actions' `${{ github.ref }}`, is kept as written.

use crate::meta::Flavor;

/// Renders `template` for `flavor`.
///
/// # Panics
///
/// On an unknown value or slot, or an unclosed token: templates are part of the crate, so that is
/// a bug the tests catch.
#[must_use]
pub fn render(template: &str, flavor: Flavor, values: &[(&str, &str)], slots: &[(&str, &str)]) -> String {
    let java = flavor.is_java();
    let mut out = String::new();
    let mut keep: Vec<bool> = Vec::new();
    // `split('\n')`, not `lines()`: a template's `\r\n` endings (`gradlew.bat`) are its text.
    let body = template.strip_suffix('\n').unwrap_or(template);
    for line in body.split('\n').skip(1) {
        match line.trim() {
            "{{#java}}" => {
                keep.push(java);
                continue;
            }
            "{{#kotlin}}" => {
                keep.push(!java);
                continue;
            }
            "{{/java}}" | "{{/kotlin}}" => {
                keep.pop();
                continue;
            }
            _ => {}
        }
        if keep.iter().any(|k| !k) {
            continue;
        }
        if let Some(name) = line.trim().strip_prefix("{{@").and_then(|r| r.strip_suffix("}}")) {
            let text = slots
                .iter()
                .find(|(n, _)| *n == name)
                .map_or_else(|| panic!("template slot {name} has no text"), |(_, t)| *t);
            if !text.is_empty() {
                out.push_str(text);
                if !text.ends_with('\n') {
                    out.push('\n');
                }
            }
            continue;
        }
        out.push_str(&inline(line, java, values));
        out.push('\n');
    }
    if !template.ends_with('\n') {
        out.pop();
    }
    out
}

fn inline(line: &str, java: bool, values: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let close = after
            .find("}}")
            .unwrap_or_else(|| panic!("unclosed template token in {line:?}"));
        let token = &after[..close];
        let is_name = token.starts_with(|c: char| c.is_ascii_uppercase())
            && token
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if !is_name && !token.starts_with("java:") && !token.starts_with("kotlin:") {
            // Not a token: text such as a GitHub Actions `${{ github.ref }}` expression.
            out.push_str("{{");
            rest = after;
            continue;
        }
        if let Some(text) = token.strip_prefix("java:") {
            if java {
                out.push_str(text);
            }
        } else if let Some(text) = token.strip_prefix("kotlin:") {
            if !java {
                out.push_str(text);
            }
        } else {
            let value = values
                .iter()
                .find(|(n, _)| *n == token)
                .map_or_else(|| panic!("template value {token} is not given"), |(_, v)| *v);
            out.push_str(value);
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_blocks_slots_and_tokens() {
        let t = "// provenance\n{{#java}}\nimport java.util.Optional\n{{/java}}\n{{java:@JvmStatic }}fun {{NAME}}()\n{{@BODY}}\n{{@NONE}}\nend\n";
        let values = [("NAME", "x")];
        let slots = [("BODY", "a\nb"), ("NONE", "")];
        assert_eq!(
            render(t, Flavor::Java, &values, &slots),
            "import java.util.Optional\n@JvmStatic fun x()\na\nb\nend\n"
        );
        assert_eq!(render(t, Flavor::Kotlin, &values, &slots), "fun x()\na\nb\nend\n");
        assert_eq!(
            render("//\nif: ${{ github.ref }} {{NAME}}\n", Flavor::Java, &values, &[]),
            "if: ${{ github.ref }} x\n"
        );
    }
}
