//! A JSON value with JavaScript semantics, so the port reproduces the TypeScript converter byte
//! for byte: numbers are `f64`, and object keys iterate the way `Object.keys` does (array-index
//! keys first in ascending numeric order, then the other keys in insertion order).

use indexmap::IndexMap;
use std::cmp::Ordering;
use std::fmt::Write as _;

/// A JSON value as JavaScript holds it.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Map),
}

/// A JavaScript object's own enumerable properties, in `Object.keys` order.
#[derive(Debug, Clone, Default)]
pub struct Map(IndexMap<String, Value>);

/// Two maps are equal when they hold the same keys and values, in any order (`toEqual`).
impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// The numeric value of an array-index key (`"0"`, `"42"`, at most 2^32 - 2), the keys
/// JavaScript orders first.
#[must_use]
pub fn array_index(key: &str) -> Option<u32> {
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > 10 || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if bytes.len() > 1 && bytes[0] == b'0' {
        return None;
    }
    let n: u64 = key.parse().ok()?;
    if n > 4_294_967_294 {
        return None;
    }
    u32::try_from(n).ok()
}

impl Map {
    #[must_use]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    /// Set `key`, keeping the position of an existing key (JavaScript assignment).
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        let key = key.into();
        if let Some(slot) = self.0.get_mut(&key) {
            return Some(std::mem::replace(slot, value));
        }
        if let Some(index) = array_index(&key) {
            let pos = self
                .0
                .keys()
                .position(|k| array_index(k).is_none_or(|j| j > index))
                .unwrap_or(self.0.len());
            self.0.shift_insert(pos, key, value);
        } else {
            self.0.insert(key, value);
        }
        None
    }

    /// `delete obj[key]`: the other keys keep their order.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.shift_remove(key)
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn iter(&self) -> indexmap::map::Iter<'_, String, Value> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<'_, String, Value> {
        self.0.iter_mut()
    }

    #[must_use]
    pub fn keys(&self) -> indexmap::map::Keys<'_, String, Value> {
        self.0.keys()
    }

    #[must_use]
    pub fn values(&self) -> indexmap::map::Values<'_, String, Value> {
        self.0.values()
    }

    pub fn values_mut(&mut self) -> indexmap::map::ValuesMut<'_, String, Value> {
        self.0.values_mut()
    }

    /// Remove every key.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = (&'a String, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, String, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Map {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = indexmap::map::IterMut<'a, String, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl IntoIterator for Map {
    type Item = (String, Value);
    type IntoIter = indexmap::map::IntoIter<String, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, Value)> for Map {
    /// `Object.fromEntries`: later duplicates overwrite earlier ones in place.
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let mut map = Map::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl Value {
    #[must_use]
    pub fn as_object(&self) -> Option<&Map> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Map> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Member `key` of an object.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object().and_then(|m| m.get(key))
    }

    /// `typeof` as the converter's messages name it: `array` and `null` are their own.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// JavaScript truthiness.
    #[must_use]
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::Array(_) | Value::Object(_) => true,
        }
    }

    /// An empty object.
    #[must_use]
    pub fn object() -> Value {
        Value::Object(Map::new())
    }

    /// A string value.
    #[must_use]
    pub fn str(s: impl Into<String>) -> Value {
        Value::String(s.into())
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<Map> for Value {
    fn from(m: Map) -> Self {
        Value::Object(m)
    }
}

impl From<Vec<Value>> for Value {
    fn from(a: Vec<Value>) -> Self {
        Value::Array(a)
    }
}

impl From<&serde_json::Value> for Value {
    /// From `serde_json` (built with `preserve_order`), so tests can use `json!`.
    fn from(v: &serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Array(a) => Value::Array(a.iter().map(Value::from).collect()),
            serde_json::Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), Value::from(v))).collect()),
        }
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        Value::from(&v)
    }
}

/// Compare strings by UTF-16 code units, as JavaScript's default sort and `<` do.
#[must_use]
pub fn cmp_utf16(a: &str, b: &str) -> Ordering {
    if a.is_ascii() && b.is_ascii() {
        return a.cmp(b);
    }
    a.encode_utf16().cmp(b.encode_utf16())
}

/// UTF-16 length of a string (JavaScript `.length`).
#[must_use]
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Recursively sort object keys so equal values serialize to identical bytes (`sortKeys`).
#[must_use]
pub fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Array(a) => Value::Array(a.iter().map(sort_keys).collect()),
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort_by(|a, b| cmp_utf16(a, b));
            let mut out = Map::new();
            for k in keys {
                out.insert(k.clone(), sort_keys(&m.0[k.as_str()]));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Deterministic JSON: sorted keys, 2-space indent, trailing newline (`stableStringify`).
#[must_use]
pub fn stable_stringify(value: &Value) -> String {
    let mut out = String::new();
    write_pretty(&sort_keys(value), 0, &mut out);
    out.push('\n');
    out
}

/// `JSON.stringify(value, null, 2)`.
#[must_use]
pub fn to_json_pretty(value: &Value) -> String {
    let mut out = String::new();
    write_pretty(value, 0, &mut out);
    out
}

/// `JSON.stringify(value)`.
#[must_use]
pub fn to_json(value: &Value) -> String {
    let mut out = String::new();
    write_compact(value, &mut out);
    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn write_pretty(value: &Value, level: usize, out: &mut String) {
    match value {
        Value::Array(a) if !a.is_empty() => {
            out.push_str("[\n");
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(out, level + 1);
                write_pretty(v, level + 1, out);
            }
            out.push('\n');
            indent(out, level);
            out.push(']');
        }
        Value::Object(m) if !m.is_empty() => {
            out.push_str("{\n");
            for (i, (k, v)) in m.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(out, level + 1);
                quote_into(k, out);
                out.push_str(": ");
                write_pretty(v, level + 1, out);
            }
            out.push('\n');
            indent(out, level);
            out.push('}');
        }
        other => write_compact(other, out),
    }
}

fn write_compact(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.is_finite() {
                out.push_str(&number_to_string(*n));
            } else {
                out.push_str("null");
            }
        }
        Value::String(s) => quote_into(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_compact(v, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (i, (k, v)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                quote_into(k, out);
                out.push(':');
                write_compact(v, out);
            }
            out.push('}');
        }
    }
}

/// `JSON.stringify` of a string.
#[must_use]
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    quote_into(s, &mut out);
    out
}

fn quote_into(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `Number.prototype.toString()` for a finite or non-finite `f64`.
#[must_use]
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if n == 0.0 {
        return "0".to_owned();
    }
    // Shortest round-trip digits, from Rust's exponent formatting: "d.ddde[-]x".
    let sci = format!("{:e}", n.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exp: i64 = exp.parse().unwrap_or(0);
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let point = exp + 1; // position of the decimal point relative to the digits
    let mut out = String::new();
    if n < 0.0 {
        out.push('-');
    }
    if k <= point && point <= 21 {
        out.push_str(&digits);
        for _ in 0..(point - k) {
            out.push('0');
        }
    } else if 0 < point && point <= 21 {
        let (int, frac) = digits.split_at(usize::try_from(point).unwrap_or(0));
        out.push_str(int);
        out.push('.');
        out.push_str(frac);
    } else if -6 < point && point <= 0 {
        out.push_str("0.");
        for _ in 0..(-point) {
            out.push('0');
        }
        out.push_str(&digits);
    } else {
        let e = point - 1;
        let (first, rest) = digits.split_at(1);
        out.push_str(first);
        if !rest.is_empty() {
            out.push('.');
            out.push_str(rest);
        }
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        let _ = write!(out, "{}", e.abs());
    }
    out
}

/// JavaScript `String(value)` for the values JSON holds.
#[must_use]
pub fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(*n),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|v| if v.is_null() { String::new() } else { js_string(v) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// JavaScript whitespace (`\s`, and what `String.prototype.trim` strips).
#[must_use]
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{a}' | '\u{b}' | '\u{c}' | '\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// JavaScript `Number(string)`.
#[must_use]
pub fn string_to_number(s: &str) -> f64 {
    let t = s.trim_matches(is_js_space);
    if t.is_empty() {
        return 0.0;
    }
    let radix = |p: &str, r: u32| -> f64 {
        if p.is_empty() || !p.chars().all(|c| c.is_digit(r)) {
            return f64::NAN;
        }
        p.chars()
            .fold(0.0, |acc, c| acc * f64::from(r) + f64::from(c.to_digit(r).unwrap_or(0)))
    };
    if let Some(p) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return radix(p, 16);
    }
    if let Some(p) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return radix(p, 8);
    }
    if let Some(p) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return radix(p, 2);
    }
    let (sign, body) = match t.as_bytes()[0] {
        b'-' => (-1.0, &t[1..]),
        b'+' => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    if is_decimal_literal(body) {
        body.parse::<f64>().map_or(f64::NAN, |n| sign * n)
    } else {
        f64::NAN
    }
}

/// `StrUnsignedDecimalLiteral`: digits with an optional fraction and exponent.
fn is_decimal_literal(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    let int_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut digits = i > int_start;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        digits |= i > frac_start;
    }
    if !digits {
        return false;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            return false;
        }
    }
    i == b.len()
}

/// Strict JSON text to a [`Value`], or `None` when the text is not plain JSON or repeats a key
/// (the caller then parses it as YAML, which also reports duplicates).
#[must_use]
pub fn parse_json(text: &str) -> Option<Value> {
    let mut p = JsonParser {
        s: text.as_bytes(),
        i: 0,
        text,
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    (p.i == p.s.len()).then_some(v)
}

struct JsonParser<'a> {
    s: &'a [u8],
    i: usize,
    text: &'a str,
}

impl JsonParser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Option<Value> {
        if depth > 10_000 {
            return None;
        }
        match *self.s.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut map = Map::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Some(Value::Object(map));
                }
                loop {
                    self.ws();
                    if self.s.get(self.i) != Some(&b'"') {
                        return None;
                    }
                    let key = self.string()?;
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') {
                        return None;
                    }
                    self.i += 1;
                    self.ws();
                    let v = self.value(depth + 1)?;
                    if map.insert(key, v).is_some() {
                        return None;
                    }
                    self.ws();
                    match self.s.get(self.i)? {
                        b',' => self.i += 1,
                        b'}' => {
                            self.i += 1;
                            return Some(Value::Object(map));
                        }
                        _ => return None,
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut arr = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Some(Value::Array(arr));
                }
                loop {
                    self.ws();
                    arr.push(self.value(depth + 1)?);
                    self.ws();
                    match self.s.get(self.i)? {
                        b',' => self.i += 1,
                        b']' => {
                            self.i += 1;
                            return Some(Value::Array(arr));
                        }
                        _ => return None,
                    }
                }
            }
            b'"' => self.string().map(Value::String),
            b't' => self.word("true", Value::Bool(true)),
            b'f' => self.word("false", Value::Bool(false)),
            b'n' => self.word("null", Value::Null),
            _ => self.number(),
        }
    }

    fn word(&mut self, w: &str, v: Value) -> Option<Value> {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Some(v)
        } else {
            None
        }
    }

    fn number(&mut self) -> Option<Value> {
        let start = self.i;
        if self.s.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        match self.s.get(self.i)? {
            b'0' => self.i += 1,
            b'1'..=b'9' => {
                while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                    self.i += 1;
                }
            }
            _ => return None,
        }
        if self.s.get(self.i) == Some(&b'.') {
            self.i += 1;
            let f = self.i;
            while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            if self.i == f {
                return None;
            }
        }
        if matches!(self.s.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.s.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            let f = self.i;
            while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            if self.i == f {
                return None;
            }
        }
        self.text[start..self.i].parse::<f64>().ok().map(Value::Number)
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = self.text.get(self.i..self.i + 4)?;
        let n = u32::from_str_radix(h, 16).ok()?;
        if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        self.i += 4;
        Some(n)
    }

    fn string(&mut self) -> Option<String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while self.i < self.s.len() && self.s[self.i] != b'"' && self.s[self.i] != b'\\' && self.s[self.i] >= 0x20 {
                self.i += 1;
            }
            out.push_str(&self.text[start..self.i]);
            match *self.s.get(self.i)? {
                b'"' => {
                    self.i += 1;
                    return Some(out);
                }
                b'\\' => {
                    self.i += 1;
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            if (0xD800..0xDC00).contains(&hi) {
                                // A pair is required: lone surrogates have no UTF-8 form.
                                if self.s.get(self.i) != Some(&b'\\') || self.s.get(self.i + 1) != Some(&b'u') {
                                    return None;
                                }
                                self.i += 2;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return None;
                                }
                                out.push(char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00))?);
                            } else {
                                out.push(char::from_u32(hi)?);
                            }
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_format_like_javascript() {
        let cases = [
            (1.0, "1"),
            (-0.0, "0"),
            (0.5, "0.5"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (123_456_789_012_345_680_000.0, "123456789012345680000"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (1.5e-7, "1.5e-7"),
            (-2.5, "-2.5"),
            (9_007_199_254_740_993.0, "9007199254740992"),
            (0.1 + 0.2, "0.30000000000000004"),
            (5e-324, "5e-324"),
            (1.797_693_134_862_315_7e308, "1.7976931348623157e+308"),
            (12.0, "12"),
            (100.0, "100"),
        ];
        for (n, s) in cases {
            assert_eq!(number_to_string(n), s, "{n}");
        }
    }

    #[test]
    fn integer_keys_come_first() {
        let mut m = Map::new();
        for k in ["b", "10", "a", "2", "01", "4294967295", "4294967294"] {
            m.insert(k, Value::Null);
        }
        let keys: Vec<&str> = m.keys().map(String::as_str).collect();
        assert_eq!(keys, ["2", "10", "4294967294", "b", "a", "01", "4294967295"]);
    }

    #[test]
    fn sort_keys_uses_utf16_order_after_index_keys() {
        let mut m = Map::new();
        for k in ["\u{ff61}", "\u{1f600}", "$ref", "200", "a"] {
            m.insert(k, Value::Null);
        }
        let out = stable_stringify(&Value::Object(m));
        assert_eq!(
            out,
            "{\n  \"200\": null,\n  \"$ref\": null,\n  \"a\": null,\n  \"\u{1f600}\": null,\n  \"\u{ff61}\": null\n}\n"
        );
    }

    #[test]
    fn strings_escape_like_json_stringify() {
        assert_eq!(quote("a\"\\\n\u{1}\u{7f}é"), "\"a\\\"\\\\\\n\\u0001\u{7f}é\"");
    }

    #[test]
    fn string_to_number_follows_javascript() {
        assert!((string_to_number(" 12 ") - 12.0).abs() < f64::EPSILON);
        assert!(string_to_number("").abs() < f64::EPSILON);
        assert!(string_to_number("abc").is_nan());
        assert!((string_to_number("0x1f") - 31.0).abs() < f64::EPSILON);
        assert!(string_to_number("-Infinity").is_infinite());
        assert!(string_to_number("1e").is_nan());
        assert!((string_to_number(".5") - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn json_parser_reads_plain_json_and_refuses_duplicates() {
        let v = parse_json(r#"{"a": [1, 2.5e1, -0, "xé😀"], "b": {"c": null, "d": true}}"#);
        assert_eq!(
            v.map(|v| to_json(&v)).as_deref(),
            Some(r#"{"a":[1,25,0,"xé😀"],"b":{"c":null,"d":true}}"#)
        );
        assert!(parse_json(r#"{"a": 1, "a": 2}"#).is_none());
        assert!(parse_json("a: 1").is_none());
        assert!(parse_json("[1,]").is_none());
    }
}
