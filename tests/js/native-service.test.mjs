import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset } from './host.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: '87654321-4321-4321-4321-CBA987654321' };
const success = (extra = {}) => JSON.stringify({ ok: true, instance, ...extra });
function harness(objects) {
  const scope = vm.createContext({ include_file() {}, memberTypes() { return {}; }, PreSonus: { ControlSurfaceComponent: class {} }, ...(objects ? { Host: { Objects: objects } } : {}) });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: { open: true, document: 'contract:1', session: 'contract' }, context() { return { open: true, document: 'contract:1', session: 'contract' }; } };
  const device = { findParameter() { return null; } };
  component.pluginOf = () => ({ name: 'Expected', device });
  return { component, device };
}
function native(request) {
  return { capabilities() { return '{"ok":true,"api":"fender-mcp-native-v1"}'; }, request };
}

test('native calls preserve the exact host object and use the fixed registered service', () => {
  const lookups = [], calls = [];
  const h = harness({ getObjectByName(name) { lookups.push(name); return native((target, body) => { calls.push([target, JSON.parse(body)]); return success(); }); } });
  assert.equal(h.component.pluginInspect({}).native.ok, true);
  assert.equal(calls[0][0], h.device);
  assert.deepEqual(calls[0][1], { operation: 'inspect' });
  assert.deepEqual(lookups, ['FenderMcpNative']);
});

test('fixed object URL is the only fallback lookup and missing extension remains optional', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return null; }, getObjectByUrl(url, optional) { calls.push([url, optional]); return native(() => success()); } });
  assert.equal(h.component.nativePluginRequest(h.device, { operation: 'inspect' }).ok, true);
  assert.deepEqual(calls, [['object://FenderMcpNative/', true]]);
  assert.equal(harness().component.nativePluginRequest({}, { operation: 'inspect' }), null);
  assert.match(harness().component.pluginParams({ limit: 64 }).error, /requires/);
});

test('mismatched plugin identity and unsupported capabilities never dispatch a native target', () => {
  let calls = 0;
  const h = harness({ getObjectByName() { calls++; return native(() => { throw new Error('must not call'); }); } });
  h.component.pluginOf = () => ({ error: 'identity changed' });
  assert.match(h.component.pluginInspect({}).error, /identity/);
  assert.equal(calls, 0);
  const bad = harness({ getObjectByName() { return { capabilities() { return '{"ok":true,"api":"other"}'; }, request() { calls++; } }; } });
  assert.equal(bad.component.nativePluginRequest({}, { operation: 'inspect' }).outcome, 'not_applied');
  assert.equal(calls, 0);
});

test('malformed or oversized native replies preserve an uncertain dispatched outcome', () => {
  for (const value of [undefined, 'not json', '[]', 'null', 'x'.repeat(262145), '{"ok":true}']) {
    const h = harness({ getObjectByName() { return native(() => value); } });
    const result = h.component.nativePluginRequest(h.device, { operation: 'set_parameter' });
    assert.equal(result.ok, false);
    assert.equal(result.outcome, 'unknown');
    assert.ok(result.error.message.length <= 512);
  }
});

test('document changes during a native call cannot become verified success', () => {
  const context = { open: true, document: 'a:1', session: 'a' };
  const h = harness({ getObjectByName() { return native(() => { context.document = 'a:2'; return success({ before: { value: 1 }, after: { value: 0.5 }, verified: true }); }); } });
  h.component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  const result = h.component.nativePluginRequest(h.device, { operation: 'set_parameter' });
  assert.equal(result.outcome, 'unknown');
  assert.equal(result.before.value, 1);
  assert.equal(result.after.value, 0.5);
});

test('native dispatch requires the original bridge lease through service lookup and capabilities', () => {
  let dispatched = 0;
  const h = harness({ getObjectByName() { return native(() => { dispatched++; return success(); }); } });
  h.component.bridge = null;
  assert.equal(h.component.nativePluginRequest(h.device, { operation: 'set_parameter' }).outcome, 'not_applied');
  assert.equal(dispatched, 0);
  const context = { open: true, document: 'a:1', session: 'a' };
  const switched = harness({ getObjectByName() { return { capabilities() { context.document = 'a:2'; return '{"ok":true,"api":"fender-mcp-native-v1"}'; }, request() { dispatched++; return success(); } }; } });
  switched.component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  assert.equal(switched.component.nativePluginRequest(switched.device, { operation: 'set_parameter' }).outcome, 'not_applied');
  assert.equal(dispatched, 0);
});

test('DSP pages merge native indices with live values and expose unreadable names', () => {
  const h = harness({ getObjectByName() { return native((_, body) => {
    assert.deepEqual(JSON.parse(body), { operation: 'parameters', source: 'dsp', offset: 64, limit: 2 });
    return success({ source: 'dsp', reported_count: 66, offset: 64, next_offset: null, page_complete: true, all_indices_read: true, parameters: [{ index: 64, name: 'gain', tag: 12 }, { index: 65, name: 'meter', tag: 13 }] });
  }); } });
  h.device.findParameter = name => name === 'gain' ? { name, value: 1, min: 0, max: 2 } : null;
  const result = h.component.pluginParams({ source: 'dsp', offset: 64, limit: 2 });
  assert.equal(result.reported_count, 66);
  assert.equal(result.params[0].native.index, 64);
  assert.equal(result.params[0].value, 1);
  assert.equal(result.params[1].native.index, 65);
  assert.equal(result.params[1].readable, false);
  assert.deepEqual(Array.from(result.missing), ['meter']);
  assert.equal(result.complete, false);
});

test('empty native names and failed indices remain inspectable metadata', () => {
  const h = harness({ getObjectByName() { return native(() => success({ source: 'dsp', reported_count: 2, offset: 0, next_offset: null, page_complete: false, all_indices_read: false, parameters: [{ index: 0, name: '', title: 'Group', tag: 44 }], failed_indices: [{ index: 1, reason: 'copy failed' }] })); } });
  const result = h.component.pluginParams({});
  assert.equal(result.params[0].native.tag, 44);
  assert.equal(result.failed_indices[0].index, 1);
  assert.equal(result.page_complete, false);
  assert.equal(result.complete, false);
});

test('controller pages and named fallback reads retain native descriptors', () => {
  const descriptor = { name: 'inputGain', value: 1, min: 0, max: 4, text: '0 dB', writable: true };
  const h = harness({ getObjectByName() { return native((_, body) => {
    const request = JSON.parse(body);
    return request.operation === 'parameters' ? success({ source: 'controller', parameters: [descriptor], reported_count: 1, offset: 0, next_offset: null, page_complete: true, all_indices_read: true }) : success({ source: 'controller', parameter: descriptor });
  }); } });
  assert.equal(h.component.pluginParams({}).params[0].writable, true);
  assert.equal(h.component.pluginParams({ names: ['inputGain'] }).params[0].value, 1);
});

test('native writes run only when the script parameter was absent before any setter', () => {
  let nativeWrites = 0;
  const h = harness({ getObjectByName() { return native((_, body) => { const r = JSON.parse(body); assert.equal(r.operation, 'set_parameter'); nativeWrites++; return success({ source: 'controller', before: { value: 1 }, after: { value: 0.5 }, verified: true }); }); } });
  const result = h.component.setPluginParam({ plugin: 'Expected', param: 'inputGain', value: 0.5 });
  assert.equal(result.after.value, 0.5);
  assert.equal(nativeWrites, 1);
  h.device.findParameter = () => ({ name: 'inputGain', value: 1, min: 0, max: 4, setValue() { throw new Error('uncertain native setter'); } });
  assert.throws(() => h.component.setPluginParam({ plugin: 'Expected', param: 'inputGain', value: 0.5 }), /uncertain/);
  assert.equal(nativeWrites, 1);
});

test('expected native instance blocks script setters after same-class replacement', () => {
  let writes = 0;
  const h = harness({ getObjectByName() { return native(() => success()); } });
  h.device.findParameter = () => ({ value: 1, setValue() { writes++; } });
  assert.match(h.component.setPluginParam({ plugin: 'Expected', param: 'gain', value: 0, expected_instance: 'different' }).error, /instance/);
  assert.equal(writes, 0);
});

test('list choices preserve the exact native target and keep an empty hardware list explicit', () => {
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    assert.deepEqual(JSON.parse(body), { operation: 'choices', source: 'controller', param: 'sendPort', offset: 0, limit: 64, expected_instance: instance.object_uid });
    return success({ source: 'controller', choices: [], reported_count: 0, offset: 0, next_offset: null, page_complete: true });
  }); } });
  const result = h.component.pluginChoices({ plugin: 'Expected', param: 'sendPort', expected_instance: instance.object_uid });
  assert.equal(result.reported_count, 0);
  assert.equal(result.choices.length, 0);
  assert.equal(result.instance.object_uid, instance.object_uid);
  assert.match(harness().component.pluginChoices({ param: 'sendPort' }).error, /require/);
});
