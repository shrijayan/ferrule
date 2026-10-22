import { describe, expect, it } from 'vitest';
import { PythonPrinter } from '../src/printer.ts';

describe('PythonPrinter', () => {
  it('sorts imports deterministically and indents blocks', () => {
    const p = new PythonPrinter()
      .importFrom('typing', 'Optional')
      .importModule('os')
      .importFrom('typing', 'List')
      .importFrom('__future__', 'annotations')
      .importModule('httpx')
      .importFrom('typing', 'List');
    p.block('class Foo:', (b) => {
      b.block('def bar(self) -> None:', () => {});
      b.line();
      b.line('x = 1');
    });
    expect(p.toString()).toBe(
      [
        'import httpx',
        'import os',
        'from __future__ import annotations',
        'from typing import List, Optional',
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

  it('prints without imports', () => {
    expect(new PythonPrinter('  ').block('if x:', (b) => b.line('y()')).toString()).toBe(
      'if x:\n  y()\n',
    );
  });
});
