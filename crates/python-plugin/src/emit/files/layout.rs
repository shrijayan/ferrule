//! Lays out one Python statement the way ruff format splits it, for template lines whose length
//! depends on names (the client class, an auth option) and for README examples.
//!
//! Only the splits the templates need: the last bracket pair of the line with something inside
//! is split first ("right-hand split"); its contents stay on one line when they fit and are an
//! argument or parameter list without a magic trailing comma, and are otherwise put one element
//! per line with a trailing comma (a list, dict or tuple display always is, as ruff does). Strings
//! are skipped; comments are not supported.

/// A bracket pair at depth 0: byte offsets of the opening and the closing bracket.
#[derive(Debug, Clone, Copy)]
struct Pair {
    open: usize,
    close: usize,
}

fn width(s: &str) -> usize {
    s.chars().count()
}

/// Byte offset just past the string literal starting at `start` (a quote).
fn skip_string(s: &str, start: usize) -> usize {
    let bytes = s.as_bytes();
    let quote = bytes[start];
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == quote => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// The depth-0 bracket pairs of `s`, in order.
fn top_pairs(s: &str) -> Vec<Pair> {
    let bytes = s.as_bytes();
    let mut pairs = Vec::new();
    let mut depth = 0usize;
    let mut open = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                i = skip_string(s, i);
                continue;
            }
            b'(' | b'[' | b'{' => {
                if depth == 0 {
                    open = i;
                }
                depth += 1;
            }
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    pairs.push(Pair { open, close: i });
                }
            }
            _ => {}
        }
        i += 1;
    }
    pairs
}

/// `s` split at its depth-0 commas, each part trimmed; a trailing empty part is dropped.
fn split_commas(s: &str) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                i = skip_string(s, i);
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let last = s[start..].trim();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

/// Whether any bracket pair in `s` ends with a comma before its closing bracket.
fn has_magic_comma(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut last = None;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                i = skip_string(s, i);
                last = Some(b'"');
                continue;
            }
            b')' | b']' | b'}' if last == Some(b',') => return true,
            b' ' | b'\n' => {}
            b => last = Some(b),
        }
        i += 1;
    }
    false
}

/// A list, dict, set or tuple display (not a call, a subscript or a parameter list): the
/// opening bracket follows an operator, not a name or another bracket's close.
fn is_display(line: &str, open: usize) -> bool {
    let before = line[..open].trim_end();
    let prev = before.chars().last();
    let follows_name = prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ')' || c == ']');
    match line.as_bytes()[open] {
        b'{' => true,
        _ => !follows_name || before.ends_with(" in") || before.ends_with(" return"),
    }
}

fn lay_out(line: &str, max: usize, out: &mut Vec<String>) {
    if width(line) <= max && !has_magic_comma(line) {
        out.push(line.to_owned());
        return;
    }
    let Some(pair) = top_pairs(line)
        .into_iter()
        .rev()
        .find(|p| !line[p.open + 1..p.close].trim().is_empty())
    else {
        out.push(line.to_owned());
        return;
    };
    let indent = &line[..line.len() - line.trim_start().len()];
    let inner = format!("{indent}    ");
    let head = &line[..=pair.open];
    let body = line[pair.open + 1..pair.close].trim();
    let tail = &line[pair.close..];
    let items = split_commas(body);
    let magic = body.ends_with(',');
    out.push(head.to_owned());
    if items.len() > 1 || magic {
        let one_line = format!("{inner}{body}");
        let keep = !magic && !is_display(line, pair.open) && width(&one_line) <= max && !has_magic_comma(body);
        if keep {
            out.push(one_line);
        } else {
            for item in items {
                lay_out(&format!("{inner}{item},"), max, out);
            }
        }
    } else {
        lay_out(&format!("{inner}{body}"), max, out);
    }
    out.push(format!("{indent}{tail}"));
}

/// `line` (one statement, without its line terminator) laid out within `max` columns.
#[must_use]
pub fn layout(line: &str, max: usize) -> String {
    let mut out = Vec::new();
    lay_out(line, max, &mut out);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::layout;

    #[test]
    fn keeps_a_line_that_fits() {
        assert_eq!(layout("    x = f(a, b)", 120), "    x = f(a, b)");
    }

    #[test]
    fn puts_call_arguments_on_one_line_when_they_fit() {
        assert_eq!(
            layout("        client = Client(base_url=base_url, api_key=api_key)", 50),
            "        client = Client(\n            base_url=base_url, api_key=api_key\n        )"
        );
    }

    #[test]
    fn explodes_arguments_that_do_not_fit() {
        assert_eq!(
            layout("x = f(aaaa=1, bbbb=2)", 12),
            "x = f(\n    aaaa=1,\n    bbbb=2,\n)"
        );
    }

    #[test]
    fn splits_the_last_bracket_with_contents() {
        assert_eq!(
            layout("    c.x.create(model=\"m\", texts=[\"a\"]).__enter__()", 30),
            "    c.x.create(\n        model=\"m\", texts=[\"a\"]\n    ).__enter__()"
        );
    }

    #[test]
    fn honours_magic_trailing_commas_and_explodes_displays() {
        assert_eq!(
            layout("with update_env(**{\"A\": Omit(), \"B\": Omit(),}):", 120),
            "with update_env(\n    **{\n        \"A\": Omit(),\n        \"B\": Omit(),\n    }\n):"
        );
        assert_eq!(
            layout("x = f(a=[\"aaaa\", \"bbbb\"])", 16),
            "x = f(\n    a=[\n        \"aaaa\",\n        \"bbbb\",\n    ]\n)"
        );
        assert_eq!(layout("print(x)", 120), "print(x)");
    }

    #[test]
    fn skips_brackets_in_strings() {
        assert_eq!(
            layout("f(\"(\", 'a)', \"b\\\"(\")", 10),
            "f(\n    \"(\",\n    'a)',\n    \"b\\\"(\",\n)"
        );
    }
}
