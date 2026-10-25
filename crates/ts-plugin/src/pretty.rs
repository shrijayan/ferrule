//! The JavaScript and TypeScript the emitters print, as prettier's printers turn them into
//! documents ([`crate::doc`]): object and array literals, calls with last-argument hugging,
//! assignments with prettier's layouts, `for await` loops, union and generic types, import and
//! export lists, and method signatures. Only the shapes Stainless output contains are covered.

use crate::doc::{
    Doc, GroupId, concat, conditional_group, fill, group, group_break, group_id, hardline, if_break, indent,
    indent_if_break, join, line, print, softline, text, will_break,
};
use crate::js;

/// Stainless's prettier `printWidth` for source files.
pub const WIDTH: usize = 110;

/// The width prettier formats `@example` snippets at.
pub const EXAMPLE_WIDTH: usize = 60;

/// A JavaScript expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Printed as is and never broken: an identifier, `undefined`, `process.env['X']`.
    Raw(String),
    Str(String),
    /// A number as JavaScript prints it.
    Num(String),
    Bool(bool),
    Null,
    Object(Object),
    Array(Vec<Expr>),
    /// `callee(args)`, the callee a dotted name printed on one line.
    Call(String, Vec<Expr>),
    Await(Box<Expr>),
    /// `head {` + body lines + `}`, such as `async (err) => {`: printed on its own lines.
    Arrow(String, Vec<String>),
    /// A member chain `head.a(x).b(y)`, broken one call per line as prettier's member-chain
    /// printer does.
    Chain(String, Vec<(String, Vec<Expr>)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// Keys as written (quoted when they are not identifiers) and values.
    pub props: Vec<(String, Expr)>,
    /// Whether the source has a newline after `{`, which prettier keeps.
    pub expanded: bool,
}

impl Expr {
    #[must_use]
    pub fn str(s: &str) -> Self {
        Self::Str(s.to_owned())
    }

    #[must_use]
    pub fn raw(s: &str) -> Self {
        Self::Raw(s.to_owned())
    }

    /// An object literal the way Stainless writes one: on one line up to two properties,
    /// otherwise with a newline after `{` (which prettier then keeps).
    #[must_use]
    pub fn object(props: Vec<(String, Expr)>) -> Self {
        let expanded = props.len() > 2;
        Self::Object(Object { props, expanded })
    }

    fn is_object_or_array(&self) -> bool {
        matches!(self, Self::Object(o) if !o.props.is_empty()) || matches!(self, Self::Array(a) if !a.is_empty())
    }

    fn kind(&self) -> u8 {
        match self {
            Self::Raw(_) => 0,
            Self::Str(_) => 1,
            Self::Num(_) => 2,
            Self::Bool(_) => 3,
            Self::Null => 4,
            Self::Object(_) => 5,
            Self::Array(_) => 6,
            Self::Call(..) => 7,
            Self::Await(_) => 8,
            Self::Arrow(..) => 9,
            Self::Chain(..) => 10,
        }
    }
}

/// `printComma` joined items: `[",", line]` between them.
fn comma_line() -> Doc {
    concat(vec![text(","), line()])
}

/// Prints an expression.
#[must_use]
pub fn expr(e: &Expr) -> Doc {
    match e {
        Expr::Raw(s) | Expr::Num(s) => text(s.clone()),
        Expr::Str(s) => text(js::string_literal(s)),
        Expr::Bool(b) => text(if *b { "true" } else { "false" }),
        Expr::Null => text("null"),
        Expr::Object(o) => object(o),
        Expr::Array(items) => array(items),
        Expr::Call(callee, args) => group(concat(vec![text(callee.clone()), call_arguments(args)])),
        Expr::Await(inner) => concat(vec![text("await "), expr(inner)]),
        Expr::Arrow(head, body) => {
            let mut inner = Vec::new();
            for l in body {
                inner.push(hardline());
                inner.push(text(l.clone()));
            }
            concat(vec![text(format!("{head} {{")), indent(inner), hardline(), text("}")])
        }
        Expr::Chain(head, calls) => chain(head, calls),
    }
}

fn object(o: &Object) -> Doc {
    if o.props.is_empty() {
        return text("{}");
    }
    let props: Vec<Doc> = o.props.iter().map(|(k, v)| property(k, v)).collect();
    group_break(
        vec![
            text("{"),
            indent(vec![line(), join(&comma_line(), props)]),
            if_break(",", ""),
            line(),
            text("}"),
        ],
        o.expanded,
    )
}

/// `key: value` with prettier's assignment layout for object properties.
fn property(key: &str, value: &Expr) -> Doc {
    let key_doc = text(key.to_owned());
    // `isObjectPropertyWithShortKey`: a key narrower than tabWidth + 3 never breaks after `:`.
    let short_key = crate::doc::width(key) < 2 + 3;
    assignment(key_doc, ":", value, short_key)
}

/// `printAssignment` for the right-hand sides the emitters produce.
#[must_use]
pub fn assignment(left: Doc, operator: &str, right: &Expr, short_key: bool) -> Doc {
    let right_doc = expr(right);
    let layout = match right {
        Expr::Str(_) if !short_key => Layout::BreakAfterOperator,
        Expr::Str(_) | Expr::Num(_) | Expr::Bool(_) => Layout::NeverBreakAfterOperator,
        _ if short_key => Layout::NeverBreakAfterOperator,
        _ => Layout::Fluid,
    };
    match layout {
        Layout::BreakAfterOperator => group(vec![
            group(left),
            text(operator.to_owned()),
            group(indent(vec![line(), right_doc])),
        ]),
        Layout::NeverBreakAfterOperator => group(vec![group(left), text(format!("{operator} ")), right_doc]),
        Layout::Fluid => {
            let id = next_id();
            group(vec![
                group(left),
                text(operator.to_owned()),
                group_id(indent(line()), id),
                indent_if_break(right_doc, id),
            ])
        }
    }
}

enum Layout {
    BreakAfterOperator,
    NeverBreakAfterOperator,
    Fluid,
}

/// Group ids only need to be unique within one document; a counter per thread keeps output
/// deterministic.
fn next_id() -> GroupId {
    use std::cell::Cell;
    thread_local! {
        static NEXT: Cell<GroupId> = const { Cell::new(0) };
    }
    NEXT.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    })
}

/// `isConciselyPrintedArray`: only numbers, printed with `fill`.
fn concise(items: &[Expr]) -> bool {
    items.len() > 1 && items.iter().all(|e| matches!(e, Expr::Num(_)))
}

fn array(items: &[Expr]) -> Doc {
    if items.is_empty() {
        return text("[]");
    }
    // `shouldBreak`: more than one element, all objects (or all arrays) with more than one entry.
    let should_break = items.len() > 1
        && items.iter().all(|e| match (e, &items[0]) {
            (Expr::Object(o), Expr::Object(_)) => o.props.len() > 1,
            (Expr::Array(a), Expr::Array(_)) => a.len() > 1,
            _ => false,
        });
    if concise(items) {
        let mut parts = Vec::new();
        for (i, e) in items.iter().enumerate() {
            if i + 1 < items.len() {
                parts.push(concat(vec![expr(e), text(",")]));
                parts.push(line());
            } else {
                parts.push(expr(e));
            }
        }
        return group_break(
            vec![
                text("["),
                indent(vec![softline(), fill(parts)]),
                if_break(",", ""),
                softline(),
                text("]"),
            ],
            should_break,
        );
    }
    group_break(
        vec![
            text("["),
            indent(vec![softline(), join(&comma_line(), items.iter().map(expr).collect())]),
            if_break(",", ""),
            softline(),
            text("]"),
        ],
        should_break,
    )
}

/// `printCallArguments`: `(a, b)`, hugging a last object or array argument.
#[must_use]
pub fn call_arguments(args: &[Expr]) -> Doc {
    if args.is_empty() {
        return text("()");
    }
    let printed: Vec<Doc> = args.iter().map(expr).collect();
    let all_broken_out = || {
        group_break(
            vec![
                text("("),
                indent(vec![line(), join(&comma_line(), printed.clone()), text(",")]),
                line(),
                text(")"),
            ],
            true,
        )
    };
    let last = &args[args.len() - 1];
    let penultimate = args.len().checked_sub(2).map(|i| &args[i]);
    let group_last = (last.is_object_or_array() || matches!(last, Expr::Arrow(..)))
        && penultimate.is_none_or(|p| p.kind() != last.kind())
        && !(args.len() > 1 && matches!(last, Expr::Array(a) if concise(a)));
    if group_last {
        let head = &printed[..printed.len() - 1];
        if head.iter().any(will_break) {
            return all_broken_out();
        }
        let mut flat = vec![text("(")];
        let mut hugged = vec![text("(")];
        for d in head {
            flat.extend([d.clone(), text(", ")]);
            hugged.extend([d.clone(), text(", ")]);
        }
        flat.extend([printed[printed.len() - 1].clone(), text(")")]);
        hugged.extend([group_break(printed[printed.len() - 1].clone(), true), text(")")]);
        let any_break = printed.iter().any(will_break);
        let states = conditional_group(vec![concat(flat), concat(hugged), all_broken_out()]);
        return if any_break {
            concat(vec![Doc::BreakParent, states])
        } else {
            states
        };
    }
    let should_break = printed.iter().any(will_break);
    group_break(
        vec![
            text("("),
            indent(vec![softline(), join(&comma_line(), printed)]),
            if_break(",", ""),
            softline(),
            text(")"),
        ],
        should_break,
    )
}

/// `printMemberChain` for `head` (the first group, such as `client.embeddings`) and its calls.
fn chain(head: &str, calls: &[(String, Vec<Expr>)]) -> Doc {
    let groups: Vec<Doc> = calls
        .iter()
        .map(|(name, args)| concat(vec![text(format!(".{name}")), call_arguments(args)]))
        .collect();
    let mut one_line = vec![text(head.to_owned())];
    one_line.extend(groups.iter().cloned());
    let one_line = concat(one_line);
    // The first group and one call: never broken between them.
    if groups.len() < 2 {
        return group(one_line);
    }
    let mut broken = Vec::new();
    for g in &groups {
        broken.push(hardline());
        broken.push(g.clone());
    }
    let expanded = concat(vec![text(head.to_owned()), indent(broken)]);
    if groups[..groups.len() - 1].iter().any(will_break) {
        return group(expanded);
    }
    let states = conditional_group(vec![one_line.clone(), expanded]);
    if will_break(&one_line) {
        concat(vec![Doc::BreakParent, states])
    } else {
        states
    }
}

/// `const name = value;`.
#[must_use]
pub fn const_decl(name: &str, value: &Expr) -> Doc {
    concat(vec![
        assignment(text(format!("const {name}")), " =", value, false),
        text(";"),
    ])
}

/// `for await (const item of call) {` + `body` lines + `}`.
#[must_use]
pub fn for_await(item: &str, iterable: &Expr, body: &[&str]) -> Doc {
    let mut parts = vec![
        group(vec![
            text(format!("for await (const {item} of ")),
            expr(iterable),
            text(")"),
        ]),
        text(" {"),
    ];
    let mut inner = Vec::new();
    for l in body {
        inner.push(hardline());
        inner.push(text((*l).to_owned()));
    }
    parts.push(indent(inner));
    parts.push(hardline());
    parts.push(text("}"));
    concat(parts)
}

/// Lays out `doc` as a statement at `indent` columns: each line after the first is indented too.
#[must_use]
pub fn render(doc: &Doc, indent: usize, width: usize) -> String {
    let printed = print(doc, width.saturating_sub(indent));
    let pad = " ".repeat(indent);
    printed
        .split('\n')
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A TypeScript type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    /// A keyword, literal or reference, printed as is.
    Name(String),
    /// `Name<args>`, such as `Array<T>` or `Record<string, unknown>`.
    Generic(String, Vec<Ty>),
    Union(Vec<Ty>),
    /// `{ key: T; ... }`, keys as written.
    Object(Vec<(String, Ty)>),
}

impl Ty {
    #[must_use]
    pub fn name(s: &str) -> Self {
        Self::Name(s.to_owned())
    }

    /// `isSimpleType`: a keyword or a reference without type arguments.
    fn simple(&self) -> bool {
        matches!(self, Self::Name(n) if !n.starts_with('\'') && !n.starts_with('"') && n.parse::<f64>().is_err() && n != "true" && n != "false")
    }

    fn is_null(&self) -> bool {
        matches!(self, Self::Name(n) if n == "null" || n == "undefined" || n == "void")
    }

    /// `shouldHugType`.
    fn hug(&self) -> bool {
        match self {
            Self::Object(_) => true,
            Self::Union(members) => {
                let nulls = members.iter().filter(|m| m.is_null()).count();
                let objects = members.iter().any(|m| matches!(m, Self::Object(_)));
                members.len() - 1 == nulls && objects
            }
            other => other.simple(),
        }
    }

    /// The type on one line.
    #[must_use]
    pub fn flat(&self) -> String {
        match self {
            Self::Name(n) => n.clone(),
            Self::Generic(n, args) => format!("{n}<{}>", args.iter().map(Self::flat).collect::<Vec<_>>().join(", ")),
            Self::Union(members) => members.iter().map(Self::flat).collect::<Vec<_>>().join(" | "),
            Self::Object(fields) => {
                if fields.is_empty() {
                    "{}".to_owned()
                } else {
                    format!(
                        "{{ {} }}",
                        fields
                            .iter()
                            .map(|(k, t)| format!("{k}: {}", t.flat()))
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                }
            }
        }
    }
}

/// Where a union is printed, for prettier's `shouldIndent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnionAt {
    /// A property or parameter annotation, or a type alias: indented.
    Annotation,
    /// A type argument: not indented.
    TypeArgument,
}

/// Prints a type.
#[must_use]
pub fn ty(t: &Ty, at: UnionAt) -> Doc {
    match t {
        Ty::Name(n) => text(n.clone()),
        Ty::Generic(n, args) => {
            let printed: Vec<Doc> = args.iter().map(|a| ty(a, UnionAt::TypeArgument)).collect();
            if args.len() == 1 && args[0].hug() || args.is_empty() {
                concat(vec![text(format!("{n}<")), join(&text(", "), printed), text(">")])
            } else {
                group(vec![
                    text(format!("{n}<")),
                    indent(vec![softline(), join(&comma_line(), printed)]),
                    softline(),
                    text(">"),
                ])
            }
        }
        Ty::Union(members) => {
            let shoud_hug = t.hug();
            let printed: Vec<Doc> = members.iter().map(|m| ty(m, UnionAt::Annotation)).collect();
            if shoud_hug {
                return join(&text(" | "), printed);
            }
            let should_indent = at == UnionAt::Annotation;
            let code = concat(vec![
                if_break(
                    concat(vec![if should_indent { line() } else { text("") }, text("| ")]),
                    "",
                ),
                join(&concat(vec![line(), text("| ")]), printed),
            ]);
            group(if should_indent { indent(code) } else { code })
        }
        Ty::Object(fields) => {
            if fields.is_empty() {
                return text("{}");
            }
            let members: Vec<Doc> = fields
                .iter()
                .map(|(k, v)| concat(vec![text(format!("{k}: ")), ty(v, UnionAt::Annotation)]))
                .collect();
            group(vec![
                text("{"),
                indent(vec![line(), join(&concat(vec![text(";"), line()]), members)]),
                if_break(";", ""),
                line(),
                text("}"),
            ])
        }
    }
}

/// `name: T;` or `name?: T;` as an interface member.
#[must_use]
pub fn property_signature(name: &str, optional: bool, t: &Ty) -> Doc {
    concat(vec![
        text(format!("{name}{}: ", if optional { "?" } else { "" })),
        ty(t, UnionAt::Annotation),
        text(";"),
    ])
}

/// `export type Name = T;`: a union breaks one member per line after `=`.
#[must_use]
pub fn type_alias(name: &str, t: &Ty) -> Doc {
    match t {
        Ty::Union(_) if !t.hug() => group(vec![
            text(format!("export type {name} =")),
            ty(t, UnionAt::Annotation),
            text(";"),
        ]),
        _ => group(vec![
            text(format!("export type {name} = ")),
            ty(t, UnionAt::Annotation),
            text(";"),
        ]),
    }
}

/// `{ a, b }` of an import or export: one line when it fits, else one per line.
#[must_use]
pub fn specifiers(head: &str, names: &[String], tail: &str) -> Doc {
    if names.is_empty() {
        return text(format!("{head}{{}}{tail}"));
    }
    group(vec![
        text(format!("{head}{{")),
        indent(vec![
            line(),
            join(&comma_line(), names.iter().map(|n| text(n.clone())).collect()),
        ]),
        if_break(",", ""),
        line(),
        text(format!("}}{tail}")),
    ])
}

/// A method or function signature: `name(params): Ret {`, the parameters broken one per line when
/// the signature does not fit.
#[must_use]
pub fn signature(head: &str, params: &[Doc], ret: &Doc, tail: &str) -> Doc {
    let params_doc = if params.is_empty() {
        text("()")
    } else {
        group(vec![
            text("("),
            indent(vec![softline(), join(&comma_line(), params.to_vec())]),
            if_break(",", ""),
            softline(),
            text(")"),
        ])
    };
    concat(vec![
        group(vec![text(head.to_owned()), params_doc, ret.clone()]),
        text(tail.to_owned()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Expr {
        Expr::str(v)
    }

    #[test]
    fn breaks_after_the_operator_when_the_call_head_does_not_fit() {
        let call = Expr::Await(Box::new(Expr::Call(
            "client.extractions.qa.create".to_owned(),
            vec![Expr::object(vec![
                ("model".to_owned(), s("kanon-answer-extractor")),
                ("query".to_owned(), s("What is the punishment for murder in Victoria?")),
            ])],
        )));
        assert_eq!(
            render(&const_decl("answerExtractionResponse", &call), 0, EXAMPLE_WIDTH),
            "const answerExtractionResponse =\n  await client.extractions.qa.create({\n    model: 'kanon-answer-extractor',\n    query: 'What is the punishment for murder in Victoria?',\n  });"
        );
    }

    #[test]
    fn hugs_a_last_object_argument() {
        let call = Expr::Await(Box::new(Expr::Call(
            "client.embeddings.create".to_owned(),
            vec![Expr::object(vec![
                ("model".to_owned(), s("kanon-2-embedder")),
                (
                    "texts".to_owned(),
                    Expr::Array(vec![
                        s("Are restraints of trade enforceable under English law?"),
                        s("What is a non-compete clause?"),
                    ]),
                ),
            ])],
        )));
        assert_eq!(
            render(&const_decl("embeddingResponse", &call), 0, EXAMPLE_WIDTH),
            "const embeddingResponse = await client.embeddings.create({\n  model: 'kanon-2-embedder',\n  texts: [\n    'Are restraints of trade enforceable under English law?',\n    'What is a non-compete clause?',\n  ],\n});"
        );
    }

    #[test]
    fn breaks_long_unions_one_member_per_line() {
        let t = Ty::Union(
            ["'creation'", "'signature'", "'effective'"]
                .iter()
                .map(|m| Ty::name(m))
                .collect(),
        );
        assert_eq!(
            render(&property_signature("type", false, &t), 2, 30),
            "  type:\n    | 'creation'\n    | 'signature'\n    | 'effective';"
        );
        assert_eq!(
            render(&property_signature("type", false, &t), 2, 110),
            "  type: 'creation' | 'signature' | 'effective';"
        );
    }
}
