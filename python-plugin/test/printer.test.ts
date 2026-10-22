import { describe, expect, it } from 'vitest';
import { PythonPrinter } from '../src/printer.ts';

describe('PythonPrinter', () => {
  it('sorts imports into sections, by length, classes before functions', () => {
    const p = new PythonPrinter(['# header'])
      .importFrom('.._types', 'not_given')
      .importFrom('.._types', 'NotGiven')
      .importFrom('.._types', 'Body')
      .importFrom('.._types', 'MAX')
      .importFrom('typing', 'Optional')
      .importModule('os')
      .importFrom('typing', 'List')
      .importFrom('__future__', 'annotations')
      .importModule('httpx')
      .importFrom('typing_extensions', 'Literal')
      .importFrom('.._compat', 'cached_property')
      .importFrom('typing', 'List');
    p.block('class Foo:', (b) => {
      b.block('def bar(self) -> None:', () => {});
      b.line();
      b.line('x = 1');
    });
    expect(p.toString()).toBe(
      [
        '# header',
        '',
        'from __future__ import annotations',
        '',
        'import os',
        'from typing import List, Optional',
        'from typing_extensions import Literal',
        '',
        'import httpx',
        '',
        'from .._types import MAX, Body, NotGiven, not_given',
        'from .._compat import cached_property',
        '',
        'class Foo:',
        '    def bar(self) -> None:',
        '        pass',
        '',
        '    x = 1',
        '',
      ].join('\n'),
    );
  });

  it('wraps an import longer than the line length', () => {
    const p = new PythonPrinter();
    for (const n of [
      'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      'cccccccccccccccccccccccccccccccccccccccccccc',
    ]) {
      p.importFrom('.mod', n);
    }
    expect(p.toString()).toBe(
      [
        'from .mod import (',
        '    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,',
        '    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,',
        '    cccccccccccccccccccccccccccccccccccccccccccc,',
        ')',
        '',
      ].join('\n'),
    );
  });

  it('prints without imports, with a custom indent', () => {
    const p = new PythonPrinter([], '  ').block('if x:', (b) => b.line('y()'));
    expect(p.toString()).toBe('if x:\n  y()\n');
  });
});
