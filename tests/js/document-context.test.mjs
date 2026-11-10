import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness, editTaskHarness, musicContext } from './host.mjs';

const snapshot = (generation = '1', extra = {}) => ({
  ok: true, api: 'fender-mcp-native-v1', context_api: 'native_document_context_v1',
  token: `ndc1:0000000000000001:${generation}`, open: true,
  title: 'Scratch', fileUrl: 'file:///scratch/song.song', ...extra,
});

function nativeBridge() {
  const host = bridgeHarness();
  let value = snapshot(), action = null, calls = 0, documentReads = 0;
  const service = { documentContext() { calls++; if (action) action(); return JSON.stringify(value); } };
  host.Host.Objects.getObjectByName = name => name === 'FenderMcpNative' ? service : null;
  Object.defineProperty(host.manager, 'activeDocument', { get() { documentReads++; throw new Error('document proxy accessed'); } });
  return { ...host, service, set: next => { value = next; }, onRead: next => { action = next; },
    calls: () => calls, documentReads: () => documentReads };
}

test('native context and song metadata never obtain a JavaScript SongDocument proxy', () => {
  const host = nativeBridge();
  const first = host.current();
  assert.equal(first.open, true);
  assert.equal(host.current().document, first.document);
  host.bridge.transportState = () => ({ playing: false });
  const song = host.bridge.song();
  assert.equal(song.title, first.title);
  assert.equal(song.fileUrl, first.fileUrl);
  assert.equal(host.documentReads(), 0);
  assert.equal(typeof host.bridge.documentObject, 'string');
});

test('native identity detects close and same-path reopen while metadata-only changes keep the epoch', () => {
  const host = nativeBridge(), first = host.current();
  host.set(snapshot('1', { title: 'Renamed', fileUrl: 'file:///new.song' }));
  assert.equal(host.current().document, first.document);
  host.set(snapshot('2', { open: false, title: null, fileUrl: null }));
  const closed = host.current();
  assert.equal(closed.open, false);
  assert.notEqual(closed.document, first.document);
  host.set(snapshot('3'));
  const reopened = host.current();
  assert.notEqual(reopened.document, first.document);
  assert.notEqual(reopened.document, closed.document);
  assert.equal(host.documentReads(), 0);
});

test('native discovery latches its service and rejects a stopped service without falling back', () => {
  const host = nativeBridge();
  const context = host.current();
  let replacementCalls = 0;
  host.Host.Objects.getObjectByName = () => ({ documentContext() { replacementCalls++; return JSON.stringify(snapshot('1')); } });
  host.service.documentContext = () => { throw new Error('stopped'); };
  const before = host.effects.length;
  const result = host.request({ op: 'command', expectedSession: context.session, expectedDocument: context.document,
    args: { category: 'Contract', name: 'Increment' } });
  assert.equal(result.ok, false);
  assert.equal(host.effects.length, before);
  assert.equal(replacementCalls, 0);
  assert.equal(host.documentReads(), 0);
});

test('a legacy bridge retains its source until restart when a native extension appears later', () => {
  const host = bridgeHarness(), original = host.current();
  host.Host.Objects.getObjectByName = () => ({ documentContext() { throw new Error('unexpected source switch'); } });
  assert.equal(host.current().document, original.document);
  assert.equal(host.bridge.documentMode, 'legacy');
});

test('native discovery errors block dispatch before any legacy document access', () => {
  const host = nativeBridge();
  host.Host.Objects.getObjectByName = () => { throw new Error('lookup failed'); };
  const result = host.request({ op: 'command', expectedSession: host.bridge.session, expectedDocument: 'unrelated',
    args: { category: 'Contract', name: 'Increment' } });
  assert.equal(result.ok, false);
  assert.equal(result.code, 'missing_precondition');
  assert.equal(host.effects.length, 0);
  assert.equal(host.documentReads(), 0);
});

test('context resolution blocks MIDI reentry before requestContext is installed', () => {
  const host = nativeBridge();
  let reentries = 0;
  host.onRead(() => { reentries++; host.bridge.tick('reentrant-context'); });
  const result = host.writeRequest();
  assert.equal(result.ok, true);
  assert.equal(host.effects.length, 1);
  assert.ok(reentries > 0);
  assert.equal(host.bridge.contextBusy, false);
});

test('a native context error after dispatch discards a positive command result', () => {
  const host = nativeBridge(), current = host.current();
  const original = host.Host.GUI.Commands.interpretCommand;
  host.Host.GUI.Commands.interpretCommand = (...args) => {
    const result = original(...args);
    if (!args[2]) host.service.documentContext = () => 'not JSON';
    return result;
  };
  const result = host.request({ op: 'command', expectedSession: current.session, expectedDocument: current.document,
    args: { category: 'Contract', name: 'Increment' } });
  assert.equal(host.effects.length, 1);
  assert.equal(result.ok, false);
  assert.equal(result.code, 'native_context_lost');
  assert.equal(result.result, undefined);
});

test('a failed native read invalidates old request epochs even after the same token recovers', () => {
  const host = nativeBridge(), original = host.current();
  host.set({ ok: false, error: { code: 'unavailable' }, outcome: 'not_applied' });
  assert.equal(host.request({ op: 'context' }).ok, false);
  host.set(snapshot());
  const recovered = host.current();
  assert.notEqual(recovered.document, original.document);
  assert.equal(host.request({ op: 'command', expectedSession: original.session, expectedDocument: original.document,
    args: { category: 'Contract', name: 'Increment' } }).ok, false);
  assert.equal(host.effects.length, 0);
});

test('native token source, generation and open state cannot regress under an existing epoch', () => {
  for (const changed of [snapshot('1'), snapshot('2', { token: 'ndc1:0000000000000002:3' }),
    snapshot('2', { open: false, title: null, fileUrl: null })]) {
    const host = nativeBridge();
    host.set(snapshot('2')); host.current(); host.set(changed);
    assert.equal(host.request({ op: 'context' }).ok, false);
    assert.equal(host.documentReads(), 0);
  }
});

test('native response validation rejects malformed success carriers without publishing metadata', () => {
  const host = nativeBridge();
  for (const value of [null, [], {}, snapshot('0'), snapshot('18446744073709551616'),
    snapshot('1', { token: 'ndc1:0000000000000000:1' }), snapshot('1', { extra: true }),
    snapshot('1', { open: false }), snapshot('1', { title: 'a\0b' }), snapshot('1', { fileUrl: 1 }),
    snapshot('1', { title: 'x'.repeat(4097) }), snapshot('1', { context_api: 'other' })]) {
    host.set(value);
    const result = host.request({ op: 'context' });
    assert.equal(result.ok, false);
    assert.equal(result.result, undefined);
  }
  assert.equal(host.documentReads(), 0);
});

test('token validation preserves exact u64 generations beyond JavaScript integer precision', () => {
  const scope = vm.createContext({});
  vm.runInContext(asset('edit-tasks/package/McpDocumentContext.js'), scope);
  const parse = vm.runInContext('mcpDocumentToken', scope);
  assert.equal(parse('ndc1:0000000000000001:18446744073709551615').generation, '18446744073709551615');
  assert.equal(parse('ndc1:0000000000000001:18446744073709551616'), null);
  assert.equal(parse('ndc1:0000000000000001:01'), null);
});

test('native task leases bind exact document tokens and consume failed attempts without legacy fallback', () => {
  for (const failure of ['none', 'token', 'title', 'missing', 'malformed']) {
    const host = editTaskHarness('track'), music = musicContext();
    const id = host.arm([{ op: 'createPart', track: 'Keys', start: 0, end: 8 }], { lease: {
      nativeDocument: { context_api: 'native_document_context_v1', token: snapshot().token },
    } });
    let calls = 0, legacyReads = 0;
    host.Host.Objects.getObjectByName = () => failure === 'missing' ? null : ({ documentContext() {
      calls++;
      if (failure === 'malformed') return '{}';
      return JSON.stringify(snapshot(failure === 'token' ? '2' : '1', failure === 'title' ? { title: 'Other' } : {}));
    } });
    const original = host.Host.Objects.getObjectByUrl;
    host.Host.Objects.getObjectByUrl = url => {
      if (url === '://studioapp/DocumentManager') { legacyReads++; throw new Error('legacy task document access'); }
      return original(url);
    };
    const result = host.invoke(music.context);
    assert.equal(result.id, id);
    assert.equal(!!result.error, failure !== 'none');
    assert.equal(music.keys.events.length, failure === 'none' ? 1 : 0);
    assert.equal(legacyReads, 0);
    assert.equal(host.read('task-lease.json').id, null);
    assert.equal(host.read(host.requestFilename).id, null);
    assert.equal(calls, failure === 'missing' ? 0 : 1);
  }
});

test('arming an EditTask carries the native identity without changing its public document epoch', () => {
  const host = nativeBridge(), context = host.current();
  host.bridge.requestContext = context;
  const result = host.bridge.armTask('track', 'track-edit-request.json', 'owned-task', [{ op: 'anything' }]);
  assert.equal(result.armed, true);
  const lease = host.read('task-lease.json');
  assert.equal(lease.document, context.document);
  assert.deepEqual(lease.nativeDocument, { context_api: 'native_document_context_v1', token: snapshot().token });
  assert.equal(host.documentReads(), 0);
});
