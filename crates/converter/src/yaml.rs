//! YAML 1.2 (core schema) to [`Value`], with node positions, matching how the TypeScript
//! converter reads YAML with the `yaml` package: plain scalars resolve as the core schema does,
//! duplicate keys are errors, `<<` is an ordinary key, and keys become strings the way `toJS`
//! makes them.

use crate::value::{Map, Value, number_to_string};
use std::fmt::Write as _;
use yaml_rust2::parser::{Event, Parser, Tag};
use yaml_rust2::scanner::{Marker, ScanError, TScalarStyle};

/// A 1-based line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

/// One node of the document tree. Children are indices into [`Document::nodes`].
#[derive(Debug, Clone)]
pub enum NodeKind {
    Scalar(Value),
    Seq(Vec<usize>),
    Map(Vec<(usize, usize)>),
    /// An alias to the anchored node with this index.
    Alias(usize),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: NodeKind,
    pub pos: Pos,
}

/// A parse problem, as the `yaml` package reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError {
    pub message: String,
    pub pos: Pos,
    /// Width of the offending source range on its line, for the caret line of the message.
    pub width: usize,
}

/// A parsed document: nodes, the root, and errors.
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub nodes: Vec<Node>,
    pub root: Option<usize>,
    pub errors: Vec<YamlError>,
    /// Set when `toJS` would throw (an alias to an anchor that is not defined).
    pub fatal: Option<String>,
}

impl Document {
    #[must_use]
    pub fn node(&self, id: usize) -> &Node {
        &self.nodes[id]
    }

    /// The plain value (`doc.toJS()`); `Null` for an empty document.
    ///
    /// # Errors
    ///
    /// When `toJS` would throw.
    pub fn to_value(&self) -> Result<Value, String> {
        if let Some(f) = &self.fatal {
            return Err(f.clone());
        }
        Ok(self.root.map_or(Value::Null, |r| self.value_of(r, 0)))
    }

    fn value_of(&self, id: usize, depth: usize) -> Value {
        if depth > 10_000 {
            return Value::Null;
        }
        match &self.nodes[id].kind {
            NodeKind::Scalar(v) => v.clone(),
            NodeKind::Seq(items) => Value::Array(items.iter().map(|&i| self.value_of(i, depth + 1)).collect()),
            NodeKind::Map(pairs) => {
                let mut map = Map::new();
                for &(k, v) in pairs {
                    map.insert(self.key_string(k), self.value_of(v, depth + 1));
                }
                Value::Object(map)
            }
            NodeKind::Alias(target) => self.value_of(*target, depth + 1),
        }
    }

    /// The string a map key becomes in `toJS` (`stringifyKey`).
    #[must_use]
    pub fn key_string(&self, id: usize) -> String {
        match &self.nodes[id].kind {
            NodeKind::Scalar(Value::Null) => String::new(),
            NodeKind::Scalar(Value::String(s)) => s.clone(),
            NodeKind::Scalar(Value::Number(n)) => number_to_string(*n),
            NodeKind::Scalar(Value::Bool(b)) => b.to_string(),
            NodeKind::Alias(t) => self.key_string(*t),
            _ => self.flow(id, 0),
        }
    }

    /// A collection key, stringified as the `yaml` package's flow style does.
    fn flow(&self, id: usize, depth: usize) -> String {
        if depth > 64 {
            return String::new();
        }
        match &self.nodes[id].kind {
            NodeKind::Scalar(Value::Null) => String::new(),
            NodeKind::Scalar(v) => crate::value::js_string(v),
            NodeKind::Seq(items) if items.is_empty() => "[]".to_owned(),
            NodeKind::Seq(items) => {
                let parts: Vec<String> = items.iter().map(|&i| self.flow(i, depth + 1)).collect();
                format!("[ {} ]", parts.join(", "))
            }
            NodeKind::Map(pairs) if pairs.is_empty() => "{}".to_owned(),
            NodeKind::Map(pairs) => {
                let parts: Vec<String> = pairs
                    .iter()
                    .map(|&(k, v)| format!("{}: {}", self.flow(k, depth + 1), self.flow(v, depth + 1)))
                    .collect();
                format!("{{ {} }}", parts.join(", "))
            }
            NodeKind::Alias(t) => self.flow(*t, depth + 1),
        }
    }

    /// Follow an alias to its anchored node.
    #[must_use]
    pub fn resolve(&self, id: usize) -> usize {
        let mut id = id;
        for _ in 0..64 {
            match self.nodes[id].kind {
                NodeKind::Alias(t) => id = t,
                _ => return id,
            }
        }
        id
    }
}

fn pos_of(m: &Marker) -> Pos {
    Pos {
        line: m.line(),
        col: m.col() + 1,
    }
}

/// Resolve a plain scalar with the YAML 1.2 core schema, as the `yaml` package does.
#[must_use]
pub fn resolve_plain(s: &str) -> Value {
    match s {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        ".nan" | ".NaN" | ".NAN" => return Value::Number(f64::NAN),
        _ => {}
    }
    if let Some(n) = int_value(s) {
        return Value::Number(n);
    }
    let unsigned = s.strip_prefix(['-', '+']).unwrap_or(s);
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return Value::Number(if s.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    if is_float(unsigned) {
        return Value::Number(s.parse::<f64>().unwrap_or(f64::NAN));
    }
    Value::String(s.to_owned())
}

fn radix_value(digits: &str, radix: u32) -> f64 {
    if let Ok(n) = u128::from_str_radix(digits, radix) {
        #[expect(clippy::cast_precision_loss, reason = "parseInt rounds to the nearest double too")]
        return n as f64;
    }
    digits.chars().fold(0.0, |acc, c| {
        acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
    })
}

fn int_value(s: &str) -> Option<f64> {
    if let Some(oct) = s.strip_prefix("0o") {
        return (!oct.is_empty() && oct.bytes().all(|b| (b'0'..=b'7').contains(&b))).then(|| radix_value(oct, 8));
    }
    if let Some(hex) = s.strip_prefix("0x") {
        return (!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())).then(|| radix_value(hex, 16));
    }
    let (neg, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n = digits.parse::<f64>().unwrap_or(f64::NAN);
    Some(if neg { -n } else { n })
}

/// `^(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)(?:[eE][-+]?[0-9]+)?$`, with a `.` or an exponent.
fn is_float(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i;
    let mut dot = false;
    let mut frac = 0;
    if i < b.len() && b[i] == b'.' {
        dot = true;
        i += 1;
        let f = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        frac = i - f;
    }
    if int_digits == 0 && frac == 0 {
        return false;
    }
    let mut exp = false;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let e = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == e {
            return false;
        }
        exp = true;
    }
    i == b.len() && (dot || exp)
}

fn tagged(value: &str, style: TScalarStyle, tag: Option<&Tag>) -> Value {
    let plain = style == TScalarStyle::Plain;
    match tag {
        Some(t)
            if (t.handle == "tag:yaml.org,2002:" || t.handle == "!!")
                && matches!(t.suffix.as_str(), "null" | "bool" | "int" | "float") =>
        {
            match resolve_plain(value) {
                v @ Value::Null if t.suffix == "null" => v,
                v @ Value::Bool(_) if t.suffix == "bool" => v,
                v @ Value::Number(_) if t.suffix == "int" || t.suffix == "float" => v,
                _ => Value::String(value.to_owned()),
            }
        }
        None if plain => resolve_plain(value),
        _ => Value::String(value.to_owned()),
    }
}

/// JavaScript `===` for scalar keys (the `yaml` package's duplicate-key test).
fn same_scalar(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        #[expect(clippy::float_cmp, reason = "JavaScript === on numbers")]
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

struct Builder<'a> {
    parser: Parser<std::str::Chars<'a>>,
    doc: Document,
    anchors: std::collections::HashMap<usize, usize>,
    lines: Vec<&'a str>,
    /// How many flow collections enclose the node being built.
    flow_depth: usize,
    /// Lines already reported for tab indentation.
    tab_lines: std::collections::HashSet<usize>,
}

/// The end of the text, as `yaml` reports errors found there.
fn end_pos(lines: &[&str]) -> Pos {
    Pos {
        line: lines.len().max(1),
        col: lines.last().map_or(0, |l| l.chars().count()) + 1,
    }
}

/// An open flow collection while scanning.
struct Open {
    seq: bool,
    /// Indentation of the block line it opened on; `None` for a flow collection at the root.
    block_indent: Option<usize>,
}

fn flow_message(open: &Open) -> String {
    match (open.seq, open.block_indent.is_some()) {
        (true, true) => "Flow sequence in block collection must be sufficiently indented and end with a ]".to_owned(),
        (false, true) => "Flow map in block collection must be sufficiently indented and end with a }".to_owned(),
        (true, false) => "Flow sequence must end with a ]".to_owned(),
        (false, false) => "Flow map must end with a }".to_owned(),
    }
}

/// Find flow collections that are never closed, the way the `yaml` package reports them: at the
/// first line indented no deeper than the block line they opened on, or at the end of the text,
/// innermost first. `None` when the text has no such collection (or one this scan cannot judge,
/// such as a mismatched closing bracket).
fn unclosed_flows(lines: &[&str]) -> Option<Vec<YamlError>> {
    let mut stack: Vec<Open> = Vec::new();
    let report = |stack: &[Open], pos: Pos| -> Vec<YamlError> {
        stack
            .iter()
            .rev()
            .map(|o| YamlError {
                message: flow_message(o),
                pos,
                width: 1,
            })
            .collect()
    };
    for (li, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let indent = chars.iter().take_while(|c| **c == ' ').count();
        let content = chars.get(indent).copied();
        let blank = content.is_none_or(|c| c == '#');
        if let Some(first) = stack.first()
            && let (Some(block), false) = (first.block_indent, blank)
            && indent <= block
        {
            return Some(report(
                &stack,
                Pos {
                    line: li + 1,
                    col: indent + 1,
                },
            ));
        }
        let mut i = indent;
        // At the start of a block line, after `- `, `? ` or `: ` a node may begin.
        let mut node_start = true;
        while i < chars.len() {
            let c = chars[i];
            if c == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
                break;
            }
            if c == '"' || c == '\'' {
                if stack.is_empty() && !node_start {
                    i += 1;
                    continue;
                }
                let j = closing_quote(&chars, i)?;
                i = j + 1;
                node_start = false;
                continue;
            }
            if stack.is_empty() {
                if node_start && (c == '[' || c == '{') {
                    let block_indent = if i == indent && is_root_start(lines, li) {
                        None
                    } else {
                        Some(indent)
                    };
                    stack.push(Open {
                        seq: c == '[',
                        block_indent,
                    });
                    i += 1;
                    continue;
                }
                let next_ws = chars.get(i + 1).is_none_or(|n| n.is_whitespace());
                node_start = matches!(c, '-' | '?' | ':') && next_ws || (node_start && c.is_whitespace());
                i += 1;
                continue;
            }
            match c {
                '[' | '{' => {
                    let block_indent = stack[0].block_indent;
                    stack.push(Open {
                        seq: c == '[',
                        block_indent,
                    });
                }
                ']' | '}' => {
                    let top = stack.pop()?;
                    if top.seq != (c == ']') {
                        return None;
                    }
                    node_start = false;
                }
                _ => {}
            }
            i += 1;
        }
    }
    if stack.is_empty() {
        None
    } else {
        Some(report(&stack, end_pos(lines)))
    }
}

/// The index of the quote that closes the quoted scalar opening at `start` on this line.
fn closing_quote(chars: &[char], start: usize) -> Option<usize> {
    let q = chars[start];
    let mut j = start + 1;
    while j < chars.len() {
        if q == '"' && chars[j] == '\\' {
            j += 2;
            continue;
        }
        if chars[j] == q {
            if q == '\'' && chars.get(j + 1) == Some(&'\'') {
                j += 2;
                continue;
            }
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Whether line `li` starts the document's root node (only comments or `---` before it).
fn is_root_start(lines: &[&str], li: usize) -> bool {
    lines[..li].iter().all(|l| {
        let t = l.trim();
        t.is_empty() || t.starts_with('#') || t == "---"
    })
}

/// The `yaml` package's errors for a scanner error, where it has a close equivalent.
fn translate(err: &ScanError, lines: &[&str]) -> Vec<YamlError> {
    let info = err.info();
    let pos = pos_of(err.marker());
    let at_line = |line: usize| lines.get(line.saturating_sub(1)).copied().unwrap_or("");
    let char_at = |p: Pos| at_line(p.line).chars().nth(p.col.saturating_sub(1));
    if let Some(errors) = unclosed_flows(lines) {
        return errors;
    }
    let one = |message: String, pos: Pos| vec![YamlError { message, pos, width: 1 }];
    if info.contains("quoted scalar") && info.contains("end of stream") {
        let quote = char_at(pos).filter(|c| *c == '"' || *c == '\'').unwrap_or('"');
        return one(format!("Missing closing {quote}quote"), end_pos(lines));
    }
    if info.contains("unknown escape character") {
        // The `yaml` package points at the backslash of the first invalid escape.
        for (li, line) in lines.iter().enumerate().skip(pos.line.saturating_sub(1)) {
            let chars: Vec<char> = line.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '\\' {
                    match chars.get(i + 1) {
                        Some(c) if "0abt\tnvfre \"/\\N_LPxuU".contains(*c) => {
                            i += 2;
                            continue;
                        }
                        None => {
                            i += 1;
                            continue;
                        }
                        Some(c) => {
                            return one(
                                format!("Invalid escape sequence \\{c}"),
                                Pos {
                                    line: li + 1,
                                    col: i + 1,
                                },
                            );
                        }
                    }
                }
                i += 1;
            }
        }
    }
    if info.contains("tab") {
        let tab = at_line(pos.line)
            .chars()
            .position(|c| c == '\t')
            .map_or(pos.col, |i| i + 1);
        return one(
            "Tabs are not allowed as indentation".to_owned(),
            Pos {
                line: pos.line,
                col: tab,
            },
        );
    }
    if (info.contains("unexpected character") || info.contains("cannot start any token"))
        && let Some(c) = char_at(pos)
    {
        if c == '@' || c == '`' {
            return one(format!("Plain value cannot start with reserved character {c}"), pos);
        }
        if c == '%' {
            return one(
                "Plain value cannot start with directive indicator character %".to_owned(),
                pos,
            );
        }
    }
    one(info.to_owned(), pos)
}

impl Builder<'_> {
    fn next(&mut self) -> Result<(Event, Marker), ScanError> {
        self.parser.next_token()
    }

    fn push(&mut self, kind: NodeKind, pos: Pos) -> usize {
        if self.flow_depth == 0 {
            self.check_tab_indent(pos);
        }
        self.doc.nodes.push(Node { kind, pos });
        self.doc.nodes.len() - 1
    }

    /// A block node whose line is indented with a tab (the scanner accepts some of these).
    fn check_tab_indent(&mut self, pos: Pos) {
        let chars = self.line_chars(pos.line);
        let prefix = &chars[..pos.col.saturating_sub(1).min(chars.len())];
        if prefix.iter().all(|c| *c == ' ' || *c == '\t')
            && let Some(tab) = prefix.iter().position(|c| *c == '\t')
            && self.tab_lines.insert(pos.line)
        {
            self.doc.errors.push(YamlError {
                message: "Tabs are not allowed as indentation".to_owned(),
                pos: Pos {
                    line: pos.line,
                    col: tab + 1,
                },
                width: 1,
            });
        }
    }

    fn anchor(&mut self, anchor: usize, id: usize) {
        if anchor > 0 {
            self.anchors.insert(anchor, id);
        }
    }

    /// Build the node that starts with `ev`.
    fn node(&mut self, ev: Event, mark: Marker, depth: usize) -> Result<usize, ScanError> {
        if depth > 5_000 {
            return Err(ScanError::new(mark, "document too deeply nested"));
        }
        let pos = pos_of(&mark);
        match ev {
            Event::Scalar(v, style, anchor, tag) => {
                let pos = if matches!(style, TScalarStyle::Literal | TScalarStyle::Folded) {
                    self.block_header(pos)
                } else {
                    pos
                };
                let id = self.push(NodeKind::Scalar(tagged(&v, style, tag.as_ref())), pos);
                self.anchor(anchor, id);
                Ok(id)
            }
            Event::Alias(anchor) => {
                let Some(&target) = self.anchors.get(&anchor) else {
                    return Err(ScanError::new(mark, "while parsing node, found unknown anchor"));
                };
                Ok(self.push(NodeKind::Alias(target), pos))
            }
            Event::SequenceStart(anchor, _) => {
                let pos = self.sequence_start(pos);
                let id = self.push(NodeKind::Seq(Vec::new()), pos);
                self.anchor(anchor, id);
                let flow = self.char_at(pos) == Some('[');
                self.flow_depth += usize::from(flow);
                let mut items = Vec::new();
                loop {
                    let (ev, m) = self.next()?;
                    if ev == Event::SequenceEnd {
                        break;
                    }
                    items.push(self.node(ev, m, depth + 1)?);
                }
                self.flow_depth -= usize::from(flow);
                self.doc.nodes[id].kind = NodeKind::Seq(items);
                Ok(id)
            }
            Event::MappingStart(anchor, _) => {
                let flow = self.char_at(pos) == Some('{');
                let id = self.push(NodeKind::Map(Vec::new()), pos);
                self.anchor(anchor, id);
                self.flow_depth += usize::from(flow);
                let mut pairs: Vec<(usize, usize)> = Vec::new();
                loop {
                    let (ev, m) = self.next()?;
                    if ev == Event::MappingEnd {
                        break;
                    }
                    let key = self.node(ev, m, depth + 1)?;
                    if let NodeKind::Scalar(kv) = &self.doc.nodes[key].kind {
                        let dup = pairs.iter().any(|&(k, _)| match &self.doc.nodes[k].kind {
                            NodeKind::Scalar(other) => same_scalar(kv, other),
                            _ => false,
                        });
                        if dup {
                            let width = match kv {
                                Value::String(s) => s.chars().count().max(1),
                                _ => 1,
                            };
                            let pos = self.doc.nodes[key].pos;
                            self.doc.errors.push(YamlError {
                                message: "Map keys must be unique".to_owned(),
                                pos,
                                width,
                            });
                        }
                    }
                    let (ev, m) = self.next()?;
                    let implicit_empty = matches!(&ev, Event::Scalar(v, TScalarStyle::Plain, 0, None) if v.is_empty());
                    let value = self.node(ev, m, depth + 1)?;
                    if implicit_empty {
                        let key_pos = self.doc.nodes[key].pos;
                        if let Some(after) = self.after_colon(key_pos) {
                            self.doc.nodes[value].pos = after;
                        }
                    }
                    pairs.push((key, value));
                }
                self.flow_depth -= usize::from(flow);
                // A block mapping starts at its first key; the scanner marks the first `:`.
                if self.char_at(pos) != Some('{')
                    && let Some(&(k, _)) = pairs.first()
                {
                    self.doc.nodes[id].pos = self.doc.nodes[k].pos;
                }
                self.doc.nodes[id].kind = NodeKind::Map(pairs);
                Ok(id)
            }
            _ => Err(ScanError::new(mark, "did not find expected node content")),
        }
    }

    fn line_chars(&self, line: usize) -> Vec<char> {
        self.lines
            .get(line.wrapping_sub(1))
            .map(|l| l.chars().collect())
            .unwrap_or_default()
    }

    fn char_at(&self, pos: Pos) -> Option<char> {
        self.lines
            .get(pos.line.wrapping_sub(1))?
            .chars()
            .nth(pos.col.wrapping_sub(1))
    }

    /// A block sequence starts at its first `-`.
    fn sequence_start(&self, pos: Pos) -> Pos {
        match self.char_at(pos) {
            Some('[' | '-') | None => pos,
            Some(_) => {
                let chars = self.line_chars(pos.line);
                let mut i = pos.col.saturating_sub(1).min(chars.len());
                while i > 0 {
                    i -= 1;
                    if chars[i] == '-' && chars.get(i + 1).is_none_or(|c| c.is_whitespace()) {
                        return Pos {
                            line: pos.line,
                            col: i + 1,
                        };
                    }
                }
                pos
            }
        }
    }

    /// A block scalar starts at its `|` or `>` header.
    fn block_header(&self, pos: Pos) -> Pos {
        let mut line = pos.line;
        let mut limit = pos.col.saturating_sub(1);
        loop {
            let chars = self.line_chars(line);
            let end = limit.min(chars.len());
            // Strip a trailing comment, then look for `[|>][0-9+-]*` at the end.
            let mut text: Vec<char> = chars[..end].to_vec();
            if let Some(h) = (0..text.len()).find(|&i| text[i] == '#' && (i == 0 || text[i - 1].is_whitespace())) {
                text.truncate(h);
            }
            while text.last().is_some_and(|c| c.is_whitespace()) {
                text.pop();
            }
            while text
                .last()
                .is_some_and(|c| c.is_ascii_digit() || *c == '+' || *c == '-')
            {
                text.pop();
            }
            if matches!(text.last(), Some('|' | '>')) {
                return Pos { line, col: text.len() };
            }
            if line <= 1 || pos.line - line > 1_000 {
                return pos;
            }
            line -= 1;
            limit = usize::MAX;
        }
    }

    /// The position just after the `:` that follows the key at `key`.
    fn after_colon(&self, key: Pos) -> Option<Pos> {
        let chars = self.line_chars(key.line);
        let mut i = key.col.checked_sub(1)?;
        if let Some(&q) = chars.get(i).filter(|c| **c == '"' || **c == '\'') {
            i += 1;
            while i < chars.len() {
                if q == '"' && chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == q {
                    if q == '\'' && chars.get(i + 1) == Some(&'\'') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
        }
        while i < chars.len() {
            if chars[i] == ':'
                && chars
                    .get(i + 1)
                    .is_none_or(|c| c.is_whitespace() || matches!(c, ',' | '}' | ']'))
            {
                return Some(Pos {
                    line: key.line,
                    col: i + 2,
                });
            }
            i += 1;
        }
        None
    }

    fn run(&mut self) -> Result<(), ScanError> {
        let mut documents = 0;
        loop {
            let (ev, mark) = self.next()?;
            match ev {
                Event::StreamStart | Event::DocumentEnd | Event::Nothing => {}
                Event::StreamEnd => return Ok(()),
                Event::DocumentStart => {
                    documents += 1;
                    if documents > 1 {
                        // At the `---` of an explicit document, else at its first content.
                        let at = pos_of(&mark);
                        let chars = self.line_chars(at.line);
                        let first = chars.iter().position(|c| !c.is_whitespace()).unwrap_or(0);
                        let explicit: String = chars.iter().skip(first).take(3).collect();
                        let pos = if explicit == "---" {
                            at
                        } else {
                            Pos {
                                line: at.line,
                                col: first + 1,
                            }
                        };
                        self.doc.errors.push(YamlError {
                            message: "Source contains multiple documents; please use YAML.parseAllDocuments()"
                                .to_owned(),
                            pos,
                            width: 1,
                        });
                        return Ok(());
                    }
                    let (ev, mark) = self.next()?;
                    if ev == Event::DocumentEnd {
                        continue;
                    }
                    let root = self.node(ev, mark, 0)?;
                    // An empty plain scalar as the whole document is an empty document.
                    let empty = matches!(self.doc.nodes[root].kind, NodeKind::Scalar(Value::Null))
                        && self.doc.nodes.len() == 1
                        && self.is_blank();
                    if !empty {
                        self.doc.root = Some(root);
                    }
                }
                _ => {
                    let root = self.node(ev, mark, 0)?;
                    self.doc.root = Some(root);
                }
            }
        }
    }

    fn is_blank(&self) -> bool {
        self.lines.iter().all(|l| {
            let t = l.trim();
            t.is_empty() || t.starts_with('#') || t == "---" || t == "..."
        })
    }
}

/// Parse one YAML document (`parseDocument`). Never fails: problems are in `errors`.
#[must_use]
pub fn parse_document(text: &str) -> Document {
    let lines: Vec<&str> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let mut b = Builder {
        parser: Parser::new_from_str(text),
        doc: Document::default(),
        anchors: std::collections::HashMap::new(),
        lines,
        flow_depth: 0,
        tab_lines: std::collections::HashSet::new(),
    };
    if let Err(e) = b.run() {
        if e.info().contains("unknown anchor") {
            let name = alias_name(&b.lines, pos_of(e.marker()));
            b.doc.fatal = Some(format!(
                "Unresolved alias (the anchor must be set before the alias): {name}"
            ));
        } else {
            let errors = translate(&e, &b.lines);
            b.doc.errors.extend(errors);
        }
    }
    let mut doc = b.doc;
    doc.errors.sort_by_key(|e| (e.pos.line, e.pos.col));
    doc
}

fn alias_name(lines: &[&str], pos: Pos) -> String {
    let line = lines.get(pos.line.saturating_sub(1)).copied().unwrap_or("");
    let rest: String = line.chars().skip(pos.col).collect();
    rest.chars()
        .take_while(|c| !c.is_whitespace() && !matches!(c, ',' | ']' | '}'))
        .collect()
}

/// `error.message` as `yaml`'s `parse` throws it (`prettyErrors`): position and source context.
#[must_use]
pub fn pretty_error(text: &str, e: &YamlError) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut message = format!("{} at line {}, column {}", e.message, e.pos.line, e.pos.col);
    let raw = lines.get(e.pos.line - 1).copied().unwrap_or("");
    let mut line_str: Vec<char> = raw.trim_end_matches(['\n', '\r']).chars().collect();
    let mut ci = e.pos.col - 1;
    if ci >= 60 && line_str.len() > 80 {
        let trim_start = (ci - 39).min(line_str.len() - 79);
        let mut v = vec!['…'];
        v.extend_from_slice(&line_str[trim_start..]);
        line_str = v;
        ci -= trim_start - 1;
    }
    if line_str.len() > 80 {
        line_str.truncate(79);
        line_str.push('…');
    }
    let mut context: String = line_str.iter().collect();
    let before: String = line_str.iter().take(ci).collect();
    if e.pos.line > 1 && before.chars().all(|c| c == ' ') {
        let prev = lines.get(e.pos.line - 2).copied().unwrap_or("");
        let prev: String = if prev.chars().count() > 80 {
            let mut p: String = prev.chars().take(79).collect();
            p.push_str("…\n");
            p
        } else {
            prev.to_owned()
        };
        context = prev + &context;
    }
    if context.chars().any(|c| c != ' ') {
        let count = e.width.clamp(1, 80usize.saturating_sub(ci).max(1));
        let _ = write!(message, ":\n\n{context}\n{}{}\n", " ".repeat(ci), "^".repeat(count));
    }
    message
}

/// `YAML.parse(text)`: the plain value, or the message of the first error it throws.
///
/// # Errors
///
/// The first parse error, formatted as `yaml` formats it, or a `toJS` failure.
pub fn parse(text: &str) -> Result<Value, String> {
    let doc = parse_document(text);
    if let Some(e) = doc.errors.first() {
        return Err(pretty_error(text, e));
    }
    doc.to_value()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::to_json;

    fn js(text: &str) -> String {
        to_json(&parse_document(text).to_value().unwrap())
    }

    #[test]
    fn resolves_core_schema_scalars() {
        assert_eq!(
            js(
                "a: 0x1F\nb: 0o17\nc: 1_000\nd: .inf\ne: -.INF\nf: .NaN\ng: 1e3\nh: 1.\ni: .5\nj: +1\nk: 012\nl: 1e400\nm: 9007199254740993\nn: true\no: True\np: yes\nq: ~\nr: NULL\ns: 1.0\nt: -0\n"
            ),
            r#"{"a":31,"b":15,"c":"1_000","d":null,"e":null,"f":null,"g":1000,"h":1,"i":0.5,"j":1,"k":12,"l":null,"m":9007199254740992,"n":true,"o":true,"p":"yes","q":null,"r":null,"s":1,"t":0}"#
        );
    }

    #[test]
    fn keys_become_strings_like_to_js() {
        let doc = parse_document("1: a\n0x10: b\n1.5: c\nnull: d\ntrue: e\n");
        assert_eq!(
            to_json(&doc.to_value().unwrap()),
            r#"{"1":"a","16":"b","1.5":"c","":"d","true":"e"}"#
        );
    }

    #[test]
    fn reports_duplicate_keys_and_keeps_the_last_value() {
        let doc = parse_document("a: 1\na: 2\n");
        assert_eq!(doc.errors[0].message, "Map keys must be unique");
        assert_eq!(doc.errors[0].pos, Pos { line: 2, col: 1 });
        assert_eq!(to_json(&doc.to_value().unwrap()), r#"{"a":2}"#);
        assert_eq!(
            parse("a: 1\na: 2\n").unwrap_err(),
            "Map keys must be unique at line 2, column 1:\n\na: 1\na: 2\n^\n"
        );
    }

    #[test]
    fn merge_keys_are_plain_keys_and_aliases_resolve() {
        assert_eq!(js("<<: {a: 1}\nb: 2\n"), r#"{"<<":{"a":1},"b":2}"#);
        assert_eq!(js("a: &x 1\nb: *x\n"), r#"{"a":1,"b":1}"#);
        assert_eq!(
            parse_document("a: *nope\n").to_value().unwrap_err(),
            "Unresolved alias (the anchor must be set before the alias): nope"
        );
    }

    #[test]
    fn empty_documents_are_null() {
        assert!(parse_document("").root.is_none());
        assert!(parse_document("# just a comment\n").root.is_none());
        assert_eq!(to_json(&parse_document("").to_value().unwrap()), "null");
    }

    #[test]
    fn block_scalars_and_escapes() {
        assert_eq!(js("a: |\n  x\n  y\n\nb: >-\n  p\n  q\n"), r#"{"a":"x\ny\n","b":"p q"}"#);
        assert_eq!(js("\"a\\tb\\u00e9\\x41\\/\": 1\n"), r#"{"a\tbéA/":1}"#);
    }

    #[test]
    fn multiple_documents_are_an_error() {
        let doc = parse_document("---\na: 1\n---\nb: 2\n");
        assert_eq!(
            doc.errors[0].message,
            "Source contains multiple documents; please use YAML.parseAllDocuments()"
        );
    }
}
