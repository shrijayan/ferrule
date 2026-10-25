//! Prettier's document model and line-fitting printer, ported for the shapes the emitters print.
//!
//! Stainless formats every TypeScript file with prettier (`printWidth: 110`, single quotes,
//! trailing commas) and the `@example` snippets in doc comments at width 60. The plugin does not
//! run prettier (emitters are pure, and prettier cannot reach code inside comments), so the
//! emitters build the same documents prettier's printers would build and this module lays them
//! out with the same algorithm: `propagateBreaks`, then `printDocToString` with its `fits`
//! measure. Names follow prettier's `doc-builders.js` and `doc-printer.js`.

use std::collections::HashMap;

/// Identifies a group, for [`Doc::IfBreak`] and [`Doc::IndentIfBreak`].
pub type GroupId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A space, or a newline when the group breaks.
    Normal,
    /// Nothing, or a newline when the group breaks.
    Soft,
    /// Always a newline (with [`Doc::BreakParent`] in [`hardline`]).
    Hard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub contents: Doc,
    pub should_break: bool,
    pub id: Option<GroupId>,
    /// `conditionalGroup` states after the first (which is `contents`).
    pub expanded: Vec<Doc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doc {
    Text(String),
    Concat(Vec<Doc>),
    Group(Box<Group>),
    Indent(Box<Doc>),
    Line(Line),
    IfBreak {
        breaks: Box<Doc>,
        flat: Box<Doc>,
        group: Option<GroupId>,
    },
    IndentIfBreak {
        contents: Box<Doc>,
        group: GroupId,
    },
    BreakParent,
    Fill(Vec<Doc>),
}

impl From<&str> for Doc {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

impl From<String> for Doc {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<Vec<Doc>> for Doc {
    fn from(parts: Vec<Doc>) -> Self {
        Self::Concat(parts)
    }
}

#[must_use]
pub fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

#[must_use]
pub fn concat(parts: Vec<Doc>) -> Doc {
    Doc::Concat(parts)
}

#[must_use]
pub fn group(contents: impl Into<Doc>) -> Doc {
    Doc::Group(Box::new(Group {
        contents: contents.into(),
        should_break: false,
        id: None,
        expanded: Vec::new(),
    }))
}

/// `group(contents, { shouldBreak })`.
#[must_use]
pub fn group_break(contents: impl Into<Doc>, should_break: bool) -> Doc {
    Doc::Group(Box::new(Group {
        contents: contents.into(),
        should_break,
        id: None,
        expanded: Vec::new(),
    }))
}

/// `group(contents, { id })`.
#[must_use]
pub fn group_id(contents: impl Into<Doc>, id: GroupId) -> Doc {
    Doc::Group(Box::new(Group {
        contents: contents.into(),
        should_break: false,
        id: Some(id),
        expanded: Vec::new(),
    }))
}

/// `conditionalGroup(states)`: the first state that fits, else the last one broken.
///
/// # Panics
///
/// When `states` is empty.
#[must_use]
pub fn conditional_group(mut states: Vec<Doc>) -> Doc {
    let first = states.remove(0);
    Doc::Group(Box::new(Group {
        contents: first.clone(),
        should_break: false,
        id: None,
        expanded: std::iter::once(first).chain(states).collect(),
    }))
}

#[must_use]
pub fn indent(contents: impl Into<Doc>) -> Doc {
    Doc::Indent(Box::new(contents.into()))
}

#[must_use]
pub fn line() -> Doc {
    Doc::Line(Line::Normal)
}

#[must_use]
pub fn softline() -> Doc {
    Doc::Line(Line::Soft)
}

#[must_use]
pub fn hardline() -> Doc {
    Doc::Concat(vec![Doc::Line(Line::Hard), Doc::BreakParent])
}

#[must_use]
pub fn if_break(breaks: impl Into<Doc>, flat: impl Into<Doc>) -> Doc {
    Doc::IfBreak {
        breaks: Box::new(breaks.into()),
        flat: Box::new(flat.into()),
        group: None,
    }
}

#[must_use]
pub fn if_break_group(breaks: impl Into<Doc>, flat: impl Into<Doc>, group: GroupId) -> Doc {
    Doc::IfBreak {
        breaks: Box::new(breaks.into()),
        flat: Box::new(flat.into()),
        group: Some(group),
    }
}

#[must_use]
pub fn indent_if_break(contents: impl Into<Doc>, group: GroupId) -> Doc {
    Doc::IndentIfBreak {
        contents: Box::new(contents.into()),
        group,
    }
}

#[must_use]
pub fn fill(parts: Vec<Doc>) -> Doc {
    Doc::Fill(parts)
}

/// `join(separator, docs)`.
#[must_use]
pub fn join(separator: &Doc, docs: Vec<Doc>) -> Doc {
    let mut out = Vec::with_capacity(docs.len() * 2);
    for (i, d) in docs.into_iter().enumerate() {
        if i > 0 {
            out.push(separator.clone());
        }
        out.push(d);
    }
    Doc::Concat(out)
}

/// `willBreak`: whether the doc contains a forced break (a broken group or a hard line).
#[must_use]
pub fn will_break(doc: &Doc) -> bool {
    match doc {
        Doc::Text(_) | Doc::Line(Line::Normal | Line::Soft) => false,
        Doc::Line(Line::Hard) | Doc::BreakParent => true,
        Doc::Concat(parts) | Doc::Fill(parts) => parts.iter().any(will_break),
        // As prettier's `findInDoc`, a conditional group is searched in its first state only.
        Doc::Group(g) => g.should_break || will_break(&g.contents),
        Doc::Indent(c) | Doc::IndentIfBreak { contents: c, .. } => will_break(c),
        Doc::IfBreak { breaks, flat, .. } => will_break(breaks) || will_break(flat),
    }
}

/// `propagateBreaks`: marks every group that contains a hard break as broken; conditional groups
/// stop the propagation. Returns whether `doc` breaks its parent group.
fn propagate_breaks(doc: &mut Doc) -> bool {
    match doc {
        // A hard line breaks its group through the `BreakParent` beside it.
        Doc::Text(_) | Doc::Line(_) => false,
        Doc::BreakParent => true,
        Doc::Concat(parts) | Doc::Fill(parts) => {
            let mut any = false;
            for p in parts {
                any |= propagate_breaks(p);
            }
            any
        }
        Doc::Indent(c) | Doc::IndentIfBreak { contents: c, .. } => propagate_breaks(c),
        Doc::IfBreak { breaks, flat, .. } => {
            let a = propagate_breaks(breaks);
            let b = propagate_breaks(flat);
            a || b
        }
        Doc::Group(g) => {
            let mut child = propagate_breaks(&mut g.contents);
            for state in &mut g.expanded {
                child |= propagate_breaks(state);
            }
            if g.expanded.is_empty() && child {
                g.should_break = true;
            }
            g.should_break
        }
    }
}

/// Display width of printed text: prettier's `getStringWidth` (wide East Asian characters count
/// twice, combining marks not at all).
#[must_use]
pub fn width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let u = u32::from(c);
            if (0x300..=0x36f).contains(&u) || (0x200b..=0x200f).contains(&u) || u == 0xfe0f {
                0
            } else if (0x1100..=0x115f).contains(&u)
                || (0x2e80..=0xa4cf).contains(&u)
                || (0xac00..=0xd7a3).contains(&u)
                || (0xf900..=0xfaff).contains(&u)
                || (0xfe30..=0xfe4f).contains(&u)
                || (0xff00..=0xff60).contains(&u)
                || (0xffe0..=0xffe6).contains(&u)
                || (0x1f300..=0x1f64f).contains(&u)
                || (0x1f900..=0x1f9ff).contains(&u)
                || (0x20000..=0x3fffd).contains(&u)
            {
                2
            } else {
                1
            }
        })
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Break,
    Flat,
}

#[derive(Clone, Copy)]
enum Item<'d> {
    Doc(&'d Doc),
    /// The parts of a fill from this index on.
    Fill(&'d [Doc]),
    /// `indent(contents)` of an `IndentIfBreak`, printed in break mode.
    Indented(&'d Doc),
}

#[derive(Clone, Copy)]
struct Cmd<'d> {
    indent: usize,
    mode: Mode,
    item: Item<'d>,
}

const INDENT: usize = 2;

fn trim_trailing(out: &mut String) -> usize {
    let trimmed = out.trim_end_matches([' ', '\t']).len();
    let removed = out.len() - trimmed;
    out.truncate(trimmed);
    removed
}

/// `fits(next, restCommands, width, hasLineSuffix, groupModeMap, mustBeFlat)`.
fn fits(next: Cmd<'_>, rest: &[Cmd<'_>], mut width: isize, modes: &HashMap<GroupId, Mode>, must_be_flat: bool) -> bool {
    let mut rest_idx = rest.len();
    let mut cmds: Vec<(Mode, Item<'_>)> = vec![(next.mode, next.item)];
    while width >= 0 {
        let Some((mode, item)) = cmds.pop() else {
            if rest_idx == 0 {
                return true;
            }
            rest_idx -= 1;
            cmds.push((rest[rest_idx].mode, rest[rest_idx].item));
            continue;
        };
        let doc = match item {
            Item::Doc(d) | Item::Indented(d) => d,
            Item::Fill(parts) => {
                for p in parts.iter().rev() {
                    cmds.push((mode, Item::Doc(p)));
                }
                continue;
            }
        };
        match doc {
            Doc::Text(s) => {
                width -= isize::try_from(self::width(s)).unwrap_or(isize::MAX);
            }
            Doc::Concat(parts) | Doc::Fill(parts) => {
                for p in parts.iter().rev() {
                    cmds.push((mode, Item::Doc(p)));
                }
            }
            Doc::Indent(c) | Doc::IndentIfBreak { contents: c, .. } => cmds.push((mode, Item::Doc(c))),
            Doc::Group(g) => {
                if must_be_flat && g.should_break {
                    return false;
                }
                let group_mode = if g.should_break { Mode::Break } else { mode };
                let contents = match g.expanded.last() {
                    Some(last) if group_mode == Mode::Break => last,
                    _ => &g.contents,
                };
                cmds.push((group_mode, Item::Doc(contents)));
            }
            Doc::IfBreak { breaks, flat, group } => {
                let group_mode = group.map_or(mode, |id| modes.get(&id).copied().unwrap_or(Mode::Flat));
                let contents = if group_mode == Mode::Break { breaks } else { flat };
                cmds.push((mode, Item::Doc(contents)));
            }
            Doc::Line(kind) => {
                if mode == Mode::Break || *kind == Line::Hard {
                    return true;
                }
                if *kind == Line::Normal {
                    width -= 1;
                }
            }
            Doc::BreakParent => {}
        }
    }
    false
}

/// `printDocToString(doc, { printWidth })`: the laid-out text, without a final newline.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one match per document kind, as in prettier's printer"
)]
pub fn print(doc: &Doc, print_width: usize) -> String {
    let mut doc = doc.clone();
    propagate_breaks(&mut doc);
    let width = isize::try_from(print_width).unwrap_or(isize::MAX);
    let mut pos: isize = 0;
    let mut out = String::new();
    let mut modes: HashMap<GroupId, Mode> = HashMap::new();
    let mut cmds: Vec<Cmd<'_>> = vec![Cmd {
        indent: 0,
        mode: Mode::Break,
        item: Item::Doc(&doc),
    }];
    let mut should_remeasure = false;
    while let Some(cmd) = cmds.pop() {
        let Cmd { indent, mode, item } = cmd;
        let doc = match item {
            Item::Doc(d) => d,
            Item::Indented(d) => {
                cmds.push(Cmd {
                    indent: indent + INDENT,
                    mode,
                    item: Item::Doc(d),
                });
                continue;
            }
            Item::Fill(parts) => {
                print_fill(parts, indent, mode, width - pos, &modes, &mut cmds);
                continue;
            }
        };
        match doc {
            Doc::Text(s) => {
                out.push_str(s);
                if !cmds.is_empty() {
                    pos += isize::try_from(self::width(s)).unwrap_or(isize::MAX);
                }
            }
            Doc::Concat(parts) => {
                for p in parts.iter().rev() {
                    cmds.push(Cmd {
                        indent,
                        mode,
                        item: Item::Doc(p),
                    });
                }
            }
            Doc::Indent(c) => cmds.push(Cmd {
                indent: indent + INDENT,
                mode,
                item: Item::Doc(c),
            }),
            Doc::Group(g) => {
                let flat_ok = mode == Mode::Flat && !should_remeasure;
                if flat_ok {
                    cmds.push(Cmd {
                        indent,
                        mode: if g.should_break { Mode::Break } else { Mode::Flat },
                        item: Item::Doc(&g.contents),
                    });
                } else {
                    should_remeasure = false;
                    let next = Cmd {
                        indent,
                        mode: Mode::Flat,
                        item: Item::Doc(&g.contents),
                    };
                    let rem = width - pos;
                    if !g.should_break && fits(next, &cmds, rem, &modes, false) {
                        cmds.push(next);
                    } else if let Some(most) = g.expanded.last() {
                        if g.should_break {
                            cmds.push(Cmd {
                                indent,
                                mode: Mode::Break,
                                item: Item::Doc(most),
                            });
                        } else {
                            let mut chosen = None;
                            for state in &g.expanded[1..] {
                                let c = Cmd {
                                    indent,
                                    mode: Mode::Flat,
                                    item: Item::Doc(state),
                                };
                                if fits(c, &cmds, rem, &modes, false) {
                                    chosen = Some(c);
                                    break;
                                }
                            }
                            cmds.push(chosen.unwrap_or(Cmd {
                                indent,
                                mode: Mode::Break,
                                item: Item::Doc(most),
                            }));
                        }
                    } else {
                        cmds.push(Cmd {
                            indent,
                            mode: Mode::Break,
                            item: Item::Doc(&g.contents),
                        });
                    }
                }
                if let Some(id) = g.id
                    && let Some(top) = cmds.last()
                {
                    modes.insert(id, top.mode);
                }
            }
            Doc::Fill(parts) => print_fill(parts, indent, mode, width - pos, &modes, &mut cmds),
            Doc::IfBreak { breaks, flat, group } => {
                let group_mode = group.map_or(mode, |id| modes.get(&id).copied().unwrap_or(Mode::Flat));
                let contents = if group_mode == Mode::Break { breaks } else { flat };
                cmds.push(Cmd {
                    indent,
                    mode,
                    item: Item::Doc(contents),
                });
            }
            Doc::IndentIfBreak { contents, group } => {
                let group_mode = modes.get(group).copied().unwrap_or(Mode::Flat);
                cmds.push(Cmd {
                    indent,
                    mode,
                    item: if group_mode == Mode::Break {
                        Item::Indented(contents)
                    } else {
                        Item::Doc(contents)
                    },
                });
            }
            Doc::Line(kind) => {
                if mode == Mode::Flat && *kind != Line::Hard {
                    if *kind == Line::Normal {
                        out.push(' ');
                        pos += 1;
                    }
                } else {
                    if mode == Mode::Flat {
                        should_remeasure = true;
                    }
                    trim_trailing(&mut out);
                    out.push('\n');
                    out.push_str(&" ".repeat(indent));
                    pos = isize::try_from(indent).unwrap_or(isize::MAX);
                }
            }
            Doc::BreakParent => {}
        }
    }
    out
}

/// The `fill` case of `printDocToString`: content and separator pairs, each separator broken only
/// when the next content does not fit.
fn print_fill<'d>(
    parts: &'d [Doc],
    indent: usize,
    mode: Mode,
    rem: isize,
    modes: &HashMap<GroupId, Mode>,
    cmds: &mut Vec<Cmd<'d>>,
) {
    let Some(content) = parts.first() else { return };
    let cmd = |mode: Mode, item: Item<'d>| Cmd { indent, mode, item };
    let content_flat = cmd(Mode::Flat, Item::Doc(content));
    let content_break = cmd(Mode::Break, Item::Doc(content));
    let content_fits = fits(content_flat, &[], rem, modes, true);
    let Some(whitespace) = parts.get(1) else {
        cmds.push(if content_fits { content_flat } else { content_break });
        return;
    };
    let ws_flat = cmd(Mode::Flat, Item::Doc(whitespace));
    let ws_break = cmd(Mode::Break, Item::Doc(whitespace));
    if parts.len() == 2 {
        if content_fits {
            cmds.push(ws_flat);
            cmds.push(content_flat);
        } else {
            cmds.push(ws_break);
            cmds.push(content_break);
        }
        return;
    }
    let remaining = cmd(mode, Item::Fill(&parts[2..]));
    let pair_doc = Doc::Concat(vec![content.clone(), whitespace.clone(), parts[2].clone()]);
    let pair_fits = fits(
        Cmd {
            indent,
            mode: Mode::Flat,
            item: Item::Doc(&pair_doc),
        },
        &[],
        rem,
        modes,
        true,
    );
    cmds.push(remaining);
    if pair_fits {
        cmds.push(ws_flat);
        cmds.push(content_flat);
    } else if content_fits {
        cmds.push(ws_break);
        cmds.push(content_flat);
    } else {
        cmds.push(ws_break);
        cmds.push(content_break);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bracket(items: &[&str]) -> Doc {
        group(vec![
            text("["),
            indent(vec![
                softline(),
                join(
                    &concat(vec![text(","), line()]),
                    items.iter().map(|&s| text(s)).collect(),
                ),
            ]),
            if_break(",", ""),
            softline(),
            text("]"),
        ])
    }

    #[test]
    fn keeps_a_group_flat_when_it_fits() {
        assert_eq!(print(&bracket(&["a", "b"]), 10), "[a, b]");
    }

    #[test]
    fn breaks_a_group_that_does_not_fit() {
        assert_eq!(print(&bracket(&["aaaa", "bbbb"]), 10), "[\n  aaaa,\n  bbbb,\n]");
    }

    #[test]
    fn hard_lines_break_every_enclosing_group() {
        let doc = group(vec![
            text("{"),
            indent(vec![line(), text("a"), hardline(), text("b")]),
            line(),
            text("}"),
        ]);
        assert_eq!(print(&doc, 80), "{\n  a\n  b\n}");
    }

    #[test]
    fn fills_as_many_items_per_line_as_fit() {
        let items: Vec<Doc> = ["1", "2", "3", "4", "5"]
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                let mut v = vec![text(format!("{s},"))];
                if i < 4 {
                    v.push(line());
                }
                v
            })
            .collect();
        assert_eq!(print(&fill(items), 6), "1, 2,\n3, 4,\n5,");
    }

    #[test]
    fn measures_wide_characters_twice() {
        assert_eq!(width("ab"), 2);
        assert_eq!(width("\u{4e2d}"), 2);
        assert_eq!(width("e\u{301}"), 1);
    }
}
