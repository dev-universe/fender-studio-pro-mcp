import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: 'D2D9B002-12FC-4F1D-AFDA-70336D1DC108', native_class: 'Chorder', object_id: 'Chorder' };
const target = { track: 'Scratch', slot: 0, expected_instance: instance.object_uid, offset: 0, limit: 16 };
function page(request = target, pitches = () => []) {
  const count = Math.min(request.limit, 128 - request.offset), next = request.offset + count;
  return { ok: true, outcome: 'observed', operation: 'chorder_chords_get', scope: 'component_alias_IStorable_chorder_chords',
    schema: 'chorder_music_effect_preset_v1', serialization: 'component_alias_IStorable', instance: { ...instance }, source: 'component_alias',
    total: 128, offset: request.offset, limit: request.limit, count, next: next < 128 ? next : null,
    chords: Array.from({ length: count }, (_, i) => ({ input_pitch: request.offset + i, output_pitches: pitches(request.offset + i) })),
    capture_sha256: '1'.repeat(64), capture_byte_count: 512, model_sha256: 'a'.repeat(64), value_semantics: 'sequential_snapshot',
    mutation_capability: false, persistence_verified: false, audio_verified: false };
}
function harness(reply = request => page(request)) {
  const calls = [], targets = [], context = { open: true, document: 'chords:1', session: 'chords' };
  const service = { capabilities() { return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1' }); },
    request(device, body) { const request = JSON.parse(body); calls.push(request); const result = reply(request, context); return typeof result === 'string' ? result : JSON.stringify(result); } };
  const scope = vm.createContext({ include_file() {}, memberTypes() { return {}; }, PreSonus: { ControlSurfaceComponent: class {} }, Host: { Objects: { getObjectByName() { return service; } } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  component.pluginOf = args => { targets.push(args); return { name: 'Chorder', device: {} }; };
  return { component, calls, targets, context, service, controls: vm.runInContext('new NativePluginControls({})', scope) };
}
function noPositive(value) {
  for (const key of ['chords', 'model_sha256', 'capture_sha256', 'capture_byte_count', 'instance', 'scope', 'schema', 'total', 'next', 'token'])
    assert.equal(key in value, false, key);
  for (const flag of ['mutation_capability', 'persistence_verified', 'audio_verified']) assert.equal(value[flag], false);
}

test('stored chord pages include empty keys, sorted MIDI endpoints and exact fixed native requests', () => {
  for (const args of [target, { ...target, offset: 112 }, { ...target, offset: 127, limit: 16 }, { ...target, offset: 0, limit: 1 }]) {
    const h = harness(request => page(request, pitch => pitch === 0 ? [0, 64, 127] : []));
    const result = h.component.chorderChordsGet(args);
    assert.equal(result.ok, true);
    assert.deepEqual(h.calls, [{ operation: 'chorder_chords_get', expected_instance: args.expected_instance, offset: args.offset, limit: args.limit }]);
    assert.deepEqual(structuredClone(h.targets), [{ kind: 'note_effect', track: 'Scratch', slot: 0, plugin: 'Chorder' }]);
    assert.equal(result.chords.length, Math.min(args.limit, 128 - args.offset));
    assert.equal(result.chords[0].input_pitch, args.offset);
    assert.equal(result.next, args.offset + args.limit < 128 ? args.offset + args.limit : null);
    assert.equal(result.native_outcome, 'observed');
    for (const flag of ['mutation_capability', 'persistence_verified', 'audio_verified']) assert.equal(result[flag], false);
    assert.equal(result.output_pitch_mapping, undefined);
  }
});

test('the maximum complete MIDI membership list remains a bounded read-only page', () => {
  const h = harness(request => page(request, () => Array.from({ length: 128 }, (_, i) => i)));
  const result = h.component.chorderChordsGet(target);
  assert.equal(result.ok, true);
  assert.equal(result.chords.length, 16);
  assert.equal(result.chords[15].output_pitches.length, 128);
});

test('all input keys can be read without assuming pages are an atomic snapshot', () => {
  const h = harness();
  const inputs = [];
  for (let offset = 0; offset < 128; offset += 16) {
    const result = h.component.chorderChordsGet({ ...target, offset });
    inputs.push(...result.chords.map(chord => chord.input_pitch));
    assert.equal(result.value_semantics, 'sequential_snapshot');
    assert.equal(result.capture_sha256, '1'.repeat(64));
  }
  assert.deepEqual(inputs, Array.from({ length: 128 }, (_, i) => i));
  assert.equal(h.calls.length, 8);
});

test('closed chord read arguments refuse unsafe targets and arbitrary state inputs before resolution', () => {
  const bad = [{ track: '' }, { track: '  ' }, { track: 'bad\0name' }, { track: 'x'.repeat(257) }, { slot: -1 }, { slot: 16 }, { slot: 0.5 }, { slot: true },
    { expected_instance: '' }, { expected_instance: instance.object_uid.toLowerCase() }, { expected_instance: '00000000-0000-0000-0000-000000000000' },
    { offset: -1 }, { offset: 128 }, { offset: 0.5 }, { offset: true }, { offset: null }, { offset: '0' }, { offset: undefined },
    { limit: 0 }, { limit: 17 }, { limit: 0.5 }, { limit: true }, { limit: null }, { limit: '16' }, { limit: undefined },
    ...['source', 'kind', 'plugin', 'channel', 'xml', 'path', 'value', 'change', 'token', 'expected_model_sha256'].map(key => ({ [key]: 'unchecked' }))];
  for (const extra of bad) {
    const h = harness(); h.component.pluginOf = () => { throw Error('invalid request resolved'); };
    const result = h.component.chorderChordsGet({ ...target, ...extra });
    assert.equal(result.ok, false); noPositive(result); assert.equal(h.calls.length, 0);
  }
});

test('direct native chord requests cannot add unchecked data or omit required paging', () => {
  const h = harness();
  const request = { operation: 'chorder_chords_get', expected_instance: target.expected_instance, offset: 0, limit: 16 };
  for (const bad of [{ ...request, path: '/bad' }, { ...request, operation: 'load_chords' }, { ...request, offset: false },
    { ...request, limit: null }, { ...request, expected_instance: '' }, { ...request, xml: '<Attributes/>' }]) {
    const result = h.controls.request({}, bad);
    if (bad.operation === 'chorder_chords_get') { assert.equal(result.ok, false); noPositive(result); }
  }
  assert.equal(h.calls.length, 0);
});

test('wrong class, instance, malformed paging and forged verification claims lose the entire model', () => {
  const mutations = [
    value => { value.instance.class_uid = '00000000-0000-0000-0000-000000000000'; },
    value => { value.instance.object_uid = 'ABCDEFAB-1234-1234-1234-123456789ABC'; },
    value => { value.instance.object_id = 'x'.repeat(513); }, value => { value.instance.extra = true; },
    ...[['ok', 1], ['outcome', 'applied_verified'], ['operation', 'chorder_learning_get'], ['scope', 'other'], ['schema', 'unknown'],
      ['serialization', 'dsp'], ['source', 'dsp'], ['total', 127], ['offset', 1], ['limit', 1], ['count', 15], ['next', 17],
      ['capture_sha256', 'x'.repeat(64)], ['model_sha256', 'A'.repeat(64)], ['capture_byte_count', 1048577], ['capture_byte_count', 0],
      ['capture_byte_count', true], ['capture_byte_count', 1.5], ['value_semantics', 'atomic_snapshot'], ['mutation_capability', true],
      ['persistence_verified', true], ['audio_verified', true], ['token', 'unchecked'], ['error', { code: 'late', message: 'failure' }]]
      .map(([key, changed]) => value => { value[key] = changed; }),
    value => { delete value.model_sha256; }, value => { value.chords.pop(); }, value => { value.chords[0].input_pitch = 1; },
    value => { value.chords[0].output_pitches = [1, 1]; }, value => { value.chords[0].output_pitches = [2, 1]; },
    ...[[-1], [128], [0.5], [true], ['0'], [null], Array.from({ length: 129 }, (_, i) => i)]
      .map(outputs => value => { value.chords[0].output_pitches = outputs; }),
    value => { value.chords[0].extra = true; }, value => { value.chords[0] = null; }, value => { value.chords = {}; },
  ];
  for (const mutate of mutations) {
    const h = harness(request => { const value = page(request); mutate(value); return value; });
    const result = h.component.chorderChordsGet(target);
    assert.equal(result.ok, false); noPositive(result); assert.equal(h.calls.length, 1);
  }
});

test('unsupported serializer errors keep a bounded reason without partial model metadata', () => {
  const h = harness(() => ({ ...page(), ok: false, outcome: 'not_applied', error: { code: 'chorder_schema_unsupported', message: 'Unknown model attribute' } }));
  const result = h.component.chorderChordsGet(target);
  assert.equal(result.error.code, 'chorder_schema_unsupported'); assert.equal(result.native_outcome, 'not_applied'); noPositive(result);
});

test('native exceptions and malformed JSON remain read failures with no model evidence', () => {
  for (const reply of [() => { throw Error('late native error'); }, () => '{bad', () => '[]', () => JSON.stringify({ ...page(), native_error: 'late' })]) {
    const h = harness(reply), result = h.component.chorderChordsGet(target);
    assert.equal(result.ok, false); assert.equal(result.native_outcome, 'not_applied'); noPositive(result);
  }
});

test('document and session changes after the native call erase the model', () => {
  for (const mutate of [context => { context.document = 'other:1'; }, context => { context.session = 'other'; }, context => { context.open = false; }]) {
    const h = harness((request, context) => { mutate(context); return page(request); });
    const result = h.component.chorderChordsGet(target);
    assert.equal(result.ok, false); noPositive(result); assert.equal(h.calls.length, 1);
  }
});

test('the fixed class and current document are mandatory without opening an editor', () => {
  const wrong = harness(); wrong.component.pluginOf = () => ({ name: 'Other', device: {} });
  noPositive(wrong.component.chorderChordsGet(target)); assert.equal(wrong.calls.length, 0);
  const closed = harness(); closed.context.open = false;
  noPositive(closed.component.chorderChordsGet(target)); assert.equal(closed.calls.length, 0);
});

test('the outer bridge requires both context guards and strips late chord pages', () => {
  const host = bridgeHarness(); let calls = 0;
  host.bridge.component = { chorderChordsGet() { calls++; host.manager.activeDocument = { title: 'Different' }; return page(); } };
  const missing = host.request({ op: 'chorderChordsGet', args: target });
  assert.equal(missing.ok, false); assert.equal(calls, 0);
  const result = host.writeRequest({ op: 'chorderChordsGet', args: target });
  assert.equal(result.ok, false); assert.equal(result.code, 'native_context_lost'); assert.equal(result.result, undefined); assert.equal(calls, 1);
});
