import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readdirSync } from 'node:fs';
import { asset, bridgeHarness, mailbox, musicContext } from './host.mjs';

test('command object arguments become native attributes without losing the requested class', () => {
  const host = bridgeHarness();
  let received;
  const original = host.Host.GUI.Commands.interpretCommand;
  host.Host.GUI.Commands.interpretCommand = (...args) => { received = args[3]; return original(...args); };
  assert.equal(host.writeRequest({ args: { category: 'Contract', name: 'Increment', args: { cid: 'class-two', openEditor: false, amount: 2 } } }).ok, true);
  assert.equal(received.cid, 'class-two');
  assert.equal(received.openEditor, false);
  assert.equal(received.amount, 2);
});

test('a reentrant host callback cannot apply an in-flight request again', () => {
  const host = bridgeHarness();
  const original = host.Host.GUI.Commands.interpretCommand;
  let reentered = false;
  host.Host.GUI.Commands.interpretCommand = (...args) => {
    if (!args[2] && !reentered) {
      reentered = true;
      host.bridge.tick('reentrant');
    }
    return original(...args);
  };
  const reply = host.writeRequest();
  assert.equal(reply.ok, true);
  assert.equal(host.effects.length, 1);
});

for (const method of ['command', 'editorCommand']) {
  for (const [category, name] of [['Track', 'Fender MCP Track Edit'], ['Musical Functions', 'Fender MCP Edit']]) {
    test(`${method} refuses generic invocation of ${name} before dispatching to the host`, () => {
      const host = bridgeHarness();
      const originalLookup = host.Host.Objects.getObjectByUrl;
      const calls = [];
      host.Host.Objects.getObjectByUrl = (url) => url.endsWith('/Editor')
        ? { interpretCommand: (...args) => { calls.push(['editor', ...args]); return true; } }
        : originalLookup(url);
      host.Host.GUI.Commands.findCommand = () => ({});
      host.Host.GUI.Commands.interpretCommand = (...args) => { calls.push(['global', ...args]); return true; };
      const result = host.bridge[method]({ category, name });
      assert.match(result.bridgeError, /internal EditTask commands require a fresh bridge lease/);
      assert.deepEqual(calls, []);
    });
  }

  test(`${method} checks an internal task's enabled state without executing it`, () => {
    const host = bridgeHarness();
    const originalLookup = host.Host.Objects.getObjectByUrl;
    const dispatches = [];
    const interpret = (category, name, checkOnly) => {
      if (!checkOnly) dispatches.push({ category, name });
      return true;
    };
    host.Host.Objects.getObjectByUrl = (url) => url.endsWith('/Editor')
      ? { interpretCommand: interpret }
      : originalLookup(url);
    host.Host.GUI.Commands.findCommand = () => ({});
    host.Host.GUI.Commands.interpretCommand = interpret;
    const result = host.bridge[method]({ category: 'Track', name: 'Fender MCP Track Edit', checkOnly: true });
    assert.equal(result.enabled, true);
    assert.deepEqual(dispatches, []);
  });
}

function noteEditBridge({ partCount = 2, refusePart = null } = {}) {
  const host = bridgeHarness();
  const song = musicContext();
  const parts = Array.from({ length: partCount }, (_, index) => ({
    name: 'Repeated part name',
    startTime: { seconds: index * 8 },
    endTime: { seconds: (index + 1) * 8 },
    notes: [{ pitch: 60 + index, startTime: { time: 0 } }],
  }));
  song.keys.events.push(...parts.slice().reverse());
  let selection = [];
  const selectedParts = [], leases = [];
  song.context.editor = { createSelectFunctions: () => ({
    selectExclusive: false,
    select: (part) => { selection = [part]; selectedParts.push(parts.indexOf(part) + 1); },
  }) };
  const scope = vm.createContext({ Host: host.Host, McpEditConfig: { mailbox } });
  scope.include_file = (name) => {
    if (name !== 'McpEditConfig.js') vm.runInContext(asset(`edit-tasks/package/${name}`), scope, { filename: name });
  };
  vm.runInContext(asset('edit-tasks/package/McpTrackEdit.js'), scope);
  vm.runInContext(asset('edit-tasks/package/McpEdit.js'), scope);
  const trackTask = vm.runInContext('createMcpTrackEdit()', scope);
  const notesTask = vm.runInContext('createMcpEdit()', scope);
  host.Host.GUI.Commands.findCommand = () => ({});
  host.Host.GUI.Commands.interpretCommand = (category, name, checkOnly) => {
    if (checkOnly) return true;
    if (name === 'Fender MCP Track Edit') { trackTask.performEdit(song.context); return true; }
    if (name === 'Fender MCP Edit') {
      const request = host.read('edit-request.json'), lease = host.read('task-lease.json');
      assert.equal(request.id, lease.id);
      assert.equal(lease.kind, 'notes');
      leases.push(lease.id);
      if (selection.some((part) => parts.indexOf(part) + 1 === refusePart)) return false;
      for (const part of selection) {
        let index = 0;
        notesTask.performEdit({
          iterator: { done: () => index >= part.notes.length, next: () => part.notes[index++] },
          functions: { modifyPitch: (note, pitch) => { note.pitch = pitch; } },
        });
      }
      return true;
    }
    if (category === 'Edit' && name === 'Deselect All') { selection = []; return true; }
    return false;
  };
  host.bridge.withTrack = (_name, fn) => fn();
  host.bridge.notes = () => ({
    track: 'Keys', mediaType: 'Music',
    parts: song.keys.events.map((part) => ({
      name: part.name, start: part.startTime.seconds, end: part.endTime.seconds,
      noteCount: part.notes.length, notes: part.notes.map((note) => ({ pitch: note.pitch })),
    })),
  });
  return { ...host, parts, selectedParts, leases, selection: () => selection };
}

test('notes_edit gives each chronological part a fresh one-shot task lease', () => {
  const host = noteEditBridge();
  const reply = host.writeRequest({ op: 'editNotes', args: { track: 'Keys', ops: [{ op: 'transpose', semitones: 1 }] } });
  assert.equal(reply.ok, true);
  assert.deepEqual(reply.result.errors, []);
  assert.deepEqual(host.parts.map((part) => part.notes[0].pitch), [61, 62]);
  assert.deepEqual(host.selectedParts, [1, 2]);
  assert.equal(host.leases.length, 2);
  assert.equal(new Set(host.leases).size, 2);
  assert.equal(reply.result.notesBefore, 2);
  assert.equal(reply.result.applied.reduce((sum, item) => sum + item.notes, 0), 2);
  assert.deepEqual(host.selection(), []);
  assert.deepEqual(host.read('edit-request.json'), { id: null, ops: [] });
  assert.deepEqual(host.read('task-lease.json'), { id: null });
});

test('notes_edit stops on a refused part and retains earlier applied evidence', () => {
  const host = noteEditBridge({ partCount: 3, refusePart: 2 });
  const reply = host.writeRequest({ op: 'editNotes', args: { track: 'Keys', ops: [{ op: 'transpose', semitones: 1 }] } });
  assert.equal(reply.ok, true);
  assert.match(reply.result.errors[0], /not available/);
  assert.deepEqual(host.parts.map((part) => part.notes[0].pitch), [61, 61, 62]);
  assert.deepEqual(host.selectedParts, [1, 2]);
  assert.equal(reply.result.applied[0].notes, 1);
  assert.equal(reply.result.notesBefore, 1);
  assert.deepEqual(host.selection(), []);
  assert.deepEqual(host.read('edit-request.json'), { id: null, ops: [] });
  assert.deepEqual(host.read('task-lease.json'), { id: null });
});

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
