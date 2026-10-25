// Copyright 2026 Ferrule contributors
// SPDX-License-Identifier: Apache-2.0
// Tiny build-time highlighters for the landing page's code samples. Each returns one HTML string
// per source line, so callers can number and animate lines.

const PY_KEYWORDS = new Set(
  'from import as class def return self None True False if else for in with async await and or not is lambda yield try except raise'.split(
    ' ',
  ),
);

export function escapeHtml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function span(cls: string, text: string): string {
  return `<span class="${cls}">${escapeHtml(text)}</span>`;
}

function pyWord(word: string, next: string | undefined): string {
  if (PY_KEYWORDS.has(word)) return span('k', word);
  if (/^[A-Z]/.test(word)) return span('t', word);
  return next === '(' ? span('f', word) : escapeHtml(word);
}

export function highlightPython(source: string): string[] {
  let inDocstring = false;
  return source.split('\n').map((line) => {
    let out = '';
    let i = 0;
    if (inDocstring) {
      const end = line.indexOf('"""');
      if (end < 0) return span('s', line);
      out += span('s', line.slice(0, end + 3));
      i = end + 3;
      inDocstring = false;
    }
    while (i < line.length) {
      const rest = line.slice(i);
      if (rest.startsWith('"""')) {
        const end = rest.indexOf('"""', 3);
        if (end < 0) {
          out += span('s', rest);
          inDocstring = true;
          break;
        }
        out += span('s', rest.slice(0, end + 3));
        i += end + 3;
        continue;
      }
      if (rest.startsWith('#')) {
        out += span('c', rest);
        break;
      }
      const token =
        /^"[^"]*"/.exec(rest)?.[0] ?? /^@\w+/.exec(rest)?.[0] ?? /^[A-Za-z_]\w*/.exec(rest)?.[0];
      if (token === undefined) {
        out += escapeHtml(rest.charAt(0));
        i += 1;
      } else if (token.startsWith('"')) {
        out += span('s', token);
        i += token.length;
      } else if (token.startsWith('@')) {
        out += span('d', token);
        i += token.length;
      } else {
        out += pyWord(token, rest[token.length]);
        i += token.length;
      }
    }
    return out;
  });
}

export function highlightYaml(source: string): string[] {
  return source.split('\n').map((line) => {
    const m = /^(\s*)([\w$.-]+)(:)(.*)$/.exec(line);
    if (m === null) return escapeHtml(line);
    const [, indent = '', key = '', colon = '', value = ''] = m;
    const route = /^(\s*)(get|post|put|patch|delete)(\s+)(\S+)(.*)$/.exec(value);
    let rendered = '';
    if (route !== null) {
      const [, pad = '', verb = '', gap = '', path = '', tail = ''] = route;
      rendered =
        escapeHtml(pad) +
        span('y-verb', verb) +
        escapeHtml(gap) +
        span('y-path', path) +
        escapeHtml(tail);
    } else if (value.trim() !== '') {
      rendered = ` ${span('s', value.trim())}`;
    }
    return escapeHtml(indent) + span('y-key', key) + escapeHtml(colon) + rendered;
  });
}
