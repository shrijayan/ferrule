//! A pure, deterministic evaluator for the JSONPath Plus subset that `openapi.transforms` uses.
//! Port of `converter/src/transforms/jsonpath.ts`.
//!
//! Supported: `$`, `.name`, `..name`, `.*`, `..*`, `['name']`, `["name"]`, `[0]`, `[-1]`, `[*]`,
//! unions (`['a','b']`, `[0,2]`), slices (`[0:2]`, `[::2]`) and filters (`[?(@.name == "id")]`).
//! Filters accept `@`, `@property`, `@parentProperty`, `@parent`, `@root`/`$`, member access, the
//! literals JSON has plus `undefined` and `/regex/flags`, `! && || == === != !== < <= > >=`,
//! parentheses, and the string/array methods `includes`, `startsWith`, `endsWith`, `match`,
//! `test`, `toLowerCase`, `toUpperCase` and `trim`. Dot names run to the next `.` or `[`, so
//! `$.paths./users/{id}.get` works as in JSONPath Plus.
//!
//! Anything else (script expressions, `^` parent, `~` property names, type selectors) is a
//! [`JsonPathError`] so callers can turn it into a diagnostic instead of guessing.

use crate::diagnostic::Seg;
use crate::value::{Value, cmp_utf16, is_js_space, string_to_number};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::rc::Rc;

/// A node selected by a query: its location (from the root) and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct Match<'a> {
    pub path: Vec<Seg>,
    pub value: &'a Value,
}

/// Invalid or unsupported JSONPath syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPathError(pub String);

impl std::fmt::Display for JsonPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One selector of a segment.
#[derive(Debug, Clone)]
pub enum Selector {
    Name(String),
    Index(i64),
    Wildcard,
    Slice {
        start: Option<i64>,
        end: Option<i64>,
        step: i64,
    },
    Filter(Rc<Expr>),
}

/// One segment: `.x`/`[...]` or, with `descendant`, `..x`/`..[...]`.
#[derive(Debug, Clone)]
pub struct Segment {
    pub descendant: bool,
    pub selectors: Vec<Selector>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctx {
    Current,
    Property,
    ParentProperty,
    Parent,
    Root,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Includes,
    StartsWith,
    EndsWith,
    Match,
    Test,
    ToLowerCase,
    ToUpperCase,
    Trim,
}

const METHODS: [(&str, Method); 8] = [
    ("includes", Method::Includes),
    ("startsWith", Method::StartsWith),
    ("endsWith", Method::EndsWith),
    ("match", Method::Match),
    ("test", Method::Test),
    ("toLowerCase", Method::ToLowerCase),
    ("toUpperCase", Method::ToUpperCase),
    ("trim", Method::Trim),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Or,
    And,
    StrictEq,
    StrictNe,
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
}

/// A JavaScript regular expression literal.
#[derive(Debug)]
pub struct RegexLit {
    re: regress::Regex,
    global: bool,
    sticky: bool,
}

/// A member access key: `.name` / `["name"]`, or `[n]`.
#[derive(Debug, Clone)]
pub enum Key {
    Name(String),
    Num(i64),
}

/// A filter expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Undefined,
    Lit(Value),
    Regex(Rc<RegexLit>),
    Ctx(Ctx),
    Get(Box<Expr>, Key),
    Call(Box<Expr>, Method, Vec<Expr>),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
}

// ---------------------------------------------------------------------------------------------
// Parsing

struct Parser {
    src: Vec<u16>,
    text: String,
    pos: usize,
}

type PResult<T> = Result<T, JsonPathError>;

fn unit_is_space(u: u16) -> bool {
    char::from_u32(u32::from(u)).is_some_and(is_js_space)
}

fn is_digit(u: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&u)
}

fn is_ident_start(u: u16) -> bool {
    u8::try_from(u).is_ok_and(|b| b.is_ascii_alphabetic() || b == b'_' || b == b'$')
}

fn is_ident_part(u: u16) -> bool {
    u8::try_from(u).is_ok_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b == b'-')
}

fn is_word(u: u16) -> bool {
    u8::try_from(u).is_ok_and(|b| b.is_ascii_alphanumeric() || b == b'_')
}

impl Parser {
    fn fail<T>(&self, message: &str) -> PResult<T> {
        Err(JsonPathError(format!(
            "{message} at offset {} in \"{}\"",
            self.pos, self.text
        )))
    }

    fn peek(&self, n: usize) -> Option<u16> {
        self.src.get(self.pos + n).copied()
    }

    fn peek_is(&self, n: usize, c: char) -> bool {
        self.peek(n) == Some(c as u16)
    }

    fn peek_str(&self) -> String {
        self.peek(0).map(|u| String::from_utf16_lossy(&[u])).unwrap_or_default()
    }

    fn starts_with(&self, s: &str) -> bool {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.src.get(self.pos..self.pos + units.len()) == Some(units.as_slice())
    }

    fn eat(&mut self, s: &str) -> bool {
        if !self.starts_with(s) {
            return false;
        }
        self.pos += s.encode_utf16().count();
        true
    }

    fn expect(&mut self, s: &str) -> PResult<()> {
        if self.eat(s) {
            Ok(())
        } else {
            self.fail(&format!("Expected \"{s}\""))
        }
    }

    fn ws(&mut self) {
        while self.peek(0).is_some_and(unit_is_space) {
            self.pos += 1;
        }
    }

    fn done(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn slice(&self, from: usize, to: usize) -> String {
        String::from_utf16_lossy(&self.src[from..to])
    }

    fn path(&mut self) -> PResult<Vec<Segment>> {
        self.ws();
        if !self.eat("$") {
            return self.fail("A JSONPath must start with \"$\"");
        }
        let mut segments = Vec::new();
        self.ws();
        while !self.done() {
            if self.eat("..") {
                segments.push(Segment {
                    descendant: true,
                    selectors: self.after_dot()?,
                });
            } else if self.eat(".") {
                segments.push(Segment {
                    descendant: false,
                    selectors: self.after_dot()?,
                });
            } else if self.peek_is(0, '[') {
                segments.push(Segment {
                    descendant: false,
                    selectors: self.bracket()?,
                });
            } else {
                let c = self.peek_str();
                return self.fail(&format!("Unexpected \"{c}\""));
            }
            self.ws();
        }
        Ok(segments)
    }

    fn after_dot(&mut self) -> PResult<Vec<Selector>> {
        if self.peek_is(0, '[') {
            return self.bracket();
        }
        let start = self.pos;
        while !self.done() && !self.peek_is(0, '.') && !self.peek_is(0, '[') {
            self.pos += 1;
        }
        let raw = self.slice(start, self.pos);
        let name = raw.trim_end_matches(is_js_space).to_owned();
        if name.is_empty() {
            return self.fail("Expected a property name");
        }
        if name == "*" {
            return Ok(vec![Selector::Wildcard]);
        }
        if name.ends_with('^') || name.ends_with('~') || name.starts_with('@') || name.starts_with('(') {
            self.pos = start;
            return self.fail(&format!("Unsupported JSONPath Plus syntax \"{name}\""));
        }
        Ok(vec![Selector::Name(name)])
    }

    fn bracket(&mut self) -> PResult<Vec<Selector>> {
        self.expect("[")?;
        self.ws();
        if self.eat("?") {
            self.ws();
            let paren = self.eat("(");
            let expr = self.expr()?;
            self.ws();
            if paren {
                self.expect(")")?;
            }
            self.ws();
            self.expect("]")?;
            return Ok(vec![Selector::Filter(Rc::new(expr))]);
        }
        if self.peek_is(0, '(') {
            return self.fail("Script expressions \"[(...)]\" are not supported");
        }
        let mut selectors = Vec::new();
        loop {
            self.ws();
            selectors.push(self.union_item()?);
            self.ws();
            if !self.eat(",") {
                break;
            }
        }
        self.expect("]")?;
        Ok(selectors)
    }

    fn union_item(&mut self) -> PResult<Selector> {
        let c = self.peek(0);
        if c == Some(u16::from(b'\'')) || c == Some(u16::from(b'"')) {
            return Ok(Selector::Name(self.string()?));
        }
        if self.eat("*") {
            return Ok(Selector::Wildcard);
        }
        if c.is_some_and(|u| u == u16::from(b'-') || u == u16::from(b':') || is_digit(u)) {
            let start = self.opt_int();
            if !self.eat(":") {
                let Some(start) = start else {
                    return self.fail("Expected an index");
                };
                return Ok(Selector::Index(start));
            }
            let end = self.opt_int();
            let step = if self.eat(":") { self.opt_int().unwrap_or(1) } else { 1 };
            if step == 0 {
                return self.fail("Slice step must not be 0");
            }
            return Ok(Selector::Slice { start, end, step });
        }
        match self.ident() {
            Some(name) => Ok(Selector::Name(name)),
            None => self.fail("Expected a name, index, slice or filter"),
        }
    }

    /// `^[A-Za-z_$][\w$-]*`
    fn ident(&mut self) -> Option<String> {
        if !self.peek(0).is_some_and(is_ident_start) {
            return None;
        }
        let start = self.pos;
        self.pos += 1;
        while self.peek(0).is_some_and(is_ident_part) {
            self.pos += 1;
        }
        Some(self.slice(start, self.pos))
    }

    fn opt_int(&mut self) -> Option<i64> {
        self.ws();
        let start = self.pos;
        let mut i = self.pos;
        let neg = self.src.get(i) == Some(&u16::from(b'-'));
        if neg {
            i += 1;
        }
        let digits = i;
        while self.src.get(i).is_some_and(|u| is_digit(*u)) {
            i += 1;
        }
        if i == digits {
            return None;
        }
        let mut n: i64 = 0;
        for &u in &self.src[digits..i] {
            n = n.saturating_mul(10).saturating_add(i64::from(u - u16::from(b'0')));
        }
        self.pos = i;
        let _ = start;
        self.ws();
        Some(if neg { -n } else { n })
    }

    fn string(&mut self) -> PResult<String> {
        let quote = self.peek(0);
        self.pos += 1;
        let mut out: Vec<u16> = Vec::new();
        loop {
            if self.done() {
                return self.fail("Unterminated string");
            }
            let c = self.src[self.pos];
            self.pos += 1;
            if Some(c) == quote {
                return Ok(String::from_utf16_lossy(&out));
            }
            if c != u16::from(b'\\') {
                out.push(c);
                continue;
            }
            let e = self.peek(0);
            self.pos += 1;
            match e.and_then(|u| u8::try_from(u).ok()) {
                Some(b'u') => {
                    let hex = self.src.get(self.pos..self.pos + 4).map(String::from_utf16_lossy);
                    let Some(n) = hex
                        .filter(|h| h.len() == 4 && h.bytes().all(|b| b.is_ascii_hexdigit()))
                        .and_then(|h| u16::from_str_radix(&h, 16).ok())
                    else {
                        return self.fail("Invalid \\u escape");
                    };
                    out.push(n);
                    self.pos += 4;
                }
                Some(b'n') => out.push(u16::from(b'\n')),
                Some(b't') => out.push(u16::from(b'\t')),
                Some(b'r') => out.push(u16::from(b'\r')),
                Some(b'b') => out.push(8),
                Some(b'f') => out.push(12),
                _ => {
                    if let Some(e) = e {
                        out.push(e);
                    }
                }
            }
        }
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.binary(0)
    }

    fn binary(&mut self, level: usize) -> PResult<Expr> {
        let levels: [&[(&str, Op)]; 4] = [
            &[("||", Op::Or)],
            &[("&&", Op::And)],
            &[
                ("===", Op::StrictEq),
                ("!==", Op::StrictNe),
                ("==", Op::Eq),
                ("!=", Op::Ne),
            ],
            &[("<=", Op::Le), (">=", Op::Ge), ("<", Op::Lt), (">", Op::Gt)],
        ];
        let Some(ops) = levels.get(level) else {
            return self.unary();
        };
        let mut left = self.binary(level + 1)?;
        loop {
            self.ws();
            let Some(&(text, op)) = ops.iter().find(|(o, _)| self.starts_with(o)) else {
                return Ok(left);
            };
            self.pos += text.len();
            let right = self.binary(level + 1)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> PResult<Expr> {
        self.ws();
        if self.peek_is(0, '!') && !self.peek_is(1, '=') {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        let base = self.primary()?;
        self.postfix(base)
    }

    fn primary(&mut self) -> PResult<Expr> {
        self.ws();
        if self.eat("(") {
            let e = self.expr()?;
            self.ws();
            self.expect(")")?;
            return Ok(e);
        }
        if self.peek_is(0, '\'') || self.peek_is(0, '"') {
            return Ok(Expr::Lit(Value::String(self.string()?)));
        }
        if self.peek_is(0, '/') {
            return Ok(Expr::Regex(Rc::new(self.regex()?)));
        }
        if let Some(n) = self.number() {
            return Ok(Expr::Lit(Value::Number(n)));
        }
        if self.peek_is(0, '@') || self.peek_is(0, '$') {
            let start = self.pos;
            let mut i = self.pos + 1;
            while self
                .src
                .get(i)
                .is_some_and(|u| u8::try_from(*u).is_ok_and(|b| b.is_ascii_alphabetic()))
            {
                i += 1;
            }
            let variable = self.slice(start, i);
            let which = match variable.as_str() {
                "@" => Ctx::Current,
                "$" | "@root" => Ctx::Root,
                "@parent" => Ctx::Parent,
                "@property" => Ctx::Property,
                "@parentProperty" => Ctx::ParentProperty,
                _ => return self.fail(&format!("Unsupported variable \"{variable}\"")),
            };
            self.pos = i;
            return Ok(Expr::Ctx(which));
        }
        for (word, v) in [
            ("true", Some(Value::Bool(true))),
            ("false", Some(Value::Bool(false))),
            ("null", Some(Value::Null)),
            ("undefined", None),
        ] {
            if self.starts_with(word) && !self.peek(word.len()).is_some_and(is_word) {
                self.pos += word.len();
                return Ok(v.map_or(Expr::Undefined, Expr::Lit));
            }
        }
        self.fail("Expected a filter operand")
    }

    /// `^-?\d+(\.\d+)?([eE][-+]?\d+)?`
    fn number(&mut self) -> Option<f64> {
        let src = &self.src;
        let mut i = self.pos;
        if src.get(i) == Some(&u16::from(b'-')) {
            i += 1;
        }
        let d = i;
        while src.get(i).is_some_and(|u| is_digit(*u)) {
            i += 1;
        }
        if i == d {
            return None;
        }
        if src.get(i) == Some(&u16::from(b'.')) && src.get(i + 1).is_some_and(|u| is_digit(*u)) {
            i += 1;
            while src.get(i).is_some_and(|u| is_digit(*u)) {
                i += 1;
            }
        }
        if src
            .get(i)
            .is_some_and(|u| *u == u16::from(b'e') || *u == u16::from(b'E'))
        {
            let mut j = i + 1;
            if src
                .get(j)
                .is_some_and(|u| *u == u16::from(b'+') || *u == u16::from(b'-'))
            {
                j += 1;
            }
            let e = j;
            while src.get(j).is_some_and(|u| is_digit(*u)) {
                j += 1;
            }
            if j > e {
                i = j;
            }
        }
        let text = self.slice(self.pos, i);
        self.pos = i;
        Some(string_to_number(&text))
    }

    fn regex(&mut self) -> PResult<RegexLit> {
        self.expect("/")?;
        let mut body: Vec<u16> = Vec::new();
        let mut in_class = false;
        loop {
            if self.done() {
                return self.fail("Unterminated regular expression");
            }
            let c = self.src[self.pos];
            self.pos += 1;
            if c == u16::from(b'\\') {
                body.push(c);
                if let Some(n) = self.peek(0) {
                    body.push(n);
                }
                self.pos += 1;
                continue;
            }
            if c == u16::from(b'[') {
                in_class = true;
            }
            if c == u16::from(b']') {
                in_class = false;
            }
            if c == u16::from(b'/') && !in_class {
                break;
            }
            body.push(c);
        }
        let start = self.pos;
        while self
            .peek(0)
            .is_some_and(|u| u8::try_from(u).is_ok_and(|b| b"dgimsuvy".contains(&b)))
        {
            self.pos += 1;
        }
        let flags = self.slice(start, self.pos);
        let body = String::from_utf16_lossy(&body);
        let mut seen = HashSet::new();
        if !flags.chars().all(|c| seen.insert(c)) || (flags.contains('u') && flags.contains('v')) {
            return self.fail(&format!(
                "Invalid regular expression: Invalid flags supplied to RegExp constructor '{flags}'"
            ));
        }
        match regress::Regex::with_flags(&body, flags.as_str()) {
            Ok(re) => Ok(RegexLit {
                re,
                global: flags.contains('g'),
                sticky: flags.contains('y'),
            }),
            Err(e) => self.fail(&format!("Invalid regular expression: /{body}/{flags}: {e}")),
        }
    }

    fn postfix(&mut self, base: Expr) -> PResult<Expr> {
        let mut e = base;
        loop {
            if self.peek_is(0, '.') && !self.peek_is(1, '.') {
                self.pos += 1;
                let Some(name) = self.ident() else {
                    return self.fail("Expected a property name");
                };
                if self.peek_is(0, '(') {
                    let Some(&(_, method)) = METHODS.iter().find(|(n, _)| *n == name) else {
                        return self.fail(&format!("Unsupported method \"{name}\""));
                    };
                    self.expect("(")?;
                    let mut args = Vec::new();
                    self.ws();
                    if !self.eat(")") {
                        loop {
                            args.push(self.expr()?);
                            if !self.eat(",") {
                                break;
                            }
                        }
                        self.ws();
                        self.expect(")")?;
                    }
                    e = Expr::Call(Box::new(e), method, args);
                } else {
                    e = Expr::Get(Box::new(e), Key::Name(name));
                }
            } else if self.peek_is(0, '[') {
                self.pos += 1;
                self.ws();
                let key = if self.peek_is(0, '\'') || self.peek_is(0, '"') {
                    Key::Name(self.string()?)
                } else {
                    let Some(n) = self.opt_int() else {
                        return self.fail("Expected a string or index");
                    };
                    Key::Num(n)
                };
                self.ws();
                self.expect("]")?;
                e = Expr::Get(Box::new(e), key);
            } else {
                return Ok(e);
            }
        }
    }
}

/// Parse a JSONPath.
///
/// # Errors
///
/// On invalid or unsupported syntax.
pub fn parse_json_path(path: &str) -> Result<Vec<Segment>, JsonPathError> {
    Parser {
        src: path.encode_utf16().collect(),
        text: path.to_owned(),
        pos: 0,
    }
    .path()
}

// ---------------------------------------------------------------------------------------------
// Evaluation

/// A value a filter expression evaluates to.
#[derive(Debug, Clone)]
enum Ev<'a> {
    Undefined,
    /// A node of the document (compared by identity when it is an object or array).
    Ref(&'a Value),
    /// A value made during evaluation (literals, method results).
    Owned(Value),
    Regex(Rc<RegexLit>),
}

impl Ev<'_> {
    fn value(&self) -> Option<&Value> {
        match self {
            Ev::Ref(v) => Some(v),
            Ev::Owned(v) => Some(v),
            _ => None,
        }
    }

    fn truthy(&self) -> bool {
        match self {
            Ev::Undefined => false,
            Ev::Regex(_) => true,
            Ev::Ref(v) => v.truthy(),
            Ev::Owned(v) => v.truthy(),
        }
    }
}

struct Scope<'a> {
    current: &'a Value,
    property: Option<Seg>,
    parent: &'a Value,
    parent_property: Option<Seg>,
    root: &'a Value,
}

fn seg_value(s: Option<&Seg>) -> Ev<'static> {
    match s {
        None => Ev::Undefined,
        Some(Seg::Key(k)) => Ev::Owned(Value::String(k.clone())),
        #[expect(clippy::cast_precision_loss, reason = "array indices are far below 2^53")]
        Some(Seg::Index(i)) => Ev::Owned(Value::Number(*i as f64)),
    }
}

fn children<'a>(path: &[Seg], value: &'a Value) -> Vec<(Vec<Seg>, &'a Value)> {
    match value {
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let mut p = path.to_vec();
                p.push(Seg::Index(i));
                (p, v)
            })
            .collect(),
        Value::Object(m) => m
            .iter()
            .map(|(k, v)| {
                let mut p = path.to_vec();
                p.push(Seg::Key(k.clone()));
                (p, v)
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn self_and_descendants<'a>(path: Vec<Seg>, value: &'a Value, out: &mut Vec<(Vec<Seg>, &'a Value)>) {
    let kids = children(&path, value);
    out.push((path, value));
    for (p, v) in kids {
        self_and_descendants(p, v, out);
    }
}

/// `^\d+$`
fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn parse_index(s: &str) -> i64 {
    s.bytes()
        .fold(0i64, |n, b| n.saturating_mul(10).saturating_add(i64::from(b - b'0')))
}

fn array_child<'a>(path: &[Seg], items: &'a [Value], raw: i64) -> Option<(Vec<Seg>, &'a Value)> {
    let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
    let i = if raw < 0 { len + raw } else { raw };
    if i < 0 || i >= len {
        return None;
    }
    let i = usize::try_from(i).ok()?;
    let mut p = path.to_vec();
    p.push(Seg::Index(i));
    Some((p, &items[i]))
}

fn child_by_name<'a>(path: &[Seg], value: &'a Value, name: &str) -> Option<(Vec<Seg>, &'a Value)> {
    match value {
        Value::Array(items) => {
            if !all_digits(name) {
                return None;
            }
            array_child(path, items, parse_index(name))
        }
        Value::Object(m) => {
            let v = m.get(name)?;
            let mut p = path.to_vec();
            p.push(Seg::Key(name.to_owned()));
            Some((p, v))
        }
        _ => None,
    }
}

fn slice<'a>(
    path: &[Seg],
    value: &'a Value,
    start: Option<i64>,
    end: Option<i64>,
    step: i64,
) -> Vec<(Vec<Seg>, &'a Value)> {
    let Value::Array(items) = value else { return Vec::new() };
    let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
    let norm = |n: i64| if n < 0 { (len + n).max(0) } else { n.min(len) };
    let mut out = Vec::new();
    let mut push = |i: i64| {
        if let Ok(ix) = usize::try_from(i)
            && let Some(v) = items.get(ix)
        {
            let mut p = path.to_vec();
            p.push(Seg::Index(ix));
            out.push((p, v));
        }
    };
    if step > 0 {
        let end = end.map_or(len, norm);
        let mut i = start.map_or(0, norm);
        while i < end {
            push(i);
            i = i.saturating_add(step);
        }
    } else {
        let start = start.map_or(len - 1, |s| norm(s).min(len - 1));
        let end = end.map_or(-1, norm);
        let stride = step.saturating_neg();
        let mut i = start;
        while i > end {
            push(i);
            i = i.saturating_sub(stride);
        }
    }
    out
}

fn apply<'a>(
    path: &[Seg],
    value: &'a Value,
    selector: &Selector,
    root: &'a Value,
    out: &mut Vec<(Vec<Seg>, &'a Value)>,
) {
    match selector {
        Selector::Name(name) => out.extend(child_by_name(path, value, name)),
        Selector::Index(i) => {
            if let Value::Array(items) = value {
                out.extend(array_child(path, items, *i));
            }
        }
        Selector::Wildcard => out.extend(children(path, value)),
        Selector::Slice { start, end, step } => out.extend(slice(path, value, *start, *end, *step)),
        Selector::Filter(expr) => {
            for (p, v) in children(path, value) {
                let scope = Scope {
                    current: v,
                    property: p.last().cloned(),
                    parent: value,
                    parent_property: path.last().cloned(),
                    root,
                };
                if evaluate(expr, &scope).truthy() {
                    out.push((p, v));
                }
            }
        }
    }
}

fn utf16_units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn get<'a>(obj: &Ev<'a>, key: &Key) -> Ev<'a> {
    let (numeric, key_str) = match key {
        Key::Num(n) => (Some(*n), n.to_string()),
        Key::Name(k) => (None, k.clone()),
    };
    let index_of = |len: usize| -> Option<usize> {
        let raw = match numeric {
            Some(n) => n,
            None if all_digits(&key_str) => parse_index(&key_str),
            None => return None,
        };
        let len_i = i64::try_from(len).unwrap_or(i64::MAX);
        let i = if raw < 0 { len_i + raw } else { raw };
        usize::try_from(i).ok()
    };
    match obj {
        Ev::Ref(Value::Array(items)) => {
            if key_str == "length" && numeric.is_none() {
                #[expect(clippy::cast_precision_loss, reason = "array lengths are far below 2^53")]
                return Ev::Owned(Value::Number(items.len() as f64));
            }
            index_of(items.len())
                .and_then(|i| items.get(i))
                .map_or(Ev::Undefined, Ev::Ref)
        }
        Ev::Owned(Value::Array(items)) => {
            if key_str == "length" && numeric.is_none() {
                #[expect(clippy::cast_precision_loss, reason = "array lengths are far below 2^53")]
                return Ev::Owned(Value::Number(items.len() as f64));
            }
            index_of(items.len())
                .and_then(|i| items.get(i))
                .map_or(Ev::Undefined, |v| Ev::Owned(v.clone()))
        }
        Ev::Ref(Value::String(s)) | Ev::Owned(Value::String(s)) => {
            let units = utf16_units(s);
            if key_str == "length" && numeric.is_none() {
                #[expect(clippy::cast_precision_loss, reason = "string lengths are far below 2^53")]
                return Ev::Owned(Value::Number(units.len() as f64));
            }
            index_of(units.len())
                .and_then(|i| units.get(i))
                .map_or(Ev::Undefined, |u| {
                    Ev::Owned(Value::String(String::from_utf16_lossy(&[*u])))
                })
        }
        Ev::Ref(Value::Object(m)) => m.get(&key_str).map_or(Ev::Undefined, Ev::Ref),
        Ev::Owned(Value::Object(m)) => m.get(&key_str).map_or(Ev::Undefined, |v| Ev::Owned(v.clone())),
        _ => Ev::Undefined,
    }
}

fn regex_find(re: &RegexLit, s: &str) -> Option<regress::Match> {
    let m = re.re.find(s)?;
    if re.sticky && m.start() != 0 {
        return None;
    }
    Some(m)
}

fn string_match(s: &str, re: &RegexLit) -> Value {
    if re.global {
        let found: Vec<Value> = re
            .re
            .find_iter(s)
            .filter(|m| !re.sticky || m.start() == 0)
            .map(|m| Value::String(s[m.range()].to_owned()))
            .collect();
        return if found.is_empty() {
            Value::Null
        } else {
            Value::Array(found)
        };
    }
    match regex_find(re, s) {
        None => Value::Null,
        Some(m) => Value::Array(
            m.groups()
                .map(|g| g.map_or(Value::Null, |r| Value::String(s[r].to_owned())))
                .collect(),
        ),
    }
}

fn call<'a>(obj: &Ev<'a>, method: Method, args: &[Ev<'a>]) -> Ev<'a> {
    let a = args.first().cloned().unwrap_or(Ev::Undefined);
    if let Ev::Regex(re) = obj {
        return match (method, a.value()) {
            (Method::Test, Some(Value::String(s))) => Ev::Owned(Value::Bool(regex_find(re, s).is_some())),
            _ => Ev::Undefined,
        };
    }
    let Some(v) = obj.value() else { return Ev::Undefined };
    if let Value::Array(items) = v {
        if method != Method::Includes {
            return Ev::Undefined;
        }
        let found = items.iter().any(|x| same_value_zero(&a, x));
        return Ev::Owned(Value::Bool(found));
    }
    let Value::String(s) = v else { return Ev::Undefined };
    let arg_str = a.value().and_then(Value::as_str);
    match method {
        Method::Includes => arg_str.map_or(Ev::Undefined, |x| Ev::Owned(Value::Bool(s.contains(x)))),
        Method::StartsWith => arg_str.map_or(Ev::Undefined, |x| Ev::Owned(Value::Bool(s.starts_with(x)))),
        Method::EndsWith => arg_str.map_or(Ev::Undefined, |x| Ev::Owned(Value::Bool(s.ends_with(x)))),
        Method::Match => match &a {
            Ev::Regex(re) => Ev::Owned(string_match(s, re)),
            other => match other.value() {
                Some(Value::String(p)) => match regress::Regex::new(p) {
                    Ok(re) => Ev::Owned(string_match(
                        s,
                        &RegexLit {
                            re,
                            global: false,
                            sticky: false,
                        },
                    )),
                    Err(_) => Ev::Undefined,
                },
                _ => Ev::Undefined,
            },
        },
        Method::ToLowerCase => Ev::Owned(Value::String(s.to_lowercase())),
        Method::ToUpperCase => Ev::Owned(Value::String(s.to_uppercase())),
        Method::Trim => Ev::Owned(Value::String(s.trim_matches(is_js_space).to_owned())),
        Method::Test => Ev::Undefined,
    }
}

/// `Array.prototype.includes` (`SameValueZero`) of a filter value in an array of nodes.
fn same_value_zero(a: &Ev<'_>, x: &Value) -> bool {
    match a {
        Ev::Undefined | Ev::Regex(_) => false,
        Ev::Ref(v) => match (v, x) {
            (Value::Object(_) | Value::Array(_), _) => std::ptr::eq(*v, x),
            _ => prim_eq(v, x, true),
        },
        Ev::Owned(v) => match v {
            Value::Object(_) | Value::Array(_) => false,
            _ => prim_eq(v, x, true),
        },
    }
}

fn prim_eq(a: &Value, b: &Value, nan_equal: bool) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y || (nan_equal && x.is_nan() && y.is_nan()),
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

/// JavaScript `===`.
fn strict_equals(a: &Ev<'_>, b: &Ev<'_>) -> bool {
    match (a, b) {
        (Ev::Undefined, Ev::Undefined) => true,
        (Ev::Regex(x), Ev::Regex(y)) => Rc::ptr_eq(x, y),
        (Ev::Ref(x), Ev::Ref(y)) if matches!(x, Value::Object(_) | Value::Array(_)) => std::ptr::eq(*x, *y),
        (Ev::Owned(Value::Object(_) | Value::Array(_)), _) | (_, Ev::Owned(Value::Object(_) | Value::Array(_))) => {
            false
        }
        _ => match (a.value(), b.value()) {
            (Some(x), Some(y)) => prim_eq(x, y, false),
            _ => false,
        },
    }
}

fn to_number(v: &Value) -> f64 {
    match v {
        Value::Number(n) => *n,
        Value::String(s) => string_to_number(s),
        Value::Bool(b) => f64::from(u8::from(*b)),
        _ => f64::NAN,
    }
}

/// JavaScript `==` restricted to the primitive values JSON produces.
fn loose_equals(a: &Ev<'_>, b: &Ev<'_>) -> bool {
    if strict_equals(a, b) {
        return true;
    }
    let nullish = |e: &Ev<'_>| matches!(e, Ev::Undefined) || matches!(e.value(), Some(Value::Null));
    if nullish(a) || nullish(b) {
        return nullish(a) && nullish(b);
    }
    match (a.value(), b.value()) {
        (
            Some(x @ (Value::Number(_) | Value::String(_) | Value::Bool(_))),
            Some(y @ (Value::Number(_) | Value::String(_) | Value::Bool(_))),
        ) => {
            #[expect(clippy::float_cmp, reason = "JavaScript === on numbers")]
            let eq = to_number(x) == to_number(y);
            eq
        }
        _ => false,
    }
}

fn compare(op: Op, a: &Ev<'_>, b: &Ev<'_>) -> bool {
    let ord = match (a.value(), b.value()) {
        (Some(Value::Number(x)), Some(Value::Number(y))) => x.partial_cmp(y),
        (Some(Value::String(x)), Some(Value::String(y))) => Some(cmp_utf16(x, y)),
        _ => return false,
    };
    let Some(ord) = ord else { return false };
    match op {
        Op::Lt => ord == Ordering::Less,
        Op::Le => ord != Ordering::Greater,
        Op::Gt => ord == Ordering::Greater,
        _ => ord != Ordering::Less,
    }
}

fn evaluate<'a>(e: &Expr, scope: &Scope<'a>) -> Ev<'a> {
    match e {
        Expr::Undefined => Ev::Undefined,
        Expr::Lit(v) => Ev::Owned(v.clone()),
        Expr::Regex(r) => Ev::Regex(Rc::clone(r)),
        Expr::Ctx(c) => match c {
            Ctx::Current => Ev::Ref(scope.current),
            Ctx::Parent => Ev::Ref(scope.parent),
            Ctx::Root => Ev::Ref(scope.root),
            Ctx::Property => seg_value(scope.property.as_ref()),
            Ctx::ParentProperty => seg_value(scope.parent_property.as_ref()),
        },
        Expr::Get(obj, key) => get(&evaluate(obj, scope), key),
        Expr::Call(obj, method, args) => {
            let o = evaluate(obj, scope);
            let args: Vec<Ev<'a>> = args.iter().map(|a| evaluate(a, scope)).collect();
            call(&o, *method, &args)
        }
        Expr::Not(inner) => Ev::Owned(Value::Bool(!evaluate(inner, scope).truthy())),
        Expr::Bin(op, l, r) => match op {
            Op::And => Ev::Owned(Value::Bool(evaluate(l, scope).truthy() && evaluate(r, scope).truthy())),
            Op::Or => Ev::Owned(Value::Bool(evaluate(l, scope).truthy() || evaluate(r, scope).truthy())),
            _ => {
                let lv = evaluate(l, scope);
                let rv = evaluate(r, scope);
                let b = match op {
                    Op::StrictEq => strict_equals(&lv, &rv),
                    Op::StrictNe => !strict_equals(&lv, &rv),
                    Op::Eq => loose_equals(&lv, &rv),
                    Op::Ne => !loose_equals(&lv, &rv),
                    _ => compare(*op, &lv, &rv),
                };
                Ev::Owned(Value::Bool(b))
            }
        },
    }
}

/// Evaluate parsed segments against `root`. Results are de-duplicated, in selection order.
#[must_use]
pub fn evaluate_segments<'a>(root: &'a Value, segments: &[Segment]) -> Vec<Match<'a>> {
    let mut nodes: Vec<(Vec<Seg>, &'a Value)> = vec![(Vec::new(), root)];
    for segment in segments {
        let mut next = Vec::new();
        for (path, value) in nodes {
            let mut bases = Vec::new();
            if segment.descendant {
                self_and_descendants(path, value, &mut bases);
            } else {
                bases.push((path, value));
            }
            for (bp, bv) in &bases {
                for selector in &segment.selectors {
                    apply(bp, bv, selector, root, &mut next);
                }
            }
        }
        nodes = next;
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (path, value) in nodes {
        if seen.insert(path.clone()) {
            out.push(Match { path, value });
        }
    }
    out
}

/// Parse and evaluate `path` against `root`.
///
/// # Errors
///
/// On bad syntax.
pub fn query<'a>(root: &'a Value, path: &str) -> Result<Vec<Match<'a>>, JsonPathError> {
    Ok(evaluate_segments(root, &parse_json_path(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc() -> Value {
        Value::from(json!({
            "info": {"title": "T", "x.dotted": 1},
            "paths": {
                "/users/{id}": {
                    "get": {
                        "parameters": [
                            {"name": "id", "in": "path", "schema": {"type": "integer"}},
                            {"name": "limit", "in": "query", "deprecated": true, "schema": {"type": "integer"}},
                            {"name": "Cursor", "in": "query", "required": false}
                        ],
                        "responses": {"404": {"content": {"application/json": {"schema": {"type": "object"}}}}}
                    }
                }
            },
            "list": [10, 20, 30, 40, 50]
        }))
    }

    fn values(p: &str) -> Vec<Value> {
        let d = doc();
        query(&d, p).unwrap().into_iter().map(|m| m.value.clone()).collect()
    }

    fn paths(p: &str) -> Vec<Vec<Seg>> {
        let d = doc();
        query(&d, p).unwrap().into_iter().map(|m| m.path).collect()
    }

    fn vals(v: &serde_json::Value) -> Vec<Value> {
        v.as_array().unwrap().iter().map(Value::from).collect()
    }

    #[test]
    fn selects_the_root() {
        let d = doc();
        let m = query(&d, "$").unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].path, Vec::<Seg>::new());
        assert_eq!(m[0].value, &d);
    }

    #[test]
    fn selects_dot_names_that_contain_slashes_braces_and_digits() {
        assert_eq!(
            values("$.paths./users/{id}.get.responses.404.content['application/json'].schema.type"),
            vals(&json!(["object"]))
        );
        assert_eq!(
            values("$.paths./users/{id}.get.responses.404.content.application/json.schema.type"),
            vals(&json!(["object"]))
        );
    }

    #[test]
    fn selects_bracketed_names_quotes_and_escapes() {
        assert_eq!(values("$.info['x.dotted']"), vals(&json!([1])));
        assert_eq!(values("$.info[\"title\"]"), vals(&json!(["T"])));
        assert_eq!(values("$['info']['ti\\u0074le']"), vals(&json!(["T"])));
        assert_eq!(values("$[info][title]"), vals(&json!(["T"])));
        assert_eq!(values("$.info['a\\'b']"), vals(&json!([])));
        assert_eq!(values("$.info['\\n\\t']"), vals(&json!([])));
    }

    #[test]
    fn selects_indices_negative_indices_unions_wildcards_and_slices() {
        assert_eq!(values("$.list[0]"), vals(&json!([10])));
        assert_eq!(values("$.list.1"), vals(&json!([20])));
        assert_eq!(values("$.list[-1]"), vals(&json!([50])));
        assert_eq!(values("$.list[9]"), vals(&json!([])));
        assert_eq!(values("$.list.x"), vals(&json!([])));
        assert_eq!(values("$.list[0,2]"), vals(&json!([10, 30])));
        assert_eq!(values("$.list[1:3]"), vals(&json!([20, 30])));
        assert_eq!(values("$.list[::2]"), vals(&json!([10, 30, 50])));
        assert_eq!(values("$.list[-2:]"), vals(&json!([40, 50])));
        assert_eq!(values("$.list[::-1]"), vals(&json!([50, 40, 30, 20, 10])));
        assert_eq!(values("$.list[3:0:-2]"), vals(&json!([40, 20])));
        assert_eq!(values("$.info[*]"), vals(&json!(["T", 1])));
        assert_eq!(values("$.info.*"), vals(&json!(["T", 1])));
        assert_eq!(values("$.info[0:1]"), vals(&json!([])));
        assert_eq!(values("$.info['title','missing']"), vals(&json!(["T"])));
        assert_eq!(values("$.info.title[0]"), vals(&json!([])));
    }

    #[test]
    fn recursive_descent_visits_every_level_deduplicated() {
        assert_eq!(values("$..type"), vals(&json!(["integer", "integer", "object"])));
        assert_eq!(paths("$..parameters[*].name").len(), 3);
        let d = doc();
        assert!(query(&d, "$..*").unwrap().len() > 20);
        assert_eq!(values("$..[\"title\"]"), vals(&json!(["T"])));
        assert_eq!(
            paths("$.info['title','title']"),
            vec![vec![Seg::from("info"), Seg::from("title")]]
        );
    }

    #[test]
    fn filters_with_comparisons_and_boolean_logic() {
        assert_eq!(values("$..parameters[?(@.name == \"id\")].in"), vals(&json!(["path"])));
        assert_eq!(
            values("$..parameters[?(@.in === 'query' && !@.deprecated)].name"),
            vals(&json!(["Cursor"]))
        );
        assert_eq!(
            values("$..parameters[?(@.deprecated == true || @.name != \"limit\")].name"),
            vals(&json!(["id", "limit", "Cursor"]))
        );
        assert_eq!(
            values("$..parameters[?(@.required === false)].name"),
            vals(&json!(["Cursor"]))
        );
        assert_eq!(
            values("$..parameters[?(@.required !== undefined)].name"),
            vals(&json!(["Cursor"]))
        );
        assert_eq!(values("$..parameters[?(@.deprecated)].name"), vals(&json!(["limit"])));
        assert_eq!(values("$..parameters[?@.name==\"id\"].in"), vals(&json!(["path"])));
        assert_eq!(values("$.list[?(@ > 20 && @ <= 40)]"), vals(&json!([30, 40])));
        assert_eq!(values("$.list[?(@ >= 40)]"), vals(&json!([40, 50])));
        assert_eq!(values("$.list[?(@ < 20)]"), vals(&json!([10])));
        assert_eq!(values("$.list[?(@ == \"10\")]"), vals(&json!([10])));
        assert_eq!(values("$.list[?(@ < \"z\")]"), vals(&json!([])));
        assert_eq!(values("$.list[?(@ === -1.5e0)]"), vals(&json!([])));
        assert_eq!(values("$.list[?((@ == 10) || (@ == 50))]"), vals(&json!([10, 50])));
        assert_eq!(values("$.list[?(@ == null)]"), vals(&json!([])));
        assert_eq!(values("$.list[?(null == undefined)]").len(), 5);
        assert_eq!(values("$.list[?(@ == false)]"), vals(&json!([])));
    }

    #[test]
    fn filters_over_object_values_and_exposes_property_parent_and_root() {
        assert_eq!(
            paths("$.paths[?(@property.startsWith(\"/users\"))]"),
            vec![vec![Seg::from("paths"), Seg::from("/users/{id}")]]
        );
        assert_eq!(values("$.info[?(@property === \"title\")]"), vals(&json!(["T"])));
        assert_eq!(
            values("$.paths.*.get.parameters[?(@parentProperty === \"parameters\")].name").len(),
            3
        );
        assert_eq!(values("$.paths.*.get.parameters[?(@parent.length == 3)].name").len(), 3);
        assert_eq!(
            values("$.list[?(@root.info.title == \"T\" && $.list[0] == 10)]").len(),
            5
        );
        assert_eq!(values("$.info[?(@root['info'].title == 'T')]"), vals(&json!(["T", 1])));
    }

    #[test]
    fn supports_string_array_and_regex_methods() {
        assert_eq!(
            values("$..parameters[?(@.name.match(/^cur/i))].name"),
            vals(&json!(["Cursor"]))
        );
        assert_eq!(
            values("$..parameters[?(@.name.match(\"^l\"))].name"),
            vals(&json!(["limit"]))
        );
        assert_eq!(values("$..parameters[?(/[a-z]+/.test(@.in))].name").len(), 3);
        assert_eq!(
            values("$..parameters[?(@.name.toLowerCase() == \"cursor\")].name"),
            vals(&json!(["Cursor"]))
        );
        assert_eq!(
            values("$..parameters[?(@.name.toUpperCase() == \"ID\")].name"),
            vals(&json!(["id"]))
        );
        assert_eq!(
            values("$..parameters[?(@.name.trim().endsWith(\"it\"))].name"),
            vals(&json!(["limit"]))
        );
        assert_eq!(
            values("$..parameters[?(@.name.includes(\"mi\"))].name"),
            vals(&json!(["limit"]))
        );
        assert_eq!(
            values("$.paths.*.get[?(@property == \"parameters\" && @.length == 3)]").len(),
            1
        );
        assert_eq!(values("$[?(@.includes(30))]"), vals(&json!([[10, 20, 30, 40, 50]])));
        assert_eq!(values("$[?(@.includes)]"), vals(&json!([])));
        assert_eq!(values("$..parameters[?(@.schema.startsWith(\"x\"))]"), vals(&json!([])));
        assert_eq!(
            values("$..parameters[?(@.name.startsWith(1) || @.name.endsWith(1))]"),
            vals(&json!([]))
        );
        assert_eq!(
            values("$..parameters[?(@.name.includes(1) || @.name.match(1))]"),
            vals(&json!([]))
        );
        assert_eq!(values("$..parameters[?(/x/.test(1) || /x/.trim())]"), vals(&json!([])));
        assert_eq!(values("$.list[?(@.length)]"), vals(&json!([])));
        assert_eq!(
            values("$..parameters[?(@.name[0] == \"C\" || @.name[-1] == \"d\")].name"),
            vals(&json!(["id", "Cursor"]))
        );
        assert_eq!(values("$..parameters[?(@.name[\"x\"] == 1)]"), vals(&json!([])));
    }

    #[test]
    fn rejects_invalid_or_unsupported_paths() {
        for p in [
            "info",
            "$.",
            "$.info.",
            "$..",
            "$x",
            "$.info^",
            "$.info~",
            "$.@string()",
            "$[(@.length-1)]",
            "$[",
            "$['open",
            "$[?(@.a ==)]",
            "$[?(@.a.unknown())]",
            "$[?(@foo)]",
            "$[?(/(/.test(@))]",
            "$[?(/abc)]",
            "$[::0]",
            "$[:x]",
            "$[?(@.a[x])]",
            "$[?(@.)]",
            "$['\\u12']",
            "$[?(@.a.includes(1, 2)]",
            "$[@]",
        ] {
            assert!(parse_json_path(p).is_err(), "{p}");
        }
    }

    #[test]
    fn accepts_whitespace_and_multi_argument_calls() {
        assert_eq!(values(" $ .info [ \"title\" ] "), vals(&json!(["T"])));
        assert_eq!(values("$.list[?( @ == 10 )]"), vals(&json!([10])));
        assert_eq!(
            values("$..parameters[?(@.name.includes(\"i\", 0))].name"),
            vals(&json!(["id", "limit"]))
        );
        assert_eq!(values("$.list[?(/[/]/.test(\"a/b\") && /\\d/.test(\"1\"))]").len(), 5);
    }

    #[test]
    fn error_messages_carry_the_offset_and_path() {
        assert_eq!(
            parse_json_path("info").unwrap_err().0,
            "A JSONPath must start with \"$\" at offset 0 in \"info\""
        );
        assert_eq!(
            parse_json_path("$x").unwrap_err().0,
            "Unexpected \"x\" at offset 1 in \"$x\""
        );
    }
}
