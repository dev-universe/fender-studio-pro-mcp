import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: 'D2D9B002-12FC-4F1D-AFDA-70336D1DC108', native_class: 'Chorder', object_id: 'Chorder' };
const target = { track: 'Scratch', slot: 0, expected_instance: instance.object_uid };
const mode = enabled => ({ kind: 'learn_mode', enabled });
const input = pitch => ({ kind: 'input_pitch', pitch });
function bits(number, width) {
  const view = new DataView(new ArrayBuffer(width / 8));
  if (width === 64) { view.setFloat64(0, number); return view.getBigUint64(0).toString(16).padStart(16, '0').toUpperCase(); }
  view.setFloat32(0, number); return view.getUint32(0).toString(16).padStart(8, '0').toUpperCase();
}
function description(name, value, float = false) {
  const pitch = name === 'learnInputPitch';
  const typed = number => float ? { variant_tag: 2, data: { kind: 'float', value: number, bits_hex: bits(number, 64) } }
    : { variant_tag: 1, data: { kind: 'integer', value: number } };
  const normalized = Math.fround(pitch ? (value + 1) / 128 : value);
  return { name, title: name, tag: pitch ? 6 : 5, value, text: String(value), min: pitch ? -1 : 0, max: pitch ? 127 : 1,
    default: pitch ? -1 : 0, normalized, enabled: true, readOnly: false, writable: true, native_kind: pitch ? 'Integer' : 'Toggle',
    native_value: typed(value), native_min: typed(pitch ? -1 : 0), native_max: typed(pitch ? 127 : 1), native_default: typed(pitch ? -1 : 0),
    native_normalized: { kind: 'float32', bits_hex: bits(normalized, 32) }, native_support: { raw_number: true, normalized: true, display_text: true },
    support: { raw: true, normalized: true, text: true }, display_parser: pitch ? 'NativeFormatter { flags: 0 }' : 'PlainNumeric' };
}
function observed(views = 125) {
  return { ok: true, outcome: 'observed', operation: 'chorder_learning_get', instance: { ...instance }, receiver: 'editor_controller',
    mutation_capability: false, value_semantics: 'sequential_snapshot', binding: { editor_match: 'canonical_iunknown', autofill_same_gui: true, controller_same_observer: true },
    coverage: { complete: true, candidate_count: 1, visited_views: views, max_views: 256, max_depth: 32 },
    parameters: { learnMode: description('learnMode', 0), learnInputPitch: description('learnInputPitch', -1) }, storage_semantics: 'transient_learning_controls' };
}
function edited(change = input(0), float = false) {
  const name = change.kind === 'learn_mode' ? 'learnMode' : 'learnInputPitch';
  const value = change.kind === 'learn_mode' ? Number(change.enabled) : change.pitch === null ? -1 : change.pitch;
  const before = change.kind === 'learn_mode' ? 1 - value : value === -1 ? 0 : -1;
  return { ok: true, outcome: 'applied_unverified', operation: 'edit_chorder_model', instance: { ...instance }, binding_source: 'dsp',
    receiver: 'editor_controller', requested_change: { ...change }, edit_phase: 'closed_checked', gui_parameter: { name,
      before: description(name, before, float), after: description(name, value, float) }, gui_value_verified: true,
    verification_scope: 'gui_receiver_current_value', persistence_verified: false, effects_verified: false, dsp_propagation_verified: false,
    storage_semantics: 'transient_learning_controls' };
}
function harness(reply = request => request.operation === 'chorder_learning_get' ? observed() : edited(request.change), capability = 'chorder_learning_controls_v1') {
  const calls = [], targets = [];
  const context = { open: true, document: 'learning:1', session: 'learning' };
  const service = { capabilities() { return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1', gui_model_edit_api: capability }); },
    request(device, body) { const request = JSON.parse(body); calls.push(request); const result = reply(request, context); return typeof result === 'string' ? result : JSON.stringify(result); } };
  const scope = vm.createContext({ include_file() {}, memberTypes() { return {}; }, PreSonus: { ControlSurfaceComponent: class {} }, Host: { Objects: { getObjectByName() { return service; } } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  component.pluginOf = args => { targets.push(args); return { name: 'Chorder', device: {} }; };
  return { component, context, calls, targets, service };
}
function noPositive(result) {
  for (const key of ['parameters', 'binding', 'coverage', 'gui_parameter', 'requested_change', 'storage_semantics']) assert.equal(key in result, false, key);
  assert.notEqual(result.gui_value_verified, true);
}

test('learning getter reads only fixed GUI fields with a measured hard ceiling and no editable authority', () => {
  for (const views of [1, 125, 256]) {
    const h = harness(() => observed(views));
    const result = h.component.chorderLearningGet(target);
    assert.deepEqual(h.calls, [{ operation: 'chorder_learning_get', expected_instance: instance.object_uid }]);
    assert.equal(JSON.stringify(h.targets), JSON.stringify([{ kind: 'note_effect', track: 'Scratch', slot: 0, plugin: 'Chorder' }]));
    assert.equal(result.native_outcome, 'observed'); assert.equal(result.mutation_capability, false);
    assert.equal(result.parameters.learnInputPitch.value, -1); assert.equal(result.coverage.visited_views, views);
    assert.equal(result.coverage.max_views, 256); assert.equal(result.value_semantics, 'sequential_snapshot');
  }
});

test('learning setter preserves null, MIDI zero and endpoints without implicit mode changes or DSP offset', () => {
  for (const change of [mode(false), mode(true), input(null), input(0), input(127)]) {
    const h = harness(); const result = h.component.chorderLearningSet({ ...target, change });
    assert.deepEqual(h.calls, [{ operation: 'edit_chorder_model', expected_instance: instance.object_uid, change }]);
    assert.equal(result.native_outcome, 'applied_unverified'); assert.equal(result.gui_value_verified, true);
    assert.equal(result.gui_parameter.after.value, change.kind === 'learn_mode' ? Number(change.enabled) : change.pitch === null ? -1 : change.pitch);
    for (const flag of ['persistence_verified', 'effects_verified', 'dsp_propagation_verified']) assert.equal(result[flag], false);
    assert.equal('verified' in result, false); assert.equal(h.calls.length, 1);
  }
  for (const enabled of [false, true]) {
    const change = mode(enabled); const h = harness(() => edited(change, true));
    assert.equal(h.component.chorderLearningSet({ ...target, change }).gui_value_verified, true);
  }
});

test('closed learning intents and targets refuse before Device resolution or native dispatch', () => {
  const h = harness(); h.component.pluginOf = () => { throw Error('invalid intent resolved'); };
  for (const change of [{ kind: 'input_pitch' }, input(-1), input(128), input(0.5), input('0'), input(false), input(undefined),
    { kind: 'input_pitch', pitch: null, enabled: false }, { kind: 'learn_mode' }, mode(1), mode(null),
    { kind: 'learn_mode', enabled: true, pitch: 0 }, { kind: 'other', pitch: 0 }, {}]) {
    assert.equal(h.component.chorderLearningSet({ ...target, change }).native_outcome, 'not_applied');
  }
  for (const extra of [{ track: '' }, { track: '  ' }, { track: 'x\0y' }, { track: 'x'.repeat(257) }, { slot: -1 }, { slot: 16 }, { slot: 0.5 },
    { expected_instance: '' }, { expected_instance: instance.object_uid.toLowerCase() }, { expected_instance: '00000000-0000-0000-0000-000000000000' },
    ...['kind', 'plugin', 'source', 'param', 'path', 'budget', 'max_views', 'channel'].map(key => ({ [key]: 'unexpected' }))]) {
    assert.equal(h.component.chorderLearningGet({ ...target, ...extra }).native_outcome, 'not_applied');
    assert.equal(h.component.chorderLearningSet({ ...target, change: input(0), ...extra }).native_outcome, 'not_applied');
  }
  assert.equal(h.calls.length, 0);
});

test('learning controls require the matching capability, open document and exact resolved class', () => {
  for (const capability of [undefined, null, '', true, 'chorder_autofill_v1']) {
    const h = harness(undefined, capability);
    if (capability === undefined) h.service.capabilities = () => '{"ok":true,"api":"fender-mcp-native-v1"}';
    assert.equal(h.component.chorderLearningGet(target).native_outcome, 'not_applied');
    assert.equal(h.component.chorderLearningSet({ ...target, change: input(0) }).native_outcome, 'not_applied');
    assert.equal(h.calls.length, 0);
  }
  const wrong = harness(); wrong.component.pluginOf = () => ({ name: 'Other', device: {} });
  assert.equal(wrong.component.chorderLearningGet(target).native_outcome, 'not_applied');
  assert.equal(wrong.component.chorderLearningSet({ ...target, change: mode(true) }).native_outcome, 'not_applied');
  const closed = harness(); closed.context.open = false;
  assert.equal(closed.component.chorderLearningGet(target).native_outcome, 'not_applied');
  assert.equal(closed.component.chorderLearningSet({ ...target, change: input(0) }).native_outcome, 'not_applied');
  assert.equal(closed.calls.length, 0);
});

test('malformed learning reads erase all observed fields and never become mutation uncertainty', () => {
  const mutations = [r => { r.ok = 1; }, r => { r.outcome = 'applied_verified'; }, r => { r.source = 'dsp'; }, r => { r.token = 'callable'; },
    r => { r.receiver = 'dsp'; }, r => { r.mutation_capability = true; }, r => { r.value_semantics = 'verified'; },
    r => { r.binding.editor_match = 'caption'; }, r => { r.binding.autofill_same_gui = false; }, r => { r.binding.pointer = 1; },
    r => { r.coverage.complete = false; }, r => { r.coverage.candidate_count = 2; }, r => { r.coverage.visited_views = 257; }, r => { r.coverage.max_views = 128; },
    r => { r.parameters.learnInputPitch.name = 'chorder.learnInputPitch'; }, r => { r.parameters.learnInputPitch.value = 0; },
    r => { r.parameters.learnMode.native_value.variant_tag = 2; }, r => { delete r.parameters.learnMode; },
    r => { r.instance.object_uid = '87654321-4321-4321-4321-CBA987654321'; }, r => { r.instance.class_uid = instance.object_uid; },
    r => { r.error = { code: 'failed', message: 'read failed' }; }, r => { r.native_error = 'failed'; }, r => { r.errors = ['failure']; },
    r => { r.storage_semantics = 'persistent_model'; }];
  for (const mutate of mutations) {
    const h = harness(() => { const result = observed(); mutate(result); return result; });
    const result = h.component.chorderLearningGet(target); assert.equal(result.native_outcome, 'not_applied'); noPositive(result);
    assert.equal(h.calls.length, 1);
  }
});

test('setter receipt contradictions become unknown once dispatched and retain no GUI-current positives', () => {
  const mutations = [r => { r.ok = 1; }, r => { r.outcome = 'applied_verified'; }, r => { r.edit_phase = 'end_dispatched'; },
    r => { r.binding_source = 'controller'; }, r => { r.receiver = 'dsp'; }, r => { r.requested_change.pitch = 1; },
    r => { r.requested_change = mode(true); }, r => { r.gui_parameter.name = 'chorder.learnInputPitch'; },
    r => { r.gui_parameter.after.native_kind = 'Toggle'; }, r => { r.gui_parameter.after.tag = 7; },
    r => { r.gui_parameter.after.native_value.variant_tag = 17; }, r => { r.gui_parameter.after.value = 1; },
    r => { r.gui_parameter.after.native_default.data.value = 0; }, r => { r.gui_value_verified = 'true'; },
    r => { r.gui_value_verified = false; }, r => { r.persistence_verified = true; }, r => { r.effects_verified = true; },
    r => { r.dsp_propagation_verified = true; }, r => { r.verified = true; }, r => { r.error = { code: 'failed', message: 'late' }; },
    r => { r.instance.object_uid = '87654321-4321-4321-4321-CBA987654321'; }, r => { r.instance.class_uid = instance.object_uid; },
    r => { r.gui_parameter.before.extra = true; }, r => { r.native_outcome = 'not_applied'; }];
  for (const mutate of mutations) {
    const h = harness(() => { const result = edited(); mutate(result); return result; });
    const result = h.component.chorderLearningSet({ ...target, change: input(0) }); assert.equal(result.native_outcome, 'unknown'); noPositive(result);
    assert.equal(h.calls.length, 1);
  }
  const mismatch = harness(() => { const result = edited(); result.gui_parameter.after = description('learnInputPitch', 1); result.gui_value_verified = false; return result; });
  const result = mismatch.component.chorderLearningSet({ ...target, change: input(0) });
  assert.equal(result.native_outcome, 'applied_unverified'); assert.equal(result.gui_value_verified, false);
});

test('known refused and unknown learning phases cannot fabricate a successful value or balancing retry', () => {
  for (const [outcome, edit_phase] of [['not_applied', undefined], ['not_applied', 'prepared'], ['unknown', 'begin_dispatched'],
    ['unknown', 'value_dispatched'], ['unknown', 'end_dispatched'], ['unknown', 'closed_checked']]) {
    const h = harness(() => ({ ok: false, outcome, ...(edit_phase ? { edit_phase } : {}), error: { code: 'stopped', message: 'binding lost' } }));
    const result = h.component.chorderLearningSet({ ...target, change: input(0) }); assert.equal(result.native_outcome, outcome); noPositive(result);
    assert.equal(h.calls.length, 1);
  }
  for (const bad of [{ ok: false, outcome: 'not_applied', edit_phase: 'value_dispatched', error: { code: 'bad', message: 'contradiction' } },
    { ok: false, outcome: 'unknown', gui_parameter: edited().gui_parameter, error: { code: 'bad', message: 'positive leak' } }, null, '{']) {
    const h = harness(() => bad); const result = h.component.chorderLearningSet({ ...target, change: input(0) });
    assert.equal(result.native_outcome, 'unknown'); noPositive(result); assert.equal(h.calls.length, 1);
  }
});

test('inner document changes clear learning reads and edits after one dispatch', () => {
  for (const setter of [false, true]) {
    const h = harness((request, context) => { context.document = 'learning:2'; return setter ? edited(request.change) : observed(); });
    const result = setter ? h.component.chorderLearningSet({ ...target, change: input(0) }) : h.component.chorderLearningGet(target);
    assert.equal(result.native_outcome, setter ? 'unknown' : 'not_applied'); noPositive(result); assert.equal(h.calls.length, 1);
  }
});

test('outer bridge requires explicit context for learning reads and strips all late GUI evidence', () => {
  for (const setter of [false, true]) {
    const op = setter ? 'chorderLearningSet' : 'chorderLearningGet', args = setter ? { ...target, change: input(0) } : target;
    const host = bridgeHarness(); let calls = 0;
    host.bridge.component = { [op]() { calls++; host.manager.activeDocument = { title: 'Different' }; return setter ? edited() : observed(); } };
    const missing = host.request({ op, args }); assert.equal(missing.ok, false); assert.equal(calls, 0);
    const result = host.writeRequest({ op, args }); assert.equal(calls, 1); assert.equal(result.ok, false); assert.equal(result.code, 'native_context_lost');
    assert.equal(result.result, undefined);
  }
});
