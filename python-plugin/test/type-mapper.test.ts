import { describe, expect, it } from 'vitest';
import { readExtensions } from '../src/adapter/extensions.ts';
import { generate } from '../src/plugin.ts';
import { PythonPrinter } from '../src/printer.ts';
import { toAnnotation } from '../src/type-mapper.ts';

describe('toAnnotation', () => {
  it('maps every SdkType kind and collects imports', () => {
    const p = new PythonPrinter();
    const ann = toAnnotation(
      {
        kind: 'union',
        variants: [
          { kind: 'primitive', name: 'string' },
          { kind: 'primitive', name: 'integer' },
          { kind: 'primitive', name: 'number' },
          { kind: 'primitive', name: 'boolean' },
          { kind: 'primitive', name: 'null' },
          { kind: 'unknown' },
          { kind: 'array', items: { kind: 'ref', name: ['user'], module: '.user' } },
          {
            kind: 'map',
            values: { kind: 'optional', inner: { kind: 'primitive', name: 'string' } },
          },
          { kind: 'literal', values: ['a', 1, true, false] },
        ],
      },
      p,
    );
    expect(ann).toBe(
      'Union[str, int, float, bool, None, object, List[User], Dict[str, Optional[str]], Literal["a", 1, True, False]]',
    );
    expect(p.toString()).toBe(
      [
        'from .user import User',
        'from typing import Dict, List, Optional, Union',
        'from typing_extensions import Literal',
        '',
      ].join('\n'),
    );
  });
});

describe('stubs', () => {
  it('generate emits a file tree', () => {
    const tree = generate({ title: 'Acme API', packageName: 'acme', baseUrl: 'https://x' });
    expect([...tree.keys()]).toContain('src/acme/_client.py');
  });

  it('reads x-ferrule extensions', () => {
    expect(readExtensions({ 'x-ferrule-pagination': 1, other: 2 })).toEqual({ pagination: 1 });
  });
});
