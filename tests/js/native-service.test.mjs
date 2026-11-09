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
