import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: '87654321-4321-4321-4321-CBA987654321' };
const success = (extra = {}) => JSON.stringify({ ok: true, instance, ...extra });
function harness(objects, host = {}) {
  const scope = vm.createContext({ include_file() {}, memberTypes() { return {}; }, PreSonus: { ControlSurfaceComponent: class {} }, ...(objects ? { Host: { Objects: objects, ...host } } : {}) });
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

test('extended Pedalboard dispatch requires a rack fingerprint and its native proof', () => {
  const calls = [];
  let reportProof = true;
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    const request = JSON.parse(body);
    calls.push(request);
    return JSON.stringify({
      ok: true, outcome: 'applied_unverified', operation: request.operation, verified: false, effects_verified: false,
      instance: { ...instance, native_class: 'AudioEffect', class_uid: 'BC9129B4-EC67-41B8-96DE-EC128D5FE54D' },
      slots: [request.first_slot, request.second_slot], before: [request.expected_first_model, request.expected_second_model],
      after: [request.expected_second_model, request.expected_first_model],
      ...(reportProof && { serialized_payload_verified: true }),
    });
  }); } });
  h.component.pluginOf = () => ({ name: 'Pedalboard', device: h.device });
  const args = { channel: 'Scratch', slot: 0, expected_instance: instance.object_uid, first_slot: 1,
    second_slot: 3, expected_first_model: 'Compressor', expected_second_model: ' - ' };
  const fingerprint = 'ab'.repeat(32);
  for (const invalid of [args, { ...args, expected_rack_sha256: 'bad' },
    { ...args, expected_first_model: ' - ', expected_rack_sha256: fingerprint }]) {
    assert.equal(h.component.pedalboardSlotSwap(invalid).native_outcome, 'not_applied');
  }
  assert.equal(calls.length, 0);
  const expected = { ...args, expected_rack_sha256: fingerprint };
  const moved = h.component.pedalboardSlotSwap(expected);
  assert.equal(moved.native_outcome, 'applied_unverified');
  assert.equal(moved.serialized_payload_verified, true);
  assert.equal(calls[0].expected_rack_sha256, fingerprint);
  reportProof = false;
  assert.equal(h.component.pedalboardSlotSwap(expected).native_outcome, 'unknown');
  const ordinary = { ...args, second_slot: 2, expected_second_model: 'Reverb' };
  assert.equal(h.component.pedalboardSlotSwap(ordinary).native_outcome, 'applied_unverified');
  assert.equal(calls[2].expected_rack_sha256, undefined);
});

const actionResult = () => ({ ok: true, instance, action: 'swap_filter_frequencies', source: 'dsp',
  outcome: 'applied_unverified', verified: false, effects_verified: false,
  upper_dispatched: true, reset_dispatched: true, reset_verified: true,
  before: { value: 0 }, after: { value: 0 }, effect_parameters: ['filterlow', 'filterhigh'] });

const modelBase = { channel: 'Scratch', slot: 0, plugin: 'Expected', expected_instance: instance.object_uid, catalog: 'effects' };
const modelResult = (extra = {}) => ({ ok: true, outcome: 'observed',
  instance: { ...instance, class_uid: '11B8AA91-9BED-4A7F-90F4-02C6A6C7AF34' },
  catalog: 'effects', model_name: 'effectListModel', count: 2, offset: 0,
  rows: [{ index: 0, title: 'Clean', title_available: true }, { index: 1, title: 'Delay', title_available: true }],
  next_offset: null, complete: true, all_flat_rows_observed: true,
  scope: 'current_selector_flat_model', atomic_snapshot: false, mutation_capability: false, ...extra });

test('model lists forward only finite native catalogs with default and explicit bounded pages', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    const request = JSON.parse(body); calls.push(request);
    if (request.offset === 2) return JSON.stringify(modelResult({ offset: 2, count: 4,
      rows: [{ index: 2, title: 'Missing title', title_available: false }], next_offset: 3, complete: false, all_flat_rows_observed: false }));
    return JSON.stringify(modelResult(request.catalog === 'categories' ? { catalog: 'categories', model_name: 'categoryModel' } : {}));
  }); } });
  h.device.findParameter = () => { throw new Error('no script fallback or selection'); };
  for (const catalog of ['effects', 'categories']) {
    const result = h.component.pluginModels({ ...modelBase, catalog });
    assert.equal(result.ok, true); assert.equal(result.catalog, catalog);
    assert.equal(result.atomic_snapshot, false); assert.equal(result.mutation_capability, false);
    assert.equal(result.scope, 'current_selector_flat_model'); assert.equal(result.token, undefined);
  }
  const page = h.component.pluginModels({ ...modelBase, offset: 2, limit: 1 });
  assert.equal(page.complete, false); assert.equal(page.rows[0].title_available, false); assert.equal(page.next_offset, 3);
  assert.deepEqual(calls, [
    { operation: 'model_catalog', catalog: 'effects', offset: 0, limit: 32, expected_instance: instance.object_uid },
    { operation: 'model_catalog', catalog: 'categories', offset: 0, limit: 32, expected_instance: instance.object_uid },
    { operation: 'model_catalog', catalog: 'effects', offset: 2, limit: 1, expected_instance: instance.object_uid },
  ]);
});

test('model list selectors and invalid targets refuse before Device resolution or native lookup', () => {
  let resolutions = 0;
  const h = harness({ getObjectByName() { throw new Error('no native lookup'); } });
  h.component.pluginOf = () => { resolutions++; throw new Error('no Device resolution'); };
  for (const extra of [{ source: 'controller' }, { token: 'issued' }, { path: 'signalchain' }, { model_name: 'categoryModel' },
    { method: 'invoke' }, { classID: 'arbitrary' }, { native_address: 12 }, { selection: 0 },
    { catalog: 'cabinet' }, { catalog: undefined }, { plugin: '' }, { expected_instance: '' },
    { expected_instance: '00000000-0000-0000-0000-000000000000' }, { expected_instance: instance.object_uid.toLowerCase() },
    { offset: -1 }, { offset: 4097 }, { offset: 0.5 }, { limit: 0 }, { limit: 33 }, { limit: '1' },
    { channel: undefined }, { slot: undefined }, { slot: 16 }, { kind: 'arbitrary' }, { kind: 'instrument' },
    { kind: 'instrument', track: 'Scratch', slot: 1 }, { kind: 'mix_fx', slot: 1 }])
    assert.equal(h.component.pluginModels({ ...modelBase, ...extra }).native_outcome, 'not_applied');
  assert.equal(resolutions, 0);
  const internal = h.component.nativePluginRequest(h.device, { operation: 'model_catalog', catalog: 'effects', offset: 0, limit: 32,
    expected_instance: instance.object_uid, source: 'controller' });
  assert.equal(internal.ok, false); assert.equal(internal.outcome, 'not_applied');
});

test('model list targeting accepts standard track and single-slot targets without forwarding selectors', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((_, body) => { calls.push(JSON.parse(body)); return JSON.stringify(modelResult()); }); } });
  for (const target of [{ kind: 'instrument', track: 'Scratch' }, { kind: 'note_effect', track: 'Scratch', slot: 15 },
    { kind: 'mix_fx', channel: 'Scratch' }, { kind: 'panner', channel: 'Scratch', slot: 0 }]) {
    const { channel, slot, ...base } = modelBase;
    assert.equal(h.component.pluginModels({ ...base, ...target }).ok, true);
  }
  assert.equal(calls.length, 4);
  assert.ok(calls.every(request => !('kind' in request) && !('track' in request) && !('channel' in request) && !('slot' in request)));
});

test('malformed successful model lists lose all catalog rows and certification without fallback', () => {
  const mutations = [
    value => { value.catalog = 'categories'; }, value => { value.model_name = 'categoryModel'; },
    value => { value.count = -1; }, value => { value.count = 4097; }, value => { value.count = 2.5; },
    value => { value.offset = 1; }, value => { value.rows.pop(); }, value => { value.rows[0].index = 1; },
    value => { value.rows[1].index = 0; }, value => { value.rows[0].title = 'x'.repeat(513); },
    value => { value.rows[0].title = '🎸'.repeat(257); }, value => { value.rows[0].title = null; },
    value => { value.rows[0].title_available = 'true'; }, value => { value.rows[0].token = 'issued'; },
    value => { value.next_offset = 2; }, value => { delete value.next_offset; }, value => { value.complete = false; },
    value => { value.complete = 'true'; }, value => { value.all_flat_rows_observed = false; },
    value => { value.scope = 'all_models'; }, value => { value.atomic_snapshot = true; },
    value => { value.mutation_capability = true; }, value => { value.outcome = 'applied_verified'; },
    value => { value.instance.object_uid = '87654321-4321-4321-4321-CBA987654321'; },
    value => { value.instance.class_uid = instance.class_uid; }, value => { value.token = 'future-write'; },
    value => { value.verified = true; }, value => { value.source = 'controller'; },
  ];
  for (const mutate of mutations) {
    let calls = 0;
    const h = harness({ getObjectByName() { return native(() => { calls++; const result = modelResult(); mutate(result); return JSON.stringify(result); }); } });
    h.device.findParameter = () => { throw new Error('no fallback'); };
    const result = h.component.pluginModels(modelBase);
    assert.notEqual(result.ok, true); assert.equal(result.native_outcome, 'unknown'); assert.equal(typeof result.error, 'string');
    for (const key of ['rows', 'count', 'complete', 'all_flat_rows_observed', 'token', 'verified']) assert.equal(result[key], undefined);
    assert.equal(result.mutation_capability, false); assert.equal(calls, 1);
  }
});

test('model lists preserve empty counts, false titles, and inclusive page/text bounds', () => {
  let result = modelResult({ count: 0, rows: [] });
  const h = harness({ getObjectByName() { return native(() => JSON.stringify(result)); } });
  assert.equal(h.component.pluginModels(modelBase).all_flat_rows_observed, true);
  result = modelResult({ rows: [{ index: 0, title: '', title_available: false }, { index: 1, title: 'Delay', title_available: true }], complete: false, all_flat_rows_observed: false });
  assert.equal(h.component.pluginModels(modelBase).rows[0].title_available, false);
  result = modelResult({ count: 4096, offset: 4096, rows: [], all_flat_rows_observed: false });
  assert.equal(h.component.pluginModels({ ...modelBase, offset: 4096, limit: 32 }).ok, true);
  result = modelResult({ count: 32, rows: Array.from({ length: 32 }, (_, index) => ({ index, title: '🎸'.repeat(256), title_available: true })) });
  assert.equal(h.component.pluginModels(modelBase).rows.length, 32);
  result = modelResult({ count: 1, rows: [{ index: 0, title: '', title_available: true }], instance: { ...result.instance, class_uid: 'B9D7536C-A010-457D-A182-36E5531164C9' } });
  assert.equal(h.component.pluginModels(modelBase).rows[0].title, '');
});

test('model lists require a live document lease and native extension without fallback or retry', () => {
  assert.match(harness().component.pluginModels(modelBase).error, /native extension/);
  const context = { open: true, document: 'models:1', session: 'models' };
  let calls = 0;
  const h = harness({ getObjectByName() { return native(() => {
    calls++;
    if (calls === 1) return JSON.stringify({ ...modelResult(), ok: false, error: { message: 'named model absent' }, outcome: 'not_applied' });
    context.document = 'models:2'; return JSON.stringify(modelResult());
  }); } });
  h.device.findParameter = () => { throw new Error('no fallback'); };
  h.component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  const absent = h.component.pluginModels(modelBase);
  assert.match(absent.error, /named model absent/); assert.equal(absent.count, undefined);
  const changed = h.component.pluginModels(modelBase);
  assert.match(changed.error, /document changed/); assert.equal(changed.native_outcome, 'unknown'); assert.equal(changed.rows, undefined);
  assert.equal(calls, 2);
  h.component.bridge.requestContext = { ...context, open: false };
  assert.equal(h.component.pluginModels(modelBase).native_outcome, 'not_applied'); assert.equal(calls, 2);
  const failed = harness({ getObjectByName() { return native(() => JSON.stringify({ ok: false, outcome: 'applied_verified',
    rows: modelResult().rows, error: { message: 'invalid read receipt' } })); } }).component.pluginModels(modelBase);
  assert.equal(failed.native_outcome, 'unknown'); assert.equal(failed.rows, undefined); assert.equal(failed.mutation_capability, false);
});

test('BridgeCore dispatches model lists as reads and refuses stale explicit document intents', () => {
  const host = bridgeHarness(); let calls = 0;
  const h = harness({ getObjectByName() { return native(() => { calls++; return JSON.stringify(modelResult()); }); } });
  h.component.bridge = host.bridge; host.bridge.component = h.component;
  const observed = host.request({ op: 'pluginModels', args: modelBase });
  assert.equal(observed.ok, true); assert.equal(observed.result.catalog, 'effects'); assert.equal(observed.result.mutation_capability, false);
  assert.equal(host.request({ op: 'pluginModels', args: modelBase, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(calls, 1);
  host.manager.activeDocument = null;
  const closed = host.request({ op: 'pluginModels', args: modelBase });
  assert.equal(closed.ok, false); assert.equal(closed.result && closed.result.rows, undefined); assert.equal(calls, 1);
});

test('registered button actions use one finite native request and never certify their effects', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    calls.push(JSON.parse(body));
    return JSON.stringify(actionResult());
  }); } });
  h.device.findParameter = () => { throw new Error('no script action fallback'); };
  const args = { plugin: 'Expected', expected_instance: instance.object_uid, action: 'swap_filter_frequencies' };
  const result = h.component.pluginAction(args);
  assert.deepEqual(calls, [{ operation: 'parameter_action', action: args.action, expected_instance: instance.object_uid }]);
  assert.equal(result.native_outcome, 'applied_unverified');
  assert.equal(result.effects_verified, false);
  assert.equal(result.verified, false);
  const host = bridgeHarness();
  host.bridge.component = { pluginAction() { return result; } };
  assert.equal(host.request({ op: 'pluginAction', args }).code, 'context_required');
  assert.equal(host.writeRequest({ op: 'pluginAction', args }).result.effects_verified, false);
});

test('unknown actions and client-selected parameters refuse before resolving any Device', () => {
  const h = harness();
  h.component.pluginOf = () => { throw new Error('no Device resolution'); };
  const base = { plugin: 'Expected', expected_instance: instance.object_uid, action: 'swap_filter_frequencies' };
  for (const extra of [{ action: 'reset_anything' }, { source: 'dsp' }, { param: 'swapfreqs' },
    { value: 1 }, { method: 'invoke' }, { path: '/unchecked' }, { plugin: '' }, { expected_instance: '' }])
    assert.equal(h.component.pluginAction({ ...base, ...extra }).native_outcome, 'not_applied');
});

test('meter reset actions forward once and never infer effect success from Toggle zero', () => {
  for (const action of ['reset_meter_hold', 'reset_meter_loudness']) {
    const calls = [];
    const h = harness({ getObjectByName() { return native((target, body) => {
      calls.push(JSON.parse(body));
      return JSON.stringify({ ...actionResult(), action, effect_parameters: [],
        instance: { ...instance, class_uid: 'DF958D90-0196-434F-ADC3-1FC2BB7F4AF6' },
        param: action === 'reset_meter_hold' ? 'resethold' : 'resetloudness' });
    }); } });
    h.device.findParameter = () => { throw new Error('no script reset fallback'); };
    const args = { plugin: 'Level Meter', expected_instance: instance.object_uid, action };
    const result = h.component.pluginAction(args);
    assert.equal(result.native_outcome, 'applied_unverified');
    assert.equal(result.effects_verified, false);
    assert.equal(result.verified, false);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].action, action);
    assert.deepEqual(Array.from(result.effect_parameters), []);
    for (const field of ['upper_dispatched', 'reset_dispatched']) {
      const unknown = harness({ getObjectByName() { return native(() => JSON.stringify({
        ok: false, outcome: 'unknown', action, [field]: true, effects_verified: false
      })); } });
      assert.equal(unknown.component.pluginAction(args).native_outcome, 'unknown');
    }
  }
});

test('meter reset receipts accept initial Toggle one without claiming verified effects', () => {
  for (const action of ['reset_meter_hold', 'reset_meter_loudness']) {
    let calls = 0;
    const h = harness({ getObjectByName() { return native(() => {
      calls++;
      return JSON.stringify({ ...actionResult(), action, effect_parameters: [],
        instance: { ...instance, class_uid: 'DF958D90-0196-434F-ADC3-1FC2BB7F4AF6' },
        param: action === 'reset_meter_hold' ? 'resethold' : 'resetloudness',
        before: { value: 1 }, after: { value: 0 } });
    }); } });
    h.device.findParameter = () => { throw new Error('no script reset fallback'); };
    const result = h.component.pluginAction({ plugin: 'Level Meter', expected_instance: instance.object_uid, action });
    assert.equal(result.native_outcome, 'applied_unverified');
    assert.equal(result.before.value, 1);
    assert.equal(result.after.value, 0);
    assert.equal(result.effects_verified, false);
    assert.equal(result.verified, false);
    assert.deepEqual(Array.from(result.effect_parameters), []);
    assert.equal(calls, 1);
  }
});

test('Limiter and Compressor minimum resets use a single fixed native pulse with no effect promotion', () => {
  const action = 'reset_minimum_reduction';
  for (const [plugin, classUid, param] of [
    ['Limiter', '61B18D53-26FA-4220-8614-89944A1990EC', 'resetmin'],
    ['Compressor', '36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B', 'studiocomp.resetmin']
  ]) {
    const calls = [];
    const value = { ...actionResult(), action, param, effect_parameters: [],
      instance: { ...instance, class_uid: classUid, native_class: 'AudioEffect' } };
    const h = harness({ getObjectByName() { return native((target, body) => {
      assert.equal(target, h.device); calls.push(JSON.parse(body)); return JSON.stringify(value);
    }); } });
    h.device.findParameter = () => { throw new Error('no script reset fallback'); };
    const result = h.component.pluginAction({ plugin, expected_instance: instance.object_uid, action });
    assert.deepEqual(calls, [{ operation: 'parameter_action', action, expected_instance: instance.object_uid }]);
    assert.equal(result.native_outcome, 'applied_unverified');
    assert.equal(result.param, param);
    assert.equal(result.verified, false);
    assert.equal(result.effects_verified, false);
    assert.deepEqual(Array.from(result.effect_parameters), []);
  }
});

test('Limiter minimum reset rejects unchecked inputs before resolving a Device', () => {
  let resolutions = 0;
  const h = harness();
  h.component.pluginOf = () => { resolutions++; throw new Error('no resolution'); };
  const base = { plugin: 'Limiter', expected_instance: instance.object_uid, action: 'reset_minimum_reduction' };
  for (const extra of [{ action: 'resetmin' }, { param: 'resetmin' }, { class_uid: 'unchecked' },
    { value: 1 }, { source: 'dsp' }, { method: 'invoke' }, { token: 'unchecked' },
    { plugin: '' }, { expected_instance: '' }])
    assert.equal(h.component.pluginAction({ ...base, ...extra }).native_outcome, 'not_applied');
  assert.equal(resolutions, 0);
});

test('minimum reset malformed receipts remain unknown after one dispatch', () => {
  const action = 'reset_minimum_reduction';
  const valid = { ...actionResult(), action, param: 'resetmin', effect_parameters: [],
    instance: { ...instance, class_uid: '61B18D53-26FA-4220-8614-89944A1990EC', native_class: 'AudioEffect' } };
  for (const extra of [{ action: 'reset_meter_hold' }, { param: 'resethold' }, { source: 'controller' },
    { instance: { ...instance, class_uid: 'DF958D90-0196-434F-ADC3-1FC2BB7F4AF6' } },
    { instance: { ...valid.instance, native_class: 'Controller' } },
    { verified: true }, { effects_verified: true }, { upper_dispatched: false },
    { reset_dispatched: false }, { reset_verified: false }, { effect_parameters: ['minreduction'] }]) {
    let calls = 0;
    const h = harness({ getObjectByName() { return native(() => { calls++; return JSON.stringify({ ...valid, ...extra }); }); } });
    const result = h.component.pluginAction({ plugin: 'Limiter', expected_instance: instance.object_uid, action });
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.verified, false);
    assert.equal(result.effects_verified, false);
    assert.equal(calls, 1);
  }
  for (const oldAction of ['reset_meter_hold', 'reset_meter_loudness']) {
    const h = harness({ getObjectByName() { return native(() => JSON.stringify({ ...valid, action: oldAction })); } });
    assert.equal(h.component.pluginAction({ plugin: 'Limiter', expected_instance: instance.object_uid, action: oldAction }).native_outcome, 'unknown');
  }
});

test('Compressor minimum reset malformed receipts remain unknown after one dispatch', () => {
  const action = 'reset_minimum_reduction';
  const valid = { ...actionResult(), action, param: 'studiocomp.resetmin', effect_parameters: [],
    instance: { ...instance, class_uid: '36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B', native_class: 'AudioEffect' } };
  for (const extra of [{ action: 'reset_meter_hold' }, { param: 'resethold' }, { source: 'controller' },
    { instance: { ...instance, class_uid: 'DF958D90-0196-434F-ADC3-1FC2BB7F4AF6' } },
    { instance: { ...valid.instance, native_class: 'Controller' } },
    { verified: true }, { effects_verified: true }, { upper_dispatched: false },
    { reset_dispatched: false }, { reset_verified: false }, { effect_parameters: ['minreduction'] }]) {
    let calls = 0;
    const h = harness({ getObjectByName() { return native(() => { calls++; return JSON.stringify({ ...valid, ...extra }); }); } });
    const result = h.component.pluginAction({ plugin: 'Compressor', expected_instance: instance.object_uid, action });
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.verified, false);
    assert.equal(result.effects_verified, false);
    assert.equal(calls, 1);
  }
  for (const oldAction of ['reset_meter_hold', 'reset_meter_loudness']) {
    const h = harness({ getObjectByName() { return native(() => JSON.stringify({ ...valid, action: oldAction })); } });
    assert.equal(h.component.pluginAction({ plugin: 'Compressor', expected_instance: instance.object_uid, action: oldAction }).native_outcome, 'unknown');
  }
});

test('uncertain or malformed button receipts preserve unknown outcomes without retries', () => {
  const base = { plugin: 'Expected', expected_instance: instance.object_uid, action: 'swap_filter_frequencies' };
  for (const field of ['effects_verified', 'verified', 'reset_verified', 'upper_dispatched', 'source', 'action']) {
    let calls = 0;
    const h = harness({ getObjectByName() { return native(() => {
      calls++;
      const value = actionResult();
      value[field] = field === 'effects_verified' || field === 'verified' ? true : null;
      return JSON.stringify(value);
    }); } });
    h.device.findParameter = () => { throw new Error('no retry'); };
    const result = h.component.pluginAction(base);
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.effects_verified, false);
    assert.equal(result.reset_verified, false);
    assert.equal(calls, 1);
  }
  const h = harness({ getObjectByName() { return native(() => JSON.stringify({ ok: false, outcome: 'unknown',
    upper_dispatched: true, reset_dispatched: false, effects_verified: false, error: { message: 'context changed' } })); } });
  const result = h.component.pluginAction(base);
  assert.equal(result.native_outcome, 'unknown');
  assert.equal(result.upper_dispatched, true);
  assert.equal(result.reset_dispatched, false);
});

test('parameter details preserve finite native facts without inferring model choices', () => {
  let calls = 0;
  const h = harness({ getObjectByName() { return native((target, body) => {
    calls++;
    assert.equal(target, h.device);
    assert.deepEqual(JSON.parse(body), { operation: 'parameter_details', source: 'dsp', param: 'model', expected_instance: instance.object_uid });
    return success({ details: { precision: 10000, list_interface: false }, original_chain: { parameters: [], end: { kind: 'self_reference' } } });
  }); } });
  const args = { plugin: 'Expected', expected_instance: instance.object_uid, source: 'dsp', param: 'model' };
  const result = h.component.pluginParameterDetails(args);
  assert.equal(result.details.precision, 10000);
  assert.equal(result.choices, undefined);
  assert.match(h.component.pluginParameterDetails({ ...args, source: 'auto' }).error, /explicit source/);
  assert.equal(calls, 1);
});

test('parameter controller observations remain optional bounded read-only metadata', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    const request = JSON.parse(body);
    calls.push(request);
    assert.equal(target, h.device);
    return success({ owner_scope: 'parameter_controller_observation', mutation_capability: false,
      controller_report: { scope: 'parameter_controller_observation', mutation_capability: false } });
  }); } });
  const base = { plugin: 'Expected', expected_instance: instance.object_uid, source: 'dsp', param: 'model' };
  for (const include_owner of [true, false]) {
    const result = h.component.pluginParameterDetails({ ...base, include_owner });
    assert.equal(calls.at(-1).include_owner, include_owner);
    assert.equal(result.mutation_capability, false);
  }
  for (const include_owner of ['true', 1, null, {}])
    assert.equal(typeof h.component.pluginParameterDetails({ ...base, include_owner }).error, 'string');
  assert.equal(calls.length, 2);
  assert.equal(typeof h.component.pluginParameterDetails({ ...base, param: 'bad\0name' }).error, 'string');
  assert.equal(calls.length, 2);
});

test('parameter formatting preserves explicit request order and native labels without selecting values', () => {
  const calls = [];
  const values = [0, 1.5, 0];
  const formatted = [{ requested_value: 0, text: 'First' }, { requested_value: 1.5, text: 'Middle' }, { requested_value: 0, text: 'First' }];
  const h = harness({ getObjectByName() { return native((...args) => {
    calls.push(args);
    assert.equal(args.length, 2);
    assert.equal(args[0], h.device);
    assert.deepEqual(JSON.parse(args[1]), { operation: 'format_values', param: 'model', source: 'dsp', values, expected_instance: instance.object_uid });
    return success({ values: formatted, current_unchanged: true });
  }); } });
  h.device.findParameter = () => { throw new Error('no script fallback'); };
  const result = h.component.pluginParameterFormat({ plugin: 'Expected', param: 'model', source: 'dsp', values, expected_instance: instance.object_uid });
  assert.deepEqual(JSON.parse(JSON.stringify(result.values)), formatted);
  assert.equal(result.current_unchanged, true);
  assert.equal(result.choices, undefined);
  assert.equal(result.count, undefined);
  assert.equal(calls.length, 1);
});

test('invalid formatting requests refuse before resolving a native Device', () => {
  let calls = 0;
  const h = harness({ getObjectByName() { throw new Error('no native lookup'); } });
  h.component.pluginOf = () => { calls++; throw new Error('no Device resolution'); };
  const base = { plugin: 'Expected', param: 'model', source: 'controller', values: [0], expected_instance: instance.object_uid };
  for (const args of [
    { source: 'auto' }, { source: undefined }, { expected_instance: '' }, { plugin: '' },
    { param: '' }, { param: 'x'.repeat(513) }, { param: 'x\0y' },
    { values: undefined }, { values: [] }, { values: Array(33).fill(0) },
    { values: [NaN] }, { values: [Infinity] }, { values: [-Infinity] },
    { values: [9007199254740992] }, { values: [-9007199254740992] },
    { values: ['0'] }, { values: [null] }, { values: { 0: 0, length: 1 } },
  ]) assert.equal(typeof h.component.pluginParameterFormat({ ...base, ...args }).error, 'string');
  assert.equal(calls, 0);
});

test('native format refusals and replaced Device identities never trigger script fallback', () => {
  let calls = 0;
  const h = harness({ getObjectByName() { return native(() => {
    calls++;
    return calls === 1 ? JSON.stringify({ ok: false, error: { message: 'unsupported numeric carrier' } })
      : JSON.stringify({ ok: true, instance: { ...instance, object_uid: 'other' }, values: [] });
  }); } });
  h.device.findParameter = () => { throw new Error('no script fallback'); };
  const args = { plugin: 'Expected', param: 'model', source: 'controller', values: [0], expected_instance: instance.object_uid };
  assert.match(h.component.pluginParameterFormat(args).error, /unsupported numeric carrier/);
  assert.match(h.component.pluginParameterFormat(args).error, /instance changed/);
  assert.equal(calls, 2);
  assert.match(harness().component.pluginParameterFormat(args).error, /requires|require/);
});

test('formatting is read-only at the bridge and rejects an explicitly stale document', () => {
  const host = bridgeHarness();
  const calls = [];
  host.bridge.component = { pluginParameterFormat(args) { calls.push(args); return { values: [], current_unchanged: true }; } };
  const args = { plugin: 'Expected', param: 'model', source: 'dsp', values: [0], expected_instance: instance.object_uid };
  assert.equal(host.request({ op: 'pluginParameterFormat', args }).ok, true);
  assert.equal(calls.length, 1);
  assert.equal(host.request({ op: 'pluginParameterFormat', args, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(calls.length, 1);
});

const nativeDigest = {
  hash_format_version: 'fsp-parameter-digest-v1', hash_scope: 'exposed_parameter_projection',
  schema_sha256: 'a'.repeat(64), all_values_sha256: 'b'.repeat(64), non_read_only_values_sha256: 'c'.repeat(64),
  count: 3, readable_count: 3, read_only_count: 1, readonly_observation_count: 1, non_read_only_count: 2,
  excluded_opaque_count: 1, access_complete: true, opaque_contents_verified: false, atomic_snapshot: false,
  fields_covered: { schema: ['index'], values: ['raw'] }, coverage: 'fresh_sequential_indexed_native_parameter_observations',
  additional_observed_fact: { present: true },
};

test('parameter digests forward only the finite target request and preserve native scope facts', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((...args) => {
    assert.equal(args.length, 2);
    assert.equal(args[0], h.device);
    calls.push(JSON.parse(args[1]));
    return success(nativeDigest);
  }); } });
  h.device.findParameter = () => { throw new Error('no script fallback'); };
  for (const source of ['dsp', 'controller']) {
    const result = h.component.pluginParameterDigest({ plugin: 'Expected', source, expected_instance: instance.object_uid });
    for (const [key, value] of Object.entries(nativeDigest)) assert.deepEqual(JSON.parse(JSON.stringify(result[key])), value);
    assert.equal(result.plugin, 'Expected');
  }
  assert.deepEqual(calls, ['dsp', 'controller'].map(source => ({ operation: 'parameter_digest', source, expected_instance: instance.object_uid })));
});

test('invalid digest targets or providers refuse before resolving a Device', () => {
  let resolutions = 0;
  const h = harness({ getObjectByName() { throw new Error('no native lookup'); } });
  h.component.pluginOf = () => { resolutions++; throw new Error('no Device resolution'); };
  const base = { plugin: 'Expected', source: 'dsp', expected_instance: instance.object_uid };
  for (const args of [
    { plugin: '' }, { plugin: ' ' }, { expected_instance: undefined }, { expected_instance: '' },
    { source: undefined }, { source: 'auto' }, { source: 'arbitrary' },
  ]) assert.equal(typeof h.component.pluginParameterDigest({ ...base, ...args }).error, 'string');
  assert.equal(resolutions, 0);
});

test('digest failures and changed document or Device discard all hashes without fallback', () => {
  const context = { open: true, document: 'a:1', session: 'a' };
  let calls = 0;
  const h = harness({ getObjectByName() { return native(() => {
    calls++;
    if (calls === 1) return JSON.stringify({ ok: false, error: { message: 'incomplete native scan' }, ...nativeDigest });
    if (calls === 2) return JSON.stringify({ ok: true, instance: { ...instance, object_uid: 'replaced' }, ...nativeDigest });
    context.document = 'a:2';
    return success(nativeDigest);
  }); } });
  h.device.findParameter = () => { throw new Error('no script fallback'); };
  h.component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  const args = { plugin: 'Expected', source: 'dsp', expected_instance: instance.object_uid };
  for (const expected of [/incomplete native scan/, /instance changed/, /document changed/]) {
    const result = h.component.pluginParameterDigest(args);
    assert.match(result.error, expected);
    for (const key of Object.keys(result)) assert.equal(key.includes('sha256'), false);
  }
  assert.equal(calls, 3);
  assert.match(harness().component.pluginParameterDigest(args).error, /require/);
});

test('digest observations are read-only and an explicitly stale document blocks dispatch', () => {
  const host = bridgeHarness();
  let calls = 0;
  host.bridge.component = { pluginParameterDigest() { calls++; return nativeDigest; } };
  const args = { plugin: 'Expected', source: 'dsp', expected_instance: instance.object_uid };
  const observed = host.request({ op: 'pluginParameterDigest', args });
  assert.equal(observed.ok, true);
  assert.equal(observed.result.all_values_sha256, nativeDigest.all_values_sha256);
  assert.equal(host.request({ op: 'pluginParameterDigest', args, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(calls, 1);
});

test('Multi child DSP forwards exact identities and discards rows on a mismatched child or focus', () => {
  const args = { track: 'Multi fixture', expected_instance: instance.object_uid, child_slot: 'Inst01',
    expected_child_slot: '22345678-1234-1234-1234-123456789ABC',
    expected_child_instance: '32345678-1234-1234-1234-123456789ABC' };
  const calls = [];
  let wrongChild = false, replaceDevice = false, mappingReads = 0;
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    const request = JSON.parse(body);
    calls.push(request);
    return success({
      operation: 'multi_child_parameters', outcome: 'observed', source: 'dsp', mutation_capability: false,
      instance: { ...instance, class_uid: 'B9D410AD-E47D-495E-8AEE-10D2B2987CC7',
        native_class: 'AudioSynth', object_id: 'Recalled title' },
      child_slot: request.child_slot, child_slot_uid: request.expected_child_slot,
      child_instance: { object_uid: wrongChild ? args.expected_child_instance : request.expected_child_instance,
        class_uid: 'B625F134-4485-4A50-A3C8-C9CF0C5495E1', native_class: 'AudioSynth' },
      offset: request.offset, parameters: [{ index: request.offset, name: 'noise.color' }]
    });
  }); } });
  h.component.instrumentOf = (target) => {
    assert.equal(target.track, args.track);
    mappingReads++;
    return { device: replaceDevice && mappingReads % 2 === 0 ? {} : h.device };
  };
  const second = { ...args, child_slot: 'Inst02', expected_child_slot: '42345678-1234-1234-1234-123456789ABC',
    expected_child_instance: '52345678-1234-1234-1234-123456789ABC', offset: 64, limit: 32 };
  assert.equal(h.component.pluginMultiChildParameters(args).child_instance.object_uid, args.expected_child_instance);
  assert.equal(h.component.pluginMultiChildParameters(second).child_instance.object_uid, second.expected_child_instance);
  assert.deepEqual(calls, [
    { operation: 'multi_child_parameters', expected_instance: args.expected_instance, child_slot: 'Inst01',
      expected_child_slot: args.expected_child_slot, expected_child_instance: args.expected_child_instance, offset: 0, limit: 64 },
    { operation: 'multi_child_parameters', expected_instance: args.expected_instance, child_slot: 'Inst02',
      expected_child_slot: second.expected_child_slot, expected_child_instance: second.expected_child_instance, offset: 64, limit: 32 },
  ]);
  wrongChild = true;
  assert.match(h.component.pluginMultiChildParameters(second).error, /invalid.*receipt/);
  wrongChild = false;
  mappingReads = 0;
  replaceDevice = true;
  const changed = h.component.pluginMultiChildParameters(second);
  assert.match(changed.error, /mapping changed/);
  assert.equal(changed.parameters, undefined);
});

test('Multi child DSP rejects stale document and session before bridge dispatch', () => {
  const host = bridgeHarness();
  const context = host.current();
  let calls = 0;
  host.bridge.component = { pluginMultiChildParameters() { calls++; return { observed: true }; } };
  const op = 'pluginMultiChildParameters';
  assert.equal(host.request({ op }).code, 'context_required');
  assert.equal(host.request({ op, expectedSession: 'stale', expectedDocument: context.document }).code, 'session_changed');
  assert.equal(host.request({ op, expectedSession: context.session, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(calls, 0);
  assert.equal(host.request({ op, expectedSession: context.session, expectedDocument: context.document }).ok, true);
  assert.equal(calls, 1);
});

const multiChildFilterEnvArgs = {
  track: 'Multi Fixture', expected_instance: instance.object_uid, child_slot: 'Inst02',
  expected_child_slot: '42345678-1234-1234-1234-123456789ABC',
  expected_child_instance: '52345678-1234-1234-1234-123456789ABC',
  expected_float_bits: '40b549fe36300000', float_bits: '40B38BA513300000',
};

function filterEnvAck(request) {
  return {
    ok: true, outcome: 'applied_unverified', operation: request.operation,
    source: 'dsp', parameter: 'modulationTable.cell1.amount',
    instance: { object_uid: request.expected_instance, class_uid: 'B9D410AD-E47D-495E-8AEE-10D2B2987CC7',
      native_class: 'AudioSynth', object_id: 'Multi Instrument' },
    child_slot: request.child_slot, child_slot_uid: request.expected_child_slot,
    child_instance: { object_uid: request.expected_child_instance, class_uid: 'B625F134-4485-4A50-A3C8-C9CF0C5495E1',
      native_class: 'AudioSynth', object_id: 'Mai Tai' },
    before_float_bits: request.expected_float_bits, after_float_bits: request.float_bits,
  };
}

test('Multi child Filter Env carries one addressed compare and preserves a proven stale-value or target refusal', () => {
  const args = multiChildFilterEnvArgs;
  const expectedBits = args.expected_float_bits.toUpperCase();
  let currentBits = expectedBits, writes = 0;
  const calls = [];
  const h = harness({ getObjectByName() {
    return {
      capabilities() {
        return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1', multi_child_filter_env_set_api: 'mai_tai_filter_env_v1' });
      },
      request(device, body) {
        assert.equal(device, h.device);
        const request = JSON.parse(body);
        calls.push(request);
        if (request.expected_child_instance !== args.expected_child_instance || request.expected_float_bits !== currentBits)
          return JSON.stringify({ ok: false, outcome: 'not_applied',
            error: { code: 'multi_child_filter_env_write', message: 'Expected bound child or value changed' } });
        writes++;
        currentBits = request.float_bits;
        return JSON.stringify(filterEnvAck(request));
      },
    };
  } });
  h.component.bridge.song = () => ({ transport: { playing: false, recording: false } });
  h.component.instrumentOf = ({ track }) => track === args.track ? { device: h.device } : { error: 'wrong track' };

  for (const bad of [{ track: 'Peer' }, { child_slot: 'Inst03' }, { expected_child_slot: args.expected_instance },
    { float_bits: expectedBits }, { expected_float_bits: 4 }, { float_bits: '7FF8000000000000' },
    { float_bits: '40C3880000000001' }, { param: 'filter.cutoff' }]) {
    assert.equal(h.component.pluginMultiChildFilterEnvSet({ ...args, ...bad }).native_outcome, 'not_applied');
  }
  assert.equal(calls.length, 0);
  const result = h.component.pluginMultiChildFilterEnvSet(args);
  assert.equal(result.native_outcome, 'applied_unverified');
  assert.equal(result.after_float_bits, args.float_bits);
  const { track: _track, ...fields } = args;
  assert.deepEqual(calls[0], { ...fields, expected_float_bits: expectedBits, operation: 'multi_child_filter_env_set' });

  const replay = h.component.pluginMultiChildFilterEnvSet(args);
  assert.equal(replay.native_outcome, 'not_applied');
  assert.equal(replay.after_float_bits, undefined);
  const crosswire = h.component.pluginMultiChildFilterEnvSet({ ...args, expected_float_bits: args.float_bits,
    float_bits: expectedBits, expected_child_instance: '62345678-1234-1234-1234-123456789ABC' });
  assert.equal(crosswire.native_outcome, 'not_applied');
  assert.equal(writes, 1);
});

test('Multi child Filter Env separates undispatched focus, recording and capability failures from lost native proof', () => {
  const args = multiChildFilterEnvArgs;
  let mode, phase, dispatches = 0;
  const nextDevice = {};
  const h = harness({ getObjectByName() {
    return {
      capabilities() {
        phase = 'capabilities';
        return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1',
          ...(mode === 'old_api' ? {} : { multi_child_filter_env_set_api: 'mai_tai_filter_env_v1' }) });
      },
      request(_device, body) {
        dispatches++;
        phase = 'dispatched';
        if (mode === 'lost') throw new Error('No reply from the matching native service');
        const ack = filterEnvAck(JSON.parse(body));
        if (mode === 'malformed') ack.child_slot_uid = ack.child_instance.object_uid;
        return JSON.stringify(ack);
      },
    };
  } });
  h.component.bridge.song = () => ({ transport: { playing: false, recording: mode === 'recording' } });
  h.component.instrumentOf = () => ({
    device: (mode === 'pre_focus' && phase === 'capabilities') || (mode === 'post_focus' && phase === 'dispatched')
      ? nextDevice : h.device,
  });
  for (mode of ['pre_focus', 'recording', 'old_api']) {
    phase = null;
    assert.equal(h.component.pluginMultiChildFilterEnvSet(args).native_outcome, 'not_applied');
  }
  assert.equal(dispatches, 0);
  for (mode of ['lost', 'malformed', 'post_focus']) {
    phase = null;
    const failure = h.component.pluginMultiChildFilterEnvSet(args);
    assert.equal(failure.native_outcome, 'unknown');
    assert.equal(failure.after_float_bits, undefined);
    assert.equal(failure.child_instance, undefined);
  }
  assert.equal(dispatches, 3);
});

test('Multi child Filter Env bridge refuses stale framework binds and erases an acknowledgement after a document switch', () => {
  const host = bridgeHarness();
  const context = host.current();
  let calls = 0, playing = false;
  const args = multiChildFilterEnvArgs;
  const op = 'pluginMultiChildFilterEnvSet';
  host.bridge.song = () => ({ transport: { playing, recording: false } });
  host.bridge.component = {
    pluginMultiChildFilterEnvSet() {
      calls++;
      host.manager.activeDocument = { title: 'Other', path: { url: 'file:///scratch/other.song' } };
      return { ok: true, native_outcome: 'applied_unverified', after_float_bits: args.float_bits };
    },
  };
  assert.equal(host.request({ op, args }).code, 'context_required');
  assert.equal(host.request({ op, args, expectedSession: 'old', expectedDocument: context.document }).code, 'session_changed');
  assert.equal(host.request({ op, args, expectedSession: context.session, expectedDocument: 'old' }).code, 'document_changed');
  const request = { op, args, expectedSession: context.session, expectedDocument: context.document };
  playing = true;
  assert.equal(host.request(request).result.native_outcome, 'not_applied');
  assert.equal(calls, 0);
  playing = false;
  const changed = host.request(request);
  assert.equal(changed.result.native_outcome, 'unknown');
  assert.notEqual(changed.document, context.document);
  assert.equal(changed.result.after_float_bits, undefined);
  assert.equal(calls, 1);
});

test('native calls preserve the exact host object and use the fixed registered service', () => {
  const lookups = [], calls = [];
  const h = harness({ getObjectByName(name) { lookups.push(name); return native((target, body) => { calls.push([target, JSON.parse(body)]); return success(); }); } });
  assert.equal(h.component.pluginInspect({}).native.ok, true);
  assert.equal(calls[0][0], h.device);
  assert.deepEqual(calls[0][1], { operation: 'inspect' });
  assert.deepEqual(lookups, ['FenderMcpNative']);
});

test('node inventory is an explicit read-only inspection option', () => {
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    assert.deepEqual(JSON.parse(body), { operation: 'inspect', include_nodes: true });
    return success({ node_report: { complete: false, nodes: [], diagnostics: [{ kind: 'node_limit' }] } });
  }); } });
  const report = h.component.pluginInspect({ include_nodes: true }).native.node_report;
  assert.equal(report.complete, false);
  assert.equal(report.diagnostics[0].kind, 'node_limit');
});

test('declared plugin controller scope remains a bound read-only observation', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    calls.push(JSON.parse(body));
    return success({ node_report: { scope: 'alias_declared_controller_observation', mutation_capability: false,
      complete: false, diagnostics: [{ kind: 'node_limit' }] } });
  }); } });
  const base = { plugin: 'Expected', expected_instance: instance.object_uid, include_nodes: true, node_scope: 'plugin_controller' };
  const result = h.component.pluginInspect(base).native;
  assert.equal(result.node_report.mutation_capability, false);
  assert.equal(result.node_report.complete, false);
  assert.deepEqual(calls, [{ operation: 'inspect', include_nodes: true, node_scope: 'plugin_controller', expected_instance: instance.object_uid }]);
  for (const missing of [{ plugin: '' }, { expected_instance: '' }, { include_nodes: false }])
    assert.equal(typeof h.component.pluginInspect({ ...base, ...missing }).error, 'string');
  assert.equal(calls.length, 1);
  const host = bridgeHarness();
  host.bridge.component = { pluginInspect() { return { native: result }; } };
  assert.equal(host.request({ op: 'pluginInspect', args: base }).ok, true);
  assert.equal(host.request({ op: 'pluginInspect', args: base, expectedDocument: 'stale' }).code, 'document_changed');
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

test('DSP descriptions retain native access flags and canonical provider names', () => {
  const h = harness({ getObjectByName() { return native(() => success({ source: 'dsp', parameters: [
    { index: 0, name: 'amp.gain', tag: 42, native_description: { name: 'gain', value: 1, writable: false, readOnly: true, enabled: true, native_kind: 'Float' } },
    { index: 1, name: 'unknown', tag: 43, access_error: 'native lookup failed' }
  ], reported_count: 2, offset: 0, next_offset: null, page_complete: true, all_indices_read: true })); } });
  h.device.findParameter = () => ({ value: 9, min: 0, max: 10 });
  const result = h.component.pluginParams({ source: 'dsp' });
  assert.equal(result.params[0].name, 'amp.gain');
  assert.equal(result.params[0].value, 1);
  assert.equal(result.params[0].writable, false);
  assert.equal(result.params[0].readOnly, true);
  assert.equal(result.params[1].access_metadata_complete, false);
  assert.equal(result.params[1].native.access_error, 'native lookup failed');
});

test('explicit DSP reads and writes use native access checks without a script retry', () => {
  let scriptCalls = 0;
  const h = harness({ getObjectByName() { return native((_, body) => {
    const request = JSON.parse(body);
    assert.equal(request.source, 'dsp');
    if (request.operation === 'parameter') return success({ source: 'dsp', parameter: { name: 'gain', value: 1, readOnly: true, writable: false } });
    assert.equal(request.operation, 'set_parameter');
    return JSON.stringify({ ok: false, outcome: 'not_applied', error: { code: 'read_only', message: 'read-only parameter' } });
  }); } });
  h.device.findParameter = () => { scriptCalls++; return { value: 1, setValue() { scriptCalls++; } }; };
  assert.equal(h.component.pluginParams({ names: ['gain'], source: 'dsp' }).params[0].readOnly, true);
  const rejected = h.component.setPluginParam({ plugin: 'Expected', param: 'gain', value: 0.5, source: 'dsp' });
  assert.equal(rejected.native_outcome, 'not_applied');
  assert.equal(scriptCalls, 0);
  assert.match(harness().component.setPluginParam({ plugin: 'Expected', param: 'gain', value: 0.5, source: 'dsp' }).error, /requires/);
});

test('exact Float writes preserve hexadecimal bits through the bridge', () => {
  let calls = 0;
  const h = harness({ getObjectByName() { return native((_, body) => {
    const request = JSON.parse(body);
    assert.equal(request.operation, 'set_parameter');
    assert.equal(request.source, 'dsp');
    assert.equal(request.mode, 'float_bits');
    assert.equal(request.value, '8000000000000000');
    calls++;
    return success({ source: 'dsp', before: { value: 1 }, after: { value: 0, native_value: { data: { bits_hex: request.value } } }, verified: true });
  }); } });
  h.device.findParameter = () => { throw new Error('must use exact native path'); };
  const result = h.component.setPluginParam({ plugin: 'Expected', param: 'gain', source: 'dsp', float_bits: '8000000000000000' });
  assert.equal(result.after.native_value.data.bits_hex, '8000000000000000');
  assert.equal(result.native_verified, true);
  assert.equal(calls, 1);
  assert.match(h.component.setPluginParam({ plugin: 'Expected', param: 'gain', float_bits: '8000000000000000' }).error, /explicit native source/);
  assert.equal(calls, 1);
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
  const result = h.component.setPluginParam({ plugin: 'Expected', param: 'gain', value: 0, expected_instance: 'different' });
  assert.equal(result.native_outcome, 'not_applied');
  assert.equal(result.guard, 'native_instance_preflight');
  assert.match(result.errors[0].message, /instance/);
  assert.equal(writes, 0);
});

test('a failed native instance preflight reports refusal before any explicit DSP setter', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    const request = JSON.parse(body);
    calls.push(request.operation);
    assert.equal(request.operation, 'inspect');
    return JSON.stringify({ ok: false, error: { message: 'instance changed' } });
  }); } });
  h.device.findParameter = () => { throw new Error('no script setter'); };
  const result = h.component.setPluginParam({ plugin: 'Expected', source: 'dsp', param: 'gain', value: 0.5,
    expected_instance: instance.object_uid });
  assert.equal(result.native_outcome, 'not_applied');
  assert.equal(result.guard, 'native_instance_preflight');
  assert.deepEqual(calls, ['inspect']);
  const host = bridgeHarness();
  host.bridge.component = { setPluginParam() { return result; } };
  const receipt = host.writeRequest({ op: 'setPluginParam', args: {} });
  assert.equal(receipt.ok, true);
  assert.equal(receipt.result.native_outcome, 'not_applied');
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

test('state actions pass only the resolved Device and finite native operations', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device);
    const request = JSON.parse(body);
    calls.push(request);
    return success({ token: 'issued', load_accepted: true, verified: false });
  }); } });
  const base = { plugin: 'Expected', expected_instance: instance.object_uid };
  const captured = h.component.pluginState({ ...base, action: 'capture' });
  assert.equal(captured.token, 'issued');
  const restored = h.component.pluginState({ ...base, action: 'restore', token: 'issued' });
  assert.equal(restored.load_accepted, true);
  assert.equal(restored.verified, false);
  h.component.pluginState({ ...base, action: 'release', token: 'issued' });
  assert.deepEqual(calls.map(x => x.operation), ['state_capture', 'state_restore', 'state_release']);
  assert.equal(calls[0].token, undefined);
  assert.equal(calls[1].token, 'issued');
  assert.match(h.component.pluginState({ ...base, action: 'invoke' }).error, /unsupported/);
  assert.match(h.component.pluginState({ ...base, action: 'capture', token: 'imported' }).error, /takes no/);
  assert.match(h.component.pluginState({ ...base, action: 'restore' }).error, /require/);
  assert.equal(calls.length, 3);
});

test('state load context loss remains unknown and never retries a restore', () => {
  const context = { open: true, document: 'a:1', session: 'a' };
  let calls = 0;
  const h = harness({ getObjectByName() { return native(() => {
    calls++; context.document = 'a:2';
    return success({ load_accepted: true, verified: false });
  }); } });
  h.component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  const result = h.component.pluginState({ plugin: 'Expected', expected_instance: instance.object_uid, action: 'restore', token: 'issued' });
  assert.equal(result.native_outcome, 'unknown');
  assert.equal(calls, 1);
});

const catalogToken = 'ABCDEF0123456789'.repeat(4);
const presetBase = { plugin: 'Expected', expected_instance: instance.object_uid };

test('preset catalogs use a fixed empty URL once and apply only an issued catalog index', () => {
  const calls = [], urls = [];
  const prototype = { empty: true };
  const h = harness({ getObjectByName() { return native((...args) => {
    assert.equal(args[0], h.device);
    const request = JSON.parse(args[1]);
    calls.push({ request, count: args.length, auxiliary: args[2] });
    if (request.operation === 'presets') return success({ catalog_token: catalogToken, entries: [{ index: 5, name: 'Preset', recallable: true }], next: null });
    if (request.operation === 'preset_apply') return success({ catalog_token: catalogToken, index: 5, accepted: true, verified: false, outcome: 'applied_unverified' });
    return success({ catalog_token: catalogToken, released: true, verified: true, outcome: 'applied_verified' });
  }); } }, { Url(value) { urls.push(value); return prototype; } });
  const page = h.component.pluginPresets({ ...presetBase, action: 'list', offset: 5, limit: 1 });
  assert.equal(page.catalog_token, catalogToken);
  assert.equal(page.entries[0].index, 5);
  h.component.pluginPresets({ ...presetBase, action: 'list', catalog_token: catalogToken, offset: 6 });
  const applied = h.component.pluginPresets({ ...presetBase, action: 'apply', catalog_token: catalogToken, index: 5 });
  assert.equal(applied.verified, false);
  assert.equal(applied.native_outcome, 'applied_unverified');
  const released = h.component.pluginPresets({ ...presetBase, action: 'release', catalog_token: catalogToken });
  assert.equal(released.released, true);
  assert.deepEqual(urls, ['']);
  assert.deepEqual(calls.map(call => call.count), [3, 2, 2, 2]);
  assert.equal(calls[0].auxiliary, prototype);
  assert.deepEqual(calls[0].request, { operation: 'presets', expected_instance: instance.object_uid, offset: 5, limit: 1 });
  assert.deepEqual(calls[1].request, { operation: 'presets', expected_instance: instance.object_uid, catalog_token: catalogToken, offset: 6, limit: 32 });
  assert.deepEqual(calls[2].request, { operation: 'preset_apply', expected_instance: instance.object_uid, catalog_token: catalogToken, index: 5 });
  assert.deepEqual(calls[3].request, { operation: 'preset_release', expected_instance: instance.object_uid, catalog_token: catalogToken });
});

test('invalid preset actions and bounds cannot resolve or dispatch a Device', () => {
  let resolutions = 0, calls = 0;
  const h = harness({ getObjectByName() { return native(() => { calls++; return success(); }); } });
  h.component.pluginOf = () => { resolutions++; return { name: 'Expected', device: h.device }; };
  for (const args of [
    { action: 'invoke' }, { action: 'list', index: 0 }, { action: 'list', offset: -1 },
    { action: 'list', offset: 4097 }, { action: 'list', limit: 0 }, { action: 'list', limit: 65 },
    { action: 'list', limit: 0.5 }, { action: 'list', catalog_token: 'imported' },
    { action: 'list', catalog_token: catalogToken.toLowerCase() },
    { action: 'apply', index: 0 }, { action: 'apply', catalog_token: catalogToken },
    { action: 'apply', catalog_token: catalogToken, index: 4096 },
    { action: 'apply', catalog_token: catalogToken, index: 0, offset: 0 },
    { action: 'release' }, { action: 'release', catalog_token: catalogToken, index: 0 },
    { action: 'release', catalog_token: catalogToken, limit: 1 },
  ]) assert.equal(typeof h.component.pluginPresets({ ...presetBase, ...args }).error, 'string');
  assert.equal(resolutions, 0);
  assert.equal(calls, 0);
});

test('missing or invalid empty URL factories cannot dispatch the initial catalog', () => {
  let calls = 0;
  const objects = { getObjectByName() { return native(() => { calls++; return success(); }); } };
  for (const host of [{}, { Url() { return null; } }, { Url() { throw new Error('unsupported'); } }]) {
    const result = harness(objects, host).component.pluginPresets({ ...presetBase, action: 'list' });
    assert.equal(typeof result.error, 'string');
  }
  assert.equal(calls, 0);
});

test('native auxiliary objects are refused outside initial catalogs and fixed inspection', () => {
  const calls = [];
  const auxiliary = {};
  const h = harness({ getObjectByName() { return native((...args) => { calls.push(args); return success(); }); } });
  for (const request of [
    { operation: 'inspect' }, { operation: 'state_restore' },
    { operation: 'presets', catalog_token: catalogToken }, { operation: 'preset_apply' },
  ]) assert.equal(h.component.nativePluginRequest(h.device, request, auxiliary).outcome, 'not_applied');
  for (const operation of ['presets', 'inspect_aux']) {
    assert.equal(h.component.nativePluginRequest(h.device, { operation }).outcome, 'not_applied');
    assert.equal(h.component.nativePluginRequest(h.device, { operation }, auxiliary).ok, true);
  }
  assert.equal(calls.length, 2);
  assert.equal(calls[0].length, 3);
  assert.equal(calls[1][2], auxiliary);
});

test('catalog construction preserves the original document lease and uncertain recalls never retry', () => {
  const context = { open: true, document: 'a:1', session: 'a' };
  let calls = 0;
  const h = harness({ getObjectByName() { return native(() => {
    calls++; context.document = 'a:2';
    return success({ accepted: true, verified: false, outcome: 'applied_unverified' });
  }); } }, { Url() { context.document = 'a:2'; return {}; } });
  h.component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  assert.equal(h.component.pluginPresets({ ...presetBase, action: 'list' }).native_outcome, 'not_applied');
  assert.equal(calls, 0);
  context.document = 'a:1';
  const result = h.component.pluginPresets({ ...presetBase, action: 'apply', catalog_token: catalogToken, index: 0 });
  assert.equal(result.native_outcome, 'unknown');
  assert.equal(calls, 1);
});

test('a declined preset load preserves native not_applied evidence', () => {
  const h = harness({ getObjectByName() { return native(() => success({ accepted: false, verified: false, outcome: 'not_applied' })); } });
  const result = h.component.pluginPresets({ ...presetBase, action: 'apply', catalog_token: catalogToken, index: 0 });
  assert.equal(result.accepted, false);
  assert.equal(result.native_outcome, 'not_applied');
});

test('preset listing requires the current bridge document before resolving the plugin', () => {
  const host = bridgeHarness();
  const calls = [];
  host.bridge.component = { pluginPresets(args) { calls.push(args); return { catalog_token: catalogToken }; } };
  const args = { ...presetBase, action: 'list' };
  assert.equal(host.request({ op: 'pluginPresets', args }).code, 'context_required');
  assert.equal(calls.length, 0);
  const result = host.writeRequest({ op: 'pluginPresets', args });
  assert.equal(result.ok, true);
  assert.equal(result.result.catalog_token, catalogToken);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].action, 'list');
});

test('Level settings use the exact native source for pages, named reads, choices and formatting without script fallback', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((_, body) => {
    const request = JSON.parse(body); calls.push(request);
    assert.equal(request.source, 'level_meter');
    if (request.operation === 'parameters') return success({ source: 'level_meter', parameters: [{ name: 'mode', value: 0 }], reported_count: 6, offset: 0, next_offset: 1, page_complete: true, access_metadata_complete: true });
    if (request.operation === 'parameter') return success({ source: 'level_meter', parameter: { name: 'mode', value: 0 } });
    if (request.operation === 'choices') return success({ source: 'level_meter', choices: [{ index: 0, text: 'Native label' }] });
    return success({ source: 'level_meter', values: [{ requested_value: 0, text: 'Native label' }], current_unchanged: true });
  }); } });
  h.device.findParameter = () => { throw new Error('no script fallback'); };
  h.component.nativeInstanceError = () => null;
  const args = { source: 'level_meter', plugin: 'Level Meter', expected_instance: instance.object_uid };
  assert.equal(h.component.pluginParams(args).params[0].name, 'mode');
  assert.equal(h.component.pluginParams({ ...args, names: ['mode'] }).params[0].value, 0);
  assert.equal(h.component.pluginChoices({ ...args, param: 'mode' }).choices[0].text, 'Native label');
  assert.equal(h.component.pluginParameterFormat({ ...args, param: 'mode', values: [0] }).current_unchanged, true);
  assert.deepEqual(calls.map(r => r.operation), ['parameters', 'parameter', 'choices', 'format_values']);
});

test('Level unknown writes stay unknown and dispatch once', () => {
  let writes = 0;
  const h = harness({ getObjectByName() { return native(() => {
    writes++;
    return JSON.stringify({ ok: false, outcome: 'unknown', before: { value: 0 }, after: { value: 1 }, error: { code: 'context_changed', message: 'changed after write' } });
  }); } });
  h.device.findParameter = () => { throw new Error('no script fallback'); };
  h.component.nativeInstanceError = () => null;
  const result = h.component.setPluginParam({ plugin: 'Level Meter', param: 'mode', source: 'level_meter', value: 1, expected_instance: instance.object_uid });
  assert.equal(result.native_outcome, 'unknown');
  assert.equal(result.after.value, 1);
  assert.equal(writes, 1);
});

test('minimum reset rejects cross-class names and unknown exact class identities', () => {
  const action = 'reset_minimum_reduction';
  for (const [classUid, param] of [
    ['61B18D53-26FA-4220-8614-89944A1990EC', 'studiocomp.resetmin'],
    ['36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B', 'resetmin'],
    ['36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B', 'studiocomp.resethold'],
    ['36f3f4d1-cbb4-4bf7-a7e3-ebcced53718b', 'studiocomp.resetmin'],
    ['00000000-0000-0000-0000-000000000000', 'studiocomp.resetmin']
  ]) {
    let calls = 0;
    const value = { ...actionResult(), action, param, effect_parameters: [],
      instance: { ...instance, class_uid: classUid, native_class: 'AudioEffect' } };
    const h = harness({ getObjectByName() { return native(() => { calls++; return JSON.stringify(value); }); } });
    const result = h.component.pluginAction({ plugin: 'Expected', expected_instance: instance.object_uid, action });
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.verified, false);
    assert.equal(result.effects_verified, false);
    assert.equal(calls, 1);
  }
});


test('Scope Retrigger forwards one closed pulse and never promotes acquisition verification', () => {
  const action = 'retrigger_scope';
  const calls = [];
  const value = { ...actionResult(), action, param: 'trigRetrig', effect_parameters: [],
    instance: { ...instance, class_uid: '36322412-AD90-4EE0-B63C-37929566B081', native_class: 'AudioEffect' } };
  const h = harness({ getObjectByName() { return native((target, body) => {
    assert.equal(target, h.device); calls.push(JSON.parse(body)); return JSON.stringify(value);
  }); } });
  h.device.findParameter = () => { throw new Error('no script trigger fallback'); };
  const result = h.component.pluginAction({ plugin: 'Scope', expected_instance: instance.object_uid, action });
  assert.deepEqual(calls, [{ operation: 'parameter_action', action, expected_instance: instance.object_uid }]);
  assert.equal(result.native_outcome, 'applied_unverified');
  assert.equal(result.param, 'trigRetrig');
  assert.equal(result.verified, false);
  assert.equal(result.effects_verified, false);
  assert.deepEqual(Array.from(result.effect_parameters), []);
});

test('Scope Retrigger rejects unchecked selectors before device resolution', () => {
  let resolutions = 0;
  const h = harness();
  h.component.pluginOf = () => { resolutions++; throw new Error('no resolution'); };
  const base = { plugin: 'Scope', expected_instance: instance.object_uid, action: 'retrigger_scope' };
  for (const extra of [{ action: 'trigOneshot' }, { action: 'trigRetrig' }, { param: 'trigRetrig' },
    { class_uid: '36322412-AD90-4EE0-B63C-37929566B081' }, { value: 1 }, { source: 'dsp' },
    { method: 'invoke' }, { plugin: '' }, { expected_instance: '' }])
    assert.equal(h.component.pluginAction({ ...base, ...extra }).native_outcome, 'not_applied');
  assert.equal(resolutions, 0);
});

test('Scope Retrigger malformed or ambiguous receipts never trigger a retry', () => {
  const action = 'retrigger_scope';
  const valid = { ...actionResult(), action, param: 'trigRetrig', effect_parameters: [],
    instance: { ...instance, class_uid: '36322412-AD90-4EE0-B63C-37929566B081', native_class: 'AudioEffect' } };
  for (const extra of [{ action: 'reset_meter_hold' }, { param: 'trigOneshot' }, { source: 'controller' },
    { instance: { ...instance, class_uid: 'DF958D90-0196-434F-ADC3-1FC2BB7F4AF6' } },
    { instance: { ...valid.instance, native_class: 'Controller' } },
    { instance: { ...valid.instance, object_uid: '11111111-1111-4111-8111-111111111111' } },
    { verified: true }, { effects_verified: true }, { upper_dispatched: false },
    { reset_dispatched: false }, { reset_verified: false }, { effect_parameters: ['waveform'] },
    { outcome: 'unknown' }]) {
    let calls = 0;
    const h = harness({ getObjectByName() { return native(() => { calls++; return JSON.stringify({ ...valid, ...extra }); }); } });
    const result = h.component.pluginAction({ plugin: 'Scope', expected_instance: instance.object_uid, action });
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.verified, false);
    assert.equal(result.effects_verified, false);
    assert.equal(calls, 1);
  }
});


test('cv_ports forwards only its exact finite target and requires document and session envelopes', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    calls.push(JSON.parse(body));
    return success({ node_report: { scope: 'cv_ports', availability: 'lookup_absent', routing_verified: false } });
  }); } });
  const base = { kind: 'instrument', track: 'Scratch CV', plugin: 'CV Instrument', source: 'dsp',
    include_nodes: true, node_scope: 'cv_ports', expected_instance: instance.object_uid };
  assert.equal(h.component.pluginInspect(base).native.node_report.availability, 'lookup_absent');
  assert.deepEqual(calls, [{ operation: 'inspect', source: 'dsp', include_nodes: true,
    node_scope: 'cv_ports', expected_instance: instance.object_uid }]);
  for (const patch of [{ kind: 'note_effect' }, { plugin: 'Other' }, { source: undefined }, { source: 'controller' },
    { expected_instance: undefined }, { expected_instance: '00000000-0000-0000-0000-000000000000' },
    { include_nodes: false }, { include_context: true }, { track: '' }]) {
    assert.equal(typeof h.component.pluginInspect({ ...base, ...patch }).error, 'string');
  }
  assert.equal(calls.length, 1);
  const host = bridgeHarness();
  let dispatches = 0;
  host.bridge.component = { pluginInspect() { dispatches++; return { observed: true }; } };
  const context = host.current();
  for (const envelope of [{}, { expectedSession: context.session }, { expectedDocument: context.document }]) {
    assert.equal(host.request({ op: 'pluginInspect', args: base, ...envelope }).code, 'context_required');
  }
  assert.equal(dispatches, 0);
  assert.equal(host.request({ op: 'pluginInspect', args: base,
    expectedSession: context.session, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(host.request({ op: 'pluginInspect', args: base,
    expectedSession: context.session, expectedDocument: context.document }).ok, true);
  assert.equal(dispatches, 1);
});

test('cv_port_parameters forwards only its exact finite target and requires document and session envelopes', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return native((target, body) => {
    calls.push(JSON.parse(body));
    return success({ node_report: { scope: 'cv_port_parameters', availability: 'lookup_absent', routing_verified: false } });
  }); } });
  const base = { kind: 'instrument', track: 'Scratch CV', plugin: 'CV Instrument', source: 'dsp',
    include_nodes: true, node_scope: 'cv_port_parameters', expected_instance: instance.object_uid };
  assert.equal(h.component.pluginInspect(base).native.node_report.availability, 'lookup_absent');
  assert.deepEqual(calls, [{ operation: 'inspect', source: 'dsp', include_nodes: true,
    node_scope: 'cv_port_parameters', expected_instance: instance.object_uid }]);
  for (const patch of [{ kind: 'note_effect' }, { plugin: 'Other' }, { source: undefined }, { source: 'controller' },
    { expected_instance: undefined }, { expected_instance: '00000000-0000-0000-0000-000000000000' },
    { include_nodes: false }, { include_context: true }, { track: '' }, { names: ['unchecked'] }, { offset: 0 }, { limit: 1 }]) {
    assert.equal(typeof h.component.pluginInspect({ ...base, ...patch }).error, 'string');
  }
  assert.equal(calls.length, 1);
  const host = bridgeHarness();
  let dispatches = 0;
  host.bridge.component = { pluginInspect() { dispatches++; return { observed: true }; } };
  const context = host.current();
  for (const envelope of [{}, { expectedSession: context.session }, { expectedDocument: context.document }]) {
    assert.equal(host.request({ op: 'pluginInspect', args: base, ...envelope }).code, 'context_required');
  }
  assert.equal(dispatches, 0);
  assert.equal(host.request({ op: 'pluginInspect', args: base,
    expectedSession: context.session, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(host.request({ op: 'pluginInspect', args: base,
    expectedSession: context.session, expectedDocument: context.document }).ok, true);
  assert.equal(dispatches, 1);
});

test('cv_preset_target returns the bounded native receipt for its exact document and instance', () => {
  const calls = [];
  const report = { scope: 'cv_preset_target', mediator_target_vs_device: 'distinct', target_istorable: 'present' };
  const h = harness({ getObjectByName() { return native((target, body) => {
    calls.push(JSON.parse(body));
    return success({ node_report: report });
  }); } });
  const base = { kind: 'instrument', track: 'Scratch CV', plugin: 'CV Instrument', source: 'dsp',
    include_nodes: true, node_scope: 'cv_preset_target', expected_instance: instance.object_uid };
  const result = h.component.pluginInspect(base);
  assert.deepEqual(Object.keys(result).sort(), ['native', 'plugin']);
  assert.deepEqual({ ...result.native.node_report }, report);
  for (const patch of [{ kind: 'note_effect' }, { plugin: 'Other' }, { source: 'controller' },
    { expected_instance: undefined }, { include_nodes: false }, { include_context: true },
    { names: ['unchecked'] }, { offset: 0 }, { limit: 1 }]) {
    assert.equal(typeof h.component.pluginInspect({ ...base, ...patch }).error, 'string');
  }
  assert.deepEqual(calls, [{ operation: 'inspect', source: 'dsp', include_nodes: true,
    node_scope: 'cv_preset_target', expected_instance: instance.object_uid }]);

  const host = bridgeHarness();
  let dispatches = 0;
  host.bridge.component = { pluginInspect() { dispatches++; return { observed: true }; } };
  const { session, document } = host.current();
  for (const envelope of [{}, { expectedSession: session }, { expectedDocument: document }]) {
    assert.equal(host.request({ op: 'pluginInspect', args: base, ...envelope }).code, 'context_required');
  }
  assert.equal(host.request({ op: 'pluginInspect', args: base,
    expectedSession: 'stale', expectedDocument: document }).code, 'session_changed');
  assert.equal(host.request({ op: 'pluginInspect', args: base,
    expectedSession: session, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(dispatches, 0);
  assert.equal(host.request({ op: 'pluginInspect', args: base,
    expectedSession: session, expectedDocument: document }).ok, true);
  assert.equal(dispatches, 1);
});

test('cv_preset_target discards native evidence when the document switches before publication', () => {
  const h = bridgeHarness();
  const context = h.current();
  const scope = 'cv_preset_target';
  let dispatches = 0;
  h.bridge.component = { pluginInspect() {
    dispatches++;
    h.manager.activeDocument = { title: 'Other', path: { url: 'file:///other.song' } };
    return { native: { ok: true, instance, node_report: { scope, target_istorable: 'present' } } };
  } };
  const reply = h.request({ op: 'pluginInspect', args: { node_scope: scope },
    expectedSession: context.session, expectedDocument: context.document });
  assert.equal(dispatches, 1);
  assert.notEqual(reply.document, context.document);
  assert.equal(reply.ok, false);
  assert.equal(reply.code, 'native_context_lost');
  assert.equal(reply.result, undefined);
  assert.ok(!JSON.stringify(reply).includes('node_report'));
});

const cvPresetSettingsArgs = {
  kind: 'instrument', track: 'Scratch', plugin: 'CV Instrument',
  expected_instance: instance.object_uid, action: 'inspect_cv_preset_settings',
  catalog_token: 'A'.repeat(64), index: 7,
};

test('CV preset settings forwards only the exact issued selector and projects the digest', () => {
  const calls = [];
  const settings = { byte_count: 3991, sha256: 'a'.repeat(64), rejected_extra: 'internal' };
  const h = harness({ getObjectByName() { return native((target, body, auxiliary) => {
    assert.equal(target, h.device);
    assert.equal(auxiliary, undefined);
    calls.push(JSON.parse(body));
    return success({
      instance: { ...instance, class_uid: 'EDCD22A5-A01E-4EA6-9FE4-2BB1AB243E85' },
      outcome: 'observed', index: 7, cv_preset_settings: settings, rejected_extra: 'internal',
    });
  }); } });
  const result = h.component.pluginPresets(cvPresetSettingsArgs);
  assert.equal(result.mutation_capability, false);
  assert.deepEqual({ ...result.cv_preset_settings }, { byte_count: 3991, sha256: 'a'.repeat(64) });
  assert.ok(!JSON.stringify(result).includes('internal'));
  for (const patch of [{ plugin: 'Other' }, { kind: 'note_effect' }, { track: ' ' },
    { offset: 0 }, { limit: 1 }, { slot: 1 }, { path: '/other.preset' }, { bytes: 'raw' },
    { expected_instance: '00000000-0000-0000-0000-000000000000' }, { index: 4096 },
    { catalog_token: undefined }, { index: undefined }]) {
    assert.equal(typeof h.component.pluginPresets({ ...cvPresetSettingsArgs, ...patch }).error, 'string');
  }
  assert.deepEqual(calls, [{ operation: 'inspect_cv_preset_settings', expected_instance: instance.object_uid,
    catalog_token: 'A'.repeat(64), index: 7 }]);
});

test('CV preset settings rejects a late document switch without publishing a digest', () => {
  const h = bridgeHarness();
  const context = h.current();
  let reads = 0;
  h.bridge.component = { pluginPresets() {
    reads++;
    h.manager.activeDocument = { title: 'Other', path: { url: 'file:///other.song' } };
    return { cv_preset_settings: { byte_count: 3991, sha256: 'a'.repeat(64) } };
  } };
  const request = { op: 'pluginPresets', args: cvPresetSettingsArgs, expectedSession: context.session };
  assert.equal(h.request(request).code, 'context_required');
  assert.equal(reads, 0);
  const result = h.request({ ...request, expectedDocument: context.document });
  assert.equal(reads, 1);
  assert.equal(result.code, 'native_context_lost');
  assert.equal(result.ok, false);
  assert.equal(result.result, undefined);
});

const cvPresetApplyArgs = { ...cvPresetSettingsArgs, action: 'apply_cv_preset_settings' };
const cvApplyNative = request => ({
  capabilities() {
    return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1', cv_preset_settings_apply_api: 'cv_primary_settings_v1' });
  },
  request,
});
const cvApplyAccepted = () => success({
  instance: { ...instance, class_uid: 'EDCD22A5-A01E-4EA6-9FE4-2BB1AB243E85' },
  outcome: 'applied_unverified', operation: 'apply_cv_preset_settings', index: 7,
  cv_preset_settings: { byte_count: 3991, sha256: 'a'.repeat(64) }, load_accepted: true, verified: false,
});
const cvApplyRefused = () => JSON.stringify({ ok: false, outcome: 'not_applied',
  error: { code: 'cv_preset_settings_apply_failed', message: 'Current issued preset file changed' } });

test('CV settings apply dispatches only its exact bound selector and projects an unverified load receipt', () => {
  const calls = [];
  const h = harness({ getObjectByName() { return cvApplyNative((target, body, auxiliary) => {
    assert.equal(target, h.device);
    assert.equal(auxiliary, undefined);
    calls.push(JSON.parse(body));
    return cvApplyAccepted();
  }); } });
  for (const patch of [{ plugin: 'Other' }, { kind: 'effect' }, { path: '/other.preset' }, { bytes: 'raw' },
    { index: 4096 }, { catalog_token: undefined }, { expected_instance: undefined }, { offset: 0 }]) {
    const refused = h.component.pluginPresets({ ...cvPresetApplyArgs, ...patch });
    assert.equal(refused.native_outcome, 'not_applied');
  }
  assert.equal(calls.length, 0);
  const result = h.component.pluginPresets(cvPresetApplyArgs);
  assert.deepEqual(JSON.parse(JSON.stringify(result)), {
    action: 'apply_cv_preset_settings', index: 7, plugin: 'CV Instrument',
    cv_preset_settings: { byte_count: 3991, sha256: 'a'.repeat(64) },
    ok: true, native_outcome: 'applied_unverified', load_accepted: true, verified: false,
  });
  assert.deepEqual(calls, [{ operation: 'apply_cv_preset_settings', expected_instance: instance.object_uid,
    catalog_token: 'A'.repeat(64), index: 7 }]);
});

test('CV settings apply preserves pre-load refusal and reports unknown after a disputed native dispatch', () => {
  let calls = 0;
  const old = harness({ getObjectByName() { return native(() => { calls++; return cvApplyAccepted(); }); } });
  assert.equal(old.component.pluginPresets(cvPresetApplyArgs).native_outcome, 'not_applied');
  assert.equal(calls, 0);
  const malformed = harness({ getObjectByName() { return cvApplyNative(() => { calls++; return 'not-json'; }); } });
  const unknown = malformed.component.pluginPresets(cvPresetApplyArgs);
  assert.equal(unknown.native_outcome, 'unknown');
  assert.equal(unknown.cv_preset_settings, undefined);
  assert.equal(calls, 1);

  for (const [nativeReply, outcome] of [[cvApplyAccepted, 'unknown'], [cvApplyRefused, 'not_applied']]) {
    const h = harness({ getObjectByName() { return cvApplyNative(() => {
      calls++;
      h.component.bridge.context = () => ({ open: true, document: 'contract:2', session: 'contract' });
      return nativeReply();
    }); } });
    const result = h.component.pluginPresets(cvPresetApplyArgs);
    assert.equal(result.native_outcome, outcome);
    assert.equal(result.cv_preset_settings, undefined);
    assert.equal(result.load_accepted, undefined);
  }
  assert.equal(calls, 3);
});

test('CV settings apply strips a late bridge reply after the active document changes', () => {
  for (const refused of [false, true]) {
    const h = bridgeHarness();
    const context = h.current();
    let dispatches = 0;
    h.bridge.component = { pluginPresets() {
      dispatches++;
      h.manager.activeDocument = { title: 'Other', path: { url: 'file:///other.song' } };
      return refused ? { ok: false, native_outcome: 'not_applied', errors: [{ message: 'refused' }] }
        : { ok: true, native_outcome: 'applied_unverified', load_accepted: true,
            cv_preset_settings: { byte_count: 3991, sha256: 'a'.repeat(64) } };
    } };
    const request = { op: 'pluginPresets', args: cvPresetApplyArgs, expectedSession: context.session };
    assert.equal(h.request(request).code, 'context_required');
    assert.equal(dispatches, 0);
    const envelope = h.request({ ...request, expectedDocument: context.document });
    assert.equal(dispatches, 1);
    assert.notEqual(envelope.document, context.document);
    assert.equal(envelope.result.ok, false);
    assert.equal(envelope.result.native_outcome, refused ? 'not_applied' : 'unknown');
    assert.equal(envelope.result.cv_preset_settings, undefined);
    assert.equal(envelope.result.load_accepted, undefined);
  }
});

test('CV settings read and apply publish fixed errors without native or resolver file paths', () => {
  const privatePath = '/private/mock/path.preset';
  for (const outcome of ['not_applied', 'unknown']) {
    const h = harness({ getObjectByName() { return cvApplyNative(() => JSON.stringify({
      ok: false, outcome, error: { code: privatePath, message: privatePath },
    })); } });
    const inspection = h.component.pluginPresets(cvPresetSettingsArgs);
    assert.equal(inspection.error, 'CV preset settings inspection is unavailable');
    const application = h.component.pluginPresets(cvPresetApplyArgs);
    assert.equal(application.native_outcome, outcome);
    assert.ok(!JSON.stringify(application).includes(privatePath));
    assert.ok(JSON.stringify(h.component.pluginPresets({ ...cvPresetApplyArgs, action: 'apply' })).includes(privatePath));
    h.component.pluginOf = () => ({ error: privatePath });
    assert.equal(h.component.pluginPresets(cvPresetSettingsArgs).error, 'The bound CV Instrument is unavailable');
    const refused = h.component.pluginPresets(cvPresetApplyArgs);
    assert.equal(refused.native_outcome, 'not_applied');
    assert.ok(!JSON.stringify(refused).includes(privatePath));
  }
});

test('CV settings bridge catches thrown path details with fixed text and preserves uncertainty', () => {
  const privatePath = '/private/mock/path.preset';
  const host = bridgeHarness();
  const context = host.current();
  host.bridge.component = { pluginPresets() { throw new Error(privatePath); } };
  for (const args of [cvPresetSettingsArgs, cvPresetApplyArgs]) {
    const reply = host.request({ op: 'pluginPresets', args,
      expectedSession: context.session, expectedDocument: context.document });
    assert.ok(!JSON.stringify(reply).includes(privatePath));
    if (args === cvPresetApplyArgs) {
      assert.equal(reply.result.native_outcome, 'unknown');
      assert.equal(reply.result.cv_preset_settings, undefined);
    } else {
      assert.equal(reply.error, 'CV preset settings inspection is unavailable');
    }
  }
});
