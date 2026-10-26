//! The template language of the bundled TypeScript and project templates: `%%NAME%%` is replaced
//! by a value, and whole lines `%%if flag%%` (or `%%if !flag%%`) / `%%else%%` / `%%endif%%`
//! keep or drop what they enclose. Templates are text copied from Apache-2.0 Stainless SDKs (see `NOTICE`).

/// Renders `template`.
///
/// # Panics
///
/// On a token without a value or an unbalanced directive: a bug in a bundled template, never in
/// user input (tests render every template).
#[must_use]
pub fn render(template: &str, vars: &[(&str, &str)], flags: &[(&str, bool)]) -> String {
    let mut out = String::with_capacity(template.len());
    // Whether each enclosing `if` keeps its lines.
    let mut stack: Vec<bool> = Vec::new();
    for line in template.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n');
        if let Some(cond) = trimmed.strip_prefix("%%if ").and_then(|r| r.strip_suffix("%%")) {
            let (negate, flag) = cond.strip_prefix('!').map_or((false, cond), |f| (true, f));
            let on = flags
                .iter()
                .find(|(f, _)| *f == flag)
                .unwrap_or_else(|| panic!("template flag {flag} has no value"))
                .1;
            stack.push(on != negate);
            continue;
        }
        if trimmed == "%%else%%" {
            let top = stack.last_mut().expect("%%else%% without %%if%%");
            *top = !*top;
            continue;
        }
        if trimmed == "%%endif%%" {
            stack.pop().expect("%%endif%% without %%if%%");
            continue;
        }
        if stack.iter().all(|&on| on) {
            out.push_str(&substitute(line, vars));
        }
    }
    assert!(stack.is_empty(), "unclosed %%if%% in a template");
    out
}

fn substitute(line: &str, vars: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("%%") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("%%")
            .unwrap_or_else(|| panic!("unterminated token in template line: {line}"));
        let name = &after[..end];
        let value = vars
            .iter()
            .find(|(k, _)| *k == name)
            .unwrap_or_else(|| panic!("template token {name} has no value"))
            .1;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_tokens_and_resolves_directives() {
        let t = "a %%X%%\n%%if on%%\nkept\n%%else%%\ndropped\n%%endif%%\n%%if off%%\nno\n%%endif%%\nend\n";
        assert_eq!(
            render(t, &[("X", "1")], &[("on", true), ("off", false)]),
            "a 1\nkept\nend\n"
        );
    }
}
