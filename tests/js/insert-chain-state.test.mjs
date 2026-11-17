import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const folderUid = '12345678-1234-1234-1234-123456789ABC';
const token = 'ABCDEF0123456789'.repeat(4);
const folderIdentity = () => ({ object_uid: folderUid, class_uid: null, native_class: 'AudioInsertFolder', object_id: 'Inserts' });
function reply(action) {
  const common = { ok: true, scope: 'insert_chain_state', format: 'host_multipreset_v1', version: 1,
    folder: folderIdentity(), verified: false, effects_verified: false, atomic_snapshot: false, token };
  return action === 'release' ? { ...common, outcome: 'applied_unverified', released: true }
    : { ...common, outcome: action === 'capture' ? 'observed' : 'applied_unverified',
      byte_count: 16 * 1024 * 1024, sha256: 'a'.repeat(64), part_count: 128,
      ...(action === 'restore' ? { load_accepted: true } : {}) };
}

function harness(options = {}) {
  const calls = [], urls = [], resolutions = [];
  const current = { open: true, document: 'chain:1', session: 'chain' };
  const folder = {};
  const channel = { find(name) { resolutions.push(name); options.folder?.(current); return folder; } };
  const service = {
    capabilities() { options.capability?.(current); return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1',
      insert_chain_api: 'guarded_folder_collection_v1', ...options.capabilityFields }); },
    request(...args) {
      calls.push(args);
      const body = JSON.parse(args[1]);
      options.dispatch?.(current);
      const value = reply(body.operation.slice('insert_chain_'.length));
      options.result?.(value);
      return options.rawResult === undefined ? JSON.stringify(value) : options.rawResult;
    },
  };
  const scope = vm.createContext({ include_file() {}, PreSonus: { ControlSurfaceComponent: class {} }, Host: {
    Objects: { getObjectByName() { options.lookup?.(current); return service; } },
    Url(value) { const url = { value }; urls.push(url); options.url?.(current); return url; },
  } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: { ...current }, context() { return { ...current }; },
    mixerChannel(label) { resolutions.push(label); options.channel?.(current); return options.channelResult || channel; } };
  component.pluginOf = () => { throw new Error('Device resolution is forbidden'); };
  return { component, folder, current, calls, urls, resolutions, scope };
}

function noClaims(value) {
  for (const key of ['token', 'folder', 'sha256', 'byte_count', 'part_count', 'load_accepted', 'released', 'instance'])
    assert.equal(value[key], undefined, key);
}

test('insert-chain lifecycles use actual folder primary, exact3/2/2 arity and trusted context', () => {
  const h = harness();
  for (const action of ['capture', 'restore', 'release']) {
    const result = h.component.insertChainState({ channel: 'Scratch', action, expected_folder: folderUid,
      ...(action === 'capture' ? {} : { token }) });
    assert.equal(result.ok, true);
    assert.equal(result.verified, false); assert.equal(result.effects_verified, false); assert.equal(result.atomic_snapshot, false);
    assert.equal(result.native_outcome, action === 'capture' ? 'observed' : 'applied_unverified');
    const args = h.calls.at(-1);
    assert.equal(args[0], h.folder); assert.equal(args.length, action === 'capture' ? 3 : 2);
    assert.deepEqual(JSON.parse(args[1]), { operation: `insert_chain_${action}`, bridge_session: 'chain', document_epoch: 'chain:1',
      expected_folder: folderUid, ...(action === 'capture' ? {} : { token }) });
    if (action === 'capture') assert.equal(args[2], h.urls[0]);
  }
  assert.equal(h.urls.length, 1); assert.equal(h.urls[0].value, '');
  assert.deepEqual(h.resolutions, ['Scratch', 'Inserts', 'Scratch', 'Inserts', 'Scratch', 'Inserts']);
});

test('each capture creates a fresh empty URL and accepts exact inclusive file and part bounds', () => {
  const h = harness({ result(value) { value.byte_count = 1; value.part_count = 0;
    value.sha256 = 'A'.repeat(64); value.folder.class_uid = folderUid; } });
  for (let index = 0; index < 2; index++) {
    const result = h.component.insertChainState({ channel: '🎸'.repeat(128), action: 'capture' });
    assert.equal(result.ok, true); assert.equal(result.byte_count, 1); assert.equal(result.part_count, 0);
    assert.equal(result.folder.class_uid, folderUid); assert.equal(result.sha256, 'A'.repeat(64));
  }
  assert.equal(h.urls.length, 2); assert.notEqual(h.urls[0], h.urls[1]);
  assert.equal(h.calls[0][2], h.urls[0]); assert.equal(h.calls[1][2], h.urls[1]);
});

test('finite lifecycle payloads reject before any channel or native resolution', () => {
  const h = harness();
  for (const extra of [{ channel: '' }, { channel: 'x'.repeat(257) }, { channel: 'bad\nchannel' }, { action: 'load' },
    { token }, { token: null }, { expected_folder: null }, { expected_folder: folderUid.toLowerCase() },
    { expected_folder: '00000000-0000-0000-0000-000000000000' }, { slot: 0 }, { plugin: 'Pro EQ' },
    { expected_instance: folderUid }, { path: '/private/preset' }, { bytes: 'archive' }, { source: 'dsp' },
    { bridge_session: 'forged' }, { document_epoch: 'forged' }, { expected_document: 'forged' },
    { channel: '🎸'.repeat(129) }]) {
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'capture', ...extra });
    assert.equal(result.native_outcome, 'not_applied'); noClaims(result);
  }
  for (const action of ['restore', 'release']) for (const value of [undefined, '', 'a'.repeat(64), 'A'.repeat(63), false, null])
    assert.equal(h.component.insertChainState({ channel: 'Scratch', action, token: value }).native_outcome, 'not_applied');
  assert.equal(h.resolutions.length, 0); assert.equal(h.calls.length, 0); assert.equal(h.urls.length, 0);
});

test('folder-null class is accepted separately and Device-null class remains rejected', () => {
  const h = harness();
  assert.equal(h.component.insertChainState({ channel: 'Scratch', action: 'capture' }).folder.class_uid, null);
  const legacyHost = harness({ rawResult: JSON.stringify({ ok: true, instance: { object_uid: folderUid, class_uid: null } }) });
  const legacy = legacyHost.component.nativePluginRequest({}, { operation: 'inspect' });
  assert.equal(legacy.ok, false); assert.equal(legacy.outcome, 'unknown');
});

test('missing lease, stale context and incompatible capabilities stop before dispatch', () => {
  for (const hook of ['channel', 'folder', 'lookup', 'capability', 'url']) {
    const h = harness({ [hook](context) { context.document = 'chain:2'; } });
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'capture' });
    assert.equal(result.native_outcome, 'not_applied', hook); assert.equal(h.calls.length, 0); noClaims(result);
  }
  for (const capabilityFields of [{ insert_chain_api: undefined }, { insert_chain_api: 'old' }, { api: 'old' }]) {
    const h = harness({ capabilityFields });
    assert.equal(h.component.insertChainState({ channel: 'Scratch', action: 'capture' }).native_outcome, 'not_applied');
    assert.equal(h.calls.length, 0); assert.equal(h.urls.length, 0);
  }
  const h = harness(); h.component.bridge.requestContext = null;
  assert.equal(h.component.insertChainState({ channel: 'Scratch', action: 'capture' }).native_outcome, 'not_applied');
  assert.equal(h.resolutions.length, 0);
});

test('closed or malformed trusted context cannot resolve any folder or dispatch native code', () => {
  for (const change of [{ open: false }, { session: '' }, { session: 'x'.repeat(129) },
    { document: false }, { document: 'x'.repeat(129) }, { session: 'bad\ncontext' }]) {
    const h = harness(); Object.assign(h.current, change);
    h.component.bridge.requestContext = { ...h.current };
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'capture' });
    assert.equal(result.native_outcome, 'not_applied'); noClaims(result);
    assert.equal(h.resolutions.length, 0); assert.equal(h.calls.length, 0);
  }
});

test('duplicate or missing mixer channel never falls back to selected Device', () => {
  const h = harness({ channelResult: { bridgeError: 'ambiguous channel' } });
  assert.equal(h.component.insertChainState({ channel: 'Scratch', action: 'capture' }).native_outcome, 'not_applied');
  assert.equal(h.calls.length, 0); assert.deepEqual(h.resolutions, ['Scratch']);
});

test('malformed or post-context native replies lose all ownership and success claims', () => {
  const mutations = [v => { v.folder = null; }, v => { delete v.folder.class_uid; }, v => { v.folder.object_uid = 'bad'; },
    v => { v.folder.object_uid = '87654321-4321-4321-4321-CBA987654321'; }, v => { v.folder.class_uid = 'bad'; },
    v => { v.folder.class_uid = '00000000-0000-0000-0000-000000000000'; },
    v => { v.folder.native_class = 'Device'; }, v => { v.folder.object_id = 'Other'; }, v => { v.instance = {}; },
    v => { v.path = '/private/preset'; }, v => { v.scope = 'plugin_state'; }, v => { v.format = 'unknown'; },
    v => { v.version = true; }, v => { v.verified = true; }, v => { v.effects_verified = true; },
    v => { v.atomic_snapshot = true; }, v => { v.token = 'a'.repeat(64); }, v => { v.byte_count = 0; },
    v => { v.byte_count++; }, v => { v.byte_count = 0.5; }, v => { v.byte_count = true; },
    v => { v.part_count = 129; }, v => { v.part_count = false; }, v => { v.part_count = -1; }, v => { v.part_count = 0.5; },
    v => { v.sha256 = 'broken'; }, v => { v.outcome = 'applied_verified'; }];
  for (const mutate of mutations) {
    const h = harness({ result: mutate });
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'capture', expected_folder: folderUid });
    assert.equal(result.native_outcome, 'unknown'); noClaims(result); assert.equal(h.calls.length, 1);
  }
  for (const options of [{ dispatch(c) { c.document = 'chain:2'; } }, { rawResult: '{bad' }, { rawResult: 'x'.repeat(262145) }]) {
    const h = harness(options);
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'capture' });
    assert.equal(result.native_outcome, 'unknown'); noClaims(result); assert.equal(h.calls.length, 1);
    assert.equal(JSON.stringify(result).includes('/private/'), false);
  }
});

test('restore acceptance and release acknowledgement remain unverified and cannot swap tokens', () => {
  for (const action of ['restore', 'release']) for (const mutate of [v => { v.token = 'B'.repeat(64); },
    v => { v[action === 'restore' ? 'load_accepted' : 'released'] = false; },
    v => { v[action === 'restore' ? 'released' : 'byte_count'] = true; }]) {
    const h = harness({ result: mutate });
    const result = h.component.insertChainState({ channel: 'Scratch', action, token });
    assert.equal(result.native_outcome, 'unknown'); noClaims(result); assert.equal(h.calls.length, 1);
  }
});

test('native failure receipts preserve known phase outcome without metadata or private error text', () => {
  for (const outcome of ['not_applied', 'unknown']) {
    const h = harness({ rawResult: JSON.stringify({ ok: false, outcome, error: { code: 'failed', message: '/private/preset' } }) });
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'capture' });
    assert.equal(result.native_outcome, outcome); noClaims(result);
    assert.equal(JSON.stringify(result).includes('/private/preset'), false);
  }
  for (const extra of [{ token }, { folder: folderIdentity() }, { phase: 'load' }, { outcome: 'applied_verified' }]) {
    const h = harness({ rawResult: JSON.stringify({ ok: false, outcome: 'not_applied', error: { code: 'failed', message: 'failed' }, ...extra }) });
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'capture' });
    assert.equal(result.native_outcome, 'unknown'); noClaims(result);
  }
});

test('insert-chain failures use the Runtime native outcome contract through BridgeCore', () => {
  for (const action of ['capture', 'restore', 'release']) for (const outcome of ['not_applied', 'unknown']) {
    const h = harness({ rawResult: JSON.stringify({ ok: false, outcome,
      error: { code: 'failed', message: '/private/preset' } }) });
    const host = bridgeHarness();
    host.bridge.component = h.component;
    h.component.bridge = Object.assign(host.bridge, { mixerChannel() { return { find() { return h.folder; } }; } });
    const response = host.writeRequest({ op: 'insertChainState', args: { channel: 'Scratch', action,
      expected_folder: folderUid, ...(action === 'capture' ? {} : { token }) } });
    assert.equal(response.ok, true);
    const result = response.result;
    assert.deepEqual(Object.keys(result).sort(), ['errors', 'native_outcome']);
    assert.equal(result.error, undefined);
    assert.equal(result.native_outcome, outcome);
    assert.deepEqual(result.errors, [{ code: 'native_insert_chain_failed',
      message: 'The guarded native insert-chain request did not complete' }]);
    noClaims(result);
    assert.equal(JSON.stringify(result).includes('/private/preset'), false);
    assert.equal(h.calls.length, 1);
  }
});

test('failure normalization cannot leak supplied or malformed native capsule claims', () => {
  for (const extra of [{ token }, { folder: folderIdentity() }, { sha256: 'a'.repeat(64) },
    { byte_count: 1 }, { part_count: 1 }, { load_accepted: true }, { released: true }]) {
    const h = harness({ rawResult: JSON.stringify({ ok: false, outcome: 'not_applied',
      error: { code: 'failed', message: '/private/preset' }, ...extra }) });
    const result = h.component.insertChainState({ channel: 'Scratch', action: 'restore', token,
      expected_folder: folderUid });
    assert.deepEqual(Array.from(Object.keys(result).sort()), ['errors', 'native_outcome']);
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.errors.length, 1);
    assert.equal(result.error, undefined);
    noClaims(result);
    assert.equal(JSON.stringify(result).includes('/private/preset'), false);
    assert.equal(h.calls.length, 1);
  }
  const h = harness({ dispatch(context) { context.document = 'chain:2'; } });
  const result = h.component.insertChainState({ channel: 'Scratch', action: 'restore', token,
    expected_folder: folderUid });
  assert.deepEqual(Array.from(Object.keys(result).sort()), ['errors', 'native_outcome']);
  assert.equal(result.native_outcome, 'unknown');
  assert.equal(result.error, undefined); noClaims(result);
  assert.equal(h.calls.length, 1);
});

test('missing insert-chain extension returns a known refusal without a top-level soft error', () => {
  const h = harness();
  vm.runInContext('NativePluginControls = undefined', h.scope);
  const result = h.component.insertChainState({ channel: 'Scratch', action: 'restore', token });
  assert.deepEqual(Array.from(Object.keys(result).sort()), ['errors', 'native_outcome']);
  assert.equal(result.native_outcome, 'not_applied');
  assert.equal(result.error, undefined);
  assert.equal(result.errors[0].code, 'native_required'); noClaims(result);
  assert.equal(h.calls.length, 0);
});

test('BridgeCore requires session and document intents for every lifecycle action', () => {
  const host = bridgeHarness(); let calls = 0;
  host.bridge.component = { insertChainState() { calls++; return reply('capture'); } };
  for (const action of ['capture', 'restore', 'release']) {
    const args = { channel: 'Scratch', action, ...(action === 'capture' ? {} : { token }) };
    assert.equal(host.request({ op: 'insertChainState', args }).code, 'context_required');
    assert.equal(host.writeRequest({ op: 'insertChainState', args, expectedDocument: 'stale' }).code, 'document_changed');
    assert.equal(host.writeRequest({ op: 'insertChainState', args }).ok, true);
  }
  assert.equal(calls, 3);
});

test('BridgeCore strips a previously valid capsule if the document changes before publication', () => {
  const host = bridgeHarness();
  host.bridge.component = { insertChainState() {
    host.manager.activeDocument = { title: 'Other' };
    return { ...reply('capture'), native_outcome: 'observed' };
  } };
  const result = host.writeRequest({ op: 'insertChainState', args: { channel: 'Scratch', action: 'capture' } }).result;
  assert.equal(result.native_outcome, 'unknown'); noClaims(result);
});
