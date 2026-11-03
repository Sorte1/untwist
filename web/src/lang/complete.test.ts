import { describe, expect, it } from 'vitest';
import { engineType, memberOptions, ownerOf, paramsLabel, wordOptions, type Described } from './complete';
import { activeParam } from './signature';

const described = (fields: Partial<Described>): Described => ({ kind: 'const', properties: [], ...fields });

describe('word completions', () => {
  it('keep the engine order first and say what each binding is', () => {
    const options = wordOptions([
      { label: 'sum', kind: 'let' },
      { label: 'add', kind: 'function', params: ['a', 'b'] },
      { label: 'LIMIT', kind: 'const' },
    ]);
    expect(options.slice(0, 3).map((option) => [option.label, option.type, option.detail])).toEqual([
      ['sum', 'variable', 'let'],
      ['add', 'function', 'function (a, b)'],
      ['LIMIT', 'constant', 'const'],
    ]);
    expect((options[0]?.boost ?? 0) > (options[1]?.boost ?? 0)).toBe(true);
  });

  it('document a global the text uses and add the other built-ins and snippets below', () => {
    const options = wordOptions([{ label: 'parseInt', kind: 'global' }]);
    const parse = options.find((option) => option.label === 'parseInt');
    expect(parse?.detail).toBe('(string, radix?): number');
    expect(options.filter((option) => option.label === 'parseInt')).toHaveLength(1);
    expect(options.find((option) => option.label === 'setTimeout')?.type).toBe('function');
    expect(options.find((option) => option.label === 'forof')?.type).toBe('snippet');
  });

  it('map engine kinds to editor icons', () => {
    expect(['function', 'class', 'const', 'import', 'keyword', 'param', 'var'].map(engineType)).toEqual(['function', 'class', 'constant', 'namespace', 'keyword', 'variable', 'variable']);
    expect(paramsLabel(['a', '...rest'])).toBe('(a, ...rest)');
    expect(paramsLabel(undefined)).toBe('');
  });
});

describe('member completions', () => {
  it('list seen properties first, then the built-ins of the form, without the word being typed', () => {
    const options = memberOptions({ name: 'box', from: 0, form: undefined }, described({ form: 'object', properties: ['width', 'wi', 'toString'] }), 'wi');
    expect(options[0]).toMatchObject({ label: 'width', type: 'property' });
    expect(options.map((option) => option.label)).not.toContain('wi');
    expect(options.filter((option) => option.label === 'toString')).toHaveLength(1);
  });

  it('use the namespace members of an unshadowed global', () => {
    const owner = ownerOf({ name: 'Math', from: 0, form: undefined }, null);
    expect(owner).toEqual({ name: 'Math', global: true, form: undefined });
    const options = memberOptions({ name: 'Math', from: 0, form: undefined }, null, '');
    expect(options.map((option) => option.label)).toContain('floor');
  });

  it('take a literal form from the text itself', () => {
    const options = memberOptions({ name: null, from: 0, form: 'array' }, null, '');
    expect(options.map((option) => option.label)).toContain('map');
  });

  it('treat a shadowed global as the local binding', () => {
    expect(ownerOf({ name: 'Math', from: 0, form: undefined }, described({ kind: 'const', form: 'object' }))).toEqual({ name: 'Math', global: false, form: 'object' });
  });
});

describe('parameter hints', () => {
  it('highlight the argument at the cursor and stay on a rest parameter', () => {
    expect(activeParam(['a', 'b'], 1)).toBe(1);
    expect(activeParam(['a', '...rest'], 4)).toBe(1);
    expect(activeParam(['a'], 3)).toBe(-1);
  });
});
