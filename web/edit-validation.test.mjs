import test from 'node:test';
import assert from 'node:assert/strict';
import {validatedEditCache, deferredValidation} from './edit-validation.js';

test('validated candidates require the same source and complete patch', () => {
  let writes = 0;
  const build = validatedEditCache((source, patch) => {
    writes++;
    if (patch.expectedValue !== source[0]) throw new Error('stale value');
    return Uint8Array.of(patch.replacement);
  });
  const source = Uint8Array.of(1), patch = {expectedValue: 1, replacement: 2};
  const candidate = build(source, patch);
  assert.strictEqual(build(source, {...patch}), candidate);
  assert.equal(writes, 1);
  assert.deepEqual(build(source, {...patch, replacement: 3}), Uint8Array.of(3));
  assert.deepEqual(build(source.slice(), {...patch, replacement: 3}), Uint8Array.of(3));
  assert.equal(writes, 3);
  assert.throws(() => build(Uint8Array.of(4), patch), /stale value/);
});

test('a failed candidate is never reused', () => {
  let writes = 0;
  const build = validatedEditCache(() => { writes++; throw new Error('unsupported'); });
  const source = Uint8Array.of(1);
  assert.throws(() => build(source, {}), /unsupported/);
  assert.throws(() => build(source, {}), /unsupported/);
  assert.equal(writes, 2);
});

test('typing checks only the latest draft and reports pending immediately', t => {
  t.mock.timers.enable({apis: ['setTimeout']});
  let draft = '', checked = [];
  const reports = [];
  const validation = deferredValidation(() => checked.push(draft), (...state) => reports.push(state));
  for (const value of ['n', 'ne', 'new']) {
    draft = value; validation.schedule(); t.mock.timers.tick(50);
    assert.deepEqual(reports.at(-1), ['', true]);
  }
  assert.deepEqual(checked, []);
  t.mock.timers.tick(100);
  assert.deepEqual(checked, ['new']);
  assert.deepEqual(reports.at(-1), ['', false]);
});

test('cancelled or replaced forms never check a stale draft', t => {
  t.mock.timers.enable({apis: ['setTimeout']});
  let checks = 0;
  const validation = deferredValidation(() => { checks++; }, () => {});
  validation.schedule(); validation.cancel(); t.mock.timers.tick(200);
  assert.equal(checks, 0);
  validation.schedule(); validation.run(); t.mock.timers.tick(200);
  assert.equal(checks, 1);
});

test('writer errors keep the draft invalid after delayed checking', t => {
  t.mock.timers.enable({apis: ['setTimeout']});
  const reports = [];
  const validation = deferredValidation(() => { throw new Error('ambiguous target'); }, (...state) => reports.push(state));
  validation.schedule(); t.mock.timers.tick(150);
  assert.deepEqual(reports, [['', true], ['Error: ambiguous target', false]]);
});

test('navigating away before validation does not write a detached form', t => {
  t.mock.timers.enable({apis: ['setTimeout']});
  let connected = true, checks = 0;
  const validation = deferredValidation(() => { checks++; }, () => {}, {isCurrent: () => connected});
  validation.schedule(); connected = false; t.mock.timers.tick(150);
  assert.equal(checks, 0);
});
