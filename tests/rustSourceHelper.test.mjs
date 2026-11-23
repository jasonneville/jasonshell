import assert from 'node:assert/strict';
import test from 'node:test';
import { functionBody, productionPart, stripRustComments } from './support/rustSource.mjs';

test('stripRustComments removes line, doc and nested block comments but keeps literals', () => {
  const src = [
    'fn a() { // TerminateProcess in a comment',
    '  /* outer /* nested TerminateProcess */ still comment */',
    '  /// doc TerminateProcess',
    '  let url = "http://x // not a comment";',
    "  let q = '\"'; let raw = r#\"a // b \"quoted\" { \"#;",
    '  let life: &\'static str = "ok";',
    '}',
  ].join('\n');
  const out = stripRustComments(src);
  assert.ok(!out.includes('TerminateProcess'));
  assert.ok(out.includes('"http://x // not a comment"'));
  assert.ok(out.includes('r#"a // b "quoted" { "#'));
  assert.ok(out.includes("&'static str"));
});

test('productionPart drops cfg(test) modules and tier0 inclusion lines, keeps later production items', () => {
  const src = [
    'fn keep() {}',
    '#[cfg(test)]',
    'mod tests {',
    '    fn gone() { let s = "}"; }',
    '}',
    'fn also_keep() {}',
    '#[cfg(test)]',
    '#[path = "x_tier0_tests.rs"]',
    'mod tier0_tests;',
  ].join('\n');
  const out = productionPart(src);
  assert.ok(out.includes('fn keep') && out.includes('fn also_keep'));
  assert.ok(!out.includes('gone') && !out.includes('tier0_tests'));
});

test('functionBody ignores braces inside raw strings and comments', () => {
  const src = 'fn f() { let s = r#"}"#; /* } */ 1 }\nfn g() {}';
  const body = functionBody(stripRustComments(src), 'f');
  assert.ok(body.includes('1'));
  assert.ok(!body.includes('fn g'));
});
