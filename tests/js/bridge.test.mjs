import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readdirSync } from 'node:fs';
import { asset, bridgeHarness, mailbox } from './host.mjs';

test('a write with current protocol and context executes and identifies its document', () => {
  const host = bridgeHarness();
  const before = host.current();
  const result = host.writeRequest();
  assert.equal(result.ok, true);
  assert.equal(result.result.executed, true);
  assert.equal(result.protocol, 2);
  assert.equal(result.session, before.session);
  assert.equal(result.document, before.document);
  assert.equal(host.effects.length, 1);
});

test('an unsupported protocol rejects a write before applying it', () => {
  const host = bridgeHarness();
  for (const protocol of [undefined, 1, 3, '2']) {
    const result = host.writeRequest({ protocol });
    assert.equal(result.ok, false);
    assert.equal(result.code, 'protocol_mismatch');
  }
  assert.deepEqual(host.effects, []);
});

test('writes require both session and document context', () => {
  const host = bridgeHarness();
  const scope = host.current();
  for (const expected of [{}, { expectedSession: scope.session }, { expectedDocument: scope.document }]) {
    const result = host.request({ op: 'command', args: { category: 'Contract', name: 'Increment' }, ...expected });
    assert.equal(result.ok, false);
    assert.equal(result.code, 'context_required');
  }
  assert.deepEqual(host.effects, []);
});

test('session and document mismatch cause no host mutation', () => {
  const host = bridgeHarness();
  const session = host.writeRequest({ expectedSession: 'another-session' });
  assert.equal(session.ok, false);
  assert.equal(session.code, 'session_changed');
  const document = host.writeRequest({ expectedDocument: 'another-document' });
  assert.equal(document.ok, false);
  assert.equal(document.code, 'document_changed');
  assert.deepEqual(host.effects, []);
});

test('read operations need no write context but reject explicitly stale expectations', () => {
  const host = bridgeHarness();
  const result = host.request({ op: 'ping' });
  assert.equal(result.ok, true);
  assert.equal(result.result.pong, true);
  const stale = host.request({ op: 'context', expectedDocument: 'stale' });
  assert.equal(stale.code, 'document_changed');
  assert.deepEqual(host.effects, []);
});

test('duplicate IDs replay their original reply even after a newer request', () => {
  const host = bridgeHarness();
  const scope = host.current();
  const request = {
    id: 'edit-once', op: 'command', args: { category: 'Contract', name: 'Increment' },
    expectedSession: scope.session, expectedDocument: scope.document,
  };
  const original = host.request(request);
  assert.equal(host.effects.length, 1);
  assert.deepEqual(host.request(request), original);
  host.writeRequest({ id: 'next-edit' });
  assert.equal(host.effects.length, 2);
  assert.deepEqual(host.request(request), original);
  assert.equal(host.effects.length, 2);
});

test('document token remains stable for one object and changes after an observed close and reopen', () => {
  const host = bridgeHarness();
  const original = host.current();
  assert.equal(host.current().document, original.document);
  host.manager.activeDocument = null;
  const closed = host.current();
  assert.equal(closed.open, false);
  assert.notEqual(closed.document, original.document);
  host.manager.activeDocument = { ...host.document };
  const reopened = host.current();
  assert.equal(reopened.fileUrl, original.fileUrl);
  assert.equal(reopened.open, true);
  assert.notEqual(reopened.document, original.document);
  assert.notEqual(reopened.document, closed.document);
  const result = host.writeRequest({ expectedDocument: original.document });
  assert.equal(result.code, 'document_changed');
  assert.deepEqual(host.effects, []);
});

test('reopening the same path as a new document object invalidates an old token', () => {
  const host = bridgeHarness();
  const original = host.current();
  host.manager.activeDocument = { ...host.document };
  const reopened = host.current();
  assert.equal(reopened.fileUrl, original.fileUrl);
  assert.notEqual(reopened.document, original.document);
});

test('unknown operations and eval reject without running provided script', () => {
  const host = bridgeHarness();
  for (const op of ['missing-operation', 'eval']) {
    const result = host.writeRequest({ op, args: { code: 'Host.GUI.Commands.interpretCommand("Contract", "Increment")' } });
    assert.equal(result.ok, false);
    assert.match(result.error, /unknown op/i);
  }
  assert.deepEqual(host.effects, []);
});

test('incomplete JSON and invalid request IDs never execute a host write', () => {
  const host = bridgeHarness();
  host.files.set(mailbox + 'request.json', '{"protocol":2,');
  host.bridge.tick('contract');
  assert.equal(host.read('response.json'), null);
  for (const id of [null, 123, 'x'.repeat(129)]) {
    host.request({ id, op: 'command', args: { category: 'Contract', name: 'Increment' } });
    assert.equal(host.read('response.json'), null);
  }
  assert.deepEqual(host.effects, []);
});

test('all shipped host scripts parse and contain no arbitrary script compiler', () => {
  for (const directory of ['bridge', 'edit-tasks/package']) {
    const entries = readdirSync(new URL(`../../assets/${directory}/`, import.meta.url));
    for (const name of entries.filter((file) => file.endsWith('.js'))) {
      const source = asset(`${directory}/${name}`);
      assert.doesNotThrow(() => new vm.Script(source, { filename: name }));
      assert.doesNotMatch(source, /\bnew\s+Function\s*\(|\beval\s*\(/, name);
    }
  }
});
