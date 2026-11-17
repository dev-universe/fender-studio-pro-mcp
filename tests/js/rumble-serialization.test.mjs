import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';
const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: 'B9D7536C-A010-457D-A182-36E5531164C9', native_class: 'AudioEffect', object_id: 'Rumble Native' };
const target = { channel: 'Scratch', slot: 0, plugin: 'Rumble Native', expected_instance: instance.object_uid, action: 'inspect_rumble_serialization' };
function summary() {
  return { ok: true, outcome: 'observed', operation: 'rumble_serialization_inspect', schema: 'rumble_alias_serialization_summary_v1',
    serialization: 'component_alias_IStorable', instance: { ...instance }, source: 'component_alias', capture_sha256: 'a'.repeat(64), capture_byte_count: 64,
    format: 'utf8_xml', parse_complete: true, reason: null, root_kind: 'audio_effect_preset', counts: { known_tags: [1, 0, 0, 0], other_tags: 0,
      known_ids: Array(30).fill(0), classids_total: 0, classids_valid_guid: 0, classids_zero_guid: 0, classids_invalid: 0 },
    value_semantics: 'sequential_snapshot', mutation_capability: false, model_identity_verified: false, audio_verified: false, persistence_verified: false };
}
function harness(reply = () => summary()) {
  const calls = [], context = { open: true, document: 'rumble:1', session: 'rumble' };
  const service = { capabilities() { return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1' }); }, request(device, body) { const request = JSON.parse(body); calls.push(request); const value = reply(request, context); return typeof value === 'string' ? value : JSON.stringify(value); } };
  const scope = vm.createContext({ include_file() {}, memberTypes() { return {}; }, PreSonus: { ControlSurfaceComponent: class {} }, Host: { Objects: { getObjectByName() { return service; } } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope); vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: { ...context }, context() { return { ...context }; } }; component.pluginOf = () => ({ name: 'Rumble Native', device: {} });
  return { component, calls, context, controls: vm.runInContext('new NativePluginControls({})', scope) };
}
function noPositive(value) {
  for (const key of ['counts', 'format', 'root_kind', 'instance', 'schema', 'capture_sha256', 'capture_byte_count', 'token', 'source']) assert.equal(key in value, false, key);
  assert.equal(value.ok, false);
  for (const key of ['mutation_capability', 'model_identity_verified', 'audio_verified', 'persistence_verified']) assert.equal(value[key], false);
}
test('one finite state action returns only a bounded structure receipt and no token', () => {
  const h = harness(), result = h.component.pluginState(target);
  assert.equal(result.ok, true); assert.equal(result.native_outcome, 'observed'); assert.equal(result.plugin, 'Rumble Native'); assert.equal(result.token, undefined);
  assert.deepEqual(h.calls, [{ operation: 'rumble_serialization_inspect', expected_instance: instance.object_uid }]); assert.ok(Buffer.byteLength(JSON.stringify(result)) <= 4096);
});
test('opaque and ZIP observations preserve the closed model gate', () => {
  for (const [format, reason] of [['opaque', 'opaque'], ['zip_signature_only', 'unsupported_container'], ['utf8_json', 'duplicate_key']]) {
    const h = harness(() => ({ ...summary(), format, parse_complete: false, reason, counts: null, root_kind: null })); assert.equal(h.component.pluginState(target).ok, true);
  }
});
test('unchecked requests and wrong named targets are refused before native save', () => {
  for (const extra of [{ token: 'bad' }, { plugin: 'Other' }, { path: '/private' }, { bytes: 'raw' }, { expected_instance: '' }]) {
    const h = harness(); noPositive(h.component.pluginState({ ...target, ...extra })); assert.equal(h.calls.length, 0);
  }
  const h = harness(); h.component.pluginOf = () => ({ name: 'Other', device: {} }); noPositive(h.component.pluginState(target)); assert.equal(h.calls.length, 0);
  assert.equal(h.controls.rumbleSerializationRequest({ operation: 'rumble_serialization_inspect', expected_instance: instance.object_uid, token: 'bad' }), false);
});
test('forged native observations lose every positive field', () => {
  const mutations = [v => { v.instance.class_uid = 'OTHER'; }, v => { v.instance.native_class = 'Other'; }, v => { v.instance.object_id = 'Other'; }, v => { v.instance.object_uid = 'OTHER'; },
    v => { v.counts.classids_zero_guid = 1; }, v => { v.counts.known_ids[0] = true; }, v => { v.counts.known_ids.push(0); }, v => { v.counts.other_tags = 8193; },
    ...[['model_identity_verified', true], ['audio_verified', true], ['persistence_verified', true], ['mutation_capability', true], ['capture_byte_count', 16777217],
      ['format', 'malformed'], ['root_kind', 'private'], ['reason', 'private'], ['capture_sha256', 'bad'], ['schema', 'private'], ['token', 'private'], ['parse_complete', null], ['ok', false]]
      .map(([key, value]) => v => { v[key] = value; })];
  for (const mutate of mutations) { const h = harness(() => { const value = summary(); mutate(value); return value; }); noPositive(h.component.pluginState(target)); }
});
test('late native document and session drift erase the whole observation', () => {
  for (const mutate of [c => { c.document = 'other:1'; }, c => { c.session = 'other'; }, c => { c.open = false; }]) {
    const h = harness((request, context) => { mutate(context); return summary(); }); noPositive(h.component.pluginState(target));
  }
});
test('native exceptions and malformed transport responses return fixed private failures', () => {
  for (const reply of [() => { throw Error('private-path'); }, () => '{private', () => '[]']) { const h = harness(reply); const value = h.component.pluginState(target); noPositive(value); assert.equal(JSON.stringify(value).includes('private'), false); }
});
test('outer bridge requires fresh context and erases observations after final drift', () => {
  const h = bridgeHarness(); let calls = 0;
  h.bridge.component = { pluginState() { calls++; h.manager.activeDocument = { title: 'Other' }; return summary(); } };
  assert.equal(h.request({ op: 'pluginState', args: target }).ok, false); assert.equal(calls, 0);
  const value = h.writeRequest({ op: 'pluginState', args: target }); assert.equal(value.ok, false); assert.equal(value.code, 'native_context_lost'); assert.equal(value.result, undefined); assert.equal(calls, 1);
});
