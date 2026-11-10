import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: 'D2D9B002-12FC-4F1D-AFDA-70336D1DC108' };
const args = { kind: 'note_effect', track: 'Scratch', slot: 0, plugin: 'Chorder',
  source: 'dsp', param: 'chorder.autofill', expected_instance: instance.object_uid, compare_gui: true };
const common = () => ({ scope: 'exact_device_named_control_candidate', parameter_name: 'chorder.autofill',
  semantic_binding: 'named_candidate_only', mutation_capability: false,
  coverage: { complete: true, candidate_count: 1, visited_views: 8, max_views: 128, max_depth: 16 } });
const compared = () => ({ ...common(), status: 'compared', topParameterSame: false, declaredObserverSame: true,
  declaredObserverPresence: 'both_present', originalSame: null, originalBasis: 'not_compared',
  originalUnknownReason: 'not_observed', editorMatch: 'canonical_iunknown' });
const unresolved = reason => ({ ...common(), status: 'unresolved', reason,
  coverage: { complete: false, candidate_count: 0, visited_views: 0, max_views: 128, max_depth: 16 } });
const success = (gui = compared()) => ({ ok: true, outcome: 'observed', instance, source: 'dsp', gui_comparison: gui });

function harness(reply = () => success()) {
  const calls = [];
  const context = { open: true, document: 'gui:1', session: 'gui' };
  const service = {
    capabilities() { return '{"ok":true,"api":"fender-mcp-native-v1"}'; },
    request(device, body) {
      calls.push(JSON.parse(body));
      const result = reply(calls.at(-1), context);
      return typeof result === 'string' ? result : JSON.stringify(result);
    },
  };
  const scope = vm.createContext({ include_file() {}, memberTypes() { return {}; }, PreSonus: { ControlSurfaceComponent: class {} },
    Host: { Objects: { getObjectByName() { return service; } } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  component.pluginOf = () => ({ name: 'Chorder', device: {} });
  return { component, context, calls };
}

test('GUI comparison opt-in forwards one read with optional owner metadata', () => {
  const h = harness();
  for (const include_owner of [undefined, false, true]) {
    const input = { ...args };
    if (include_owner !== undefined) input.include_owner = include_owner;
    const result = h.component.pluginParameterDetails(input);
    assert.equal(result.gui_comparison.topParameterSame, false);
    assert.equal(result.gui_comparison.declaredObserverSame, true);
    assert.equal(result.gui_comparison.originalSame, null);
    const expected = { operation: 'parameter_details', source: 'dsp', param: args.param,
      expected_instance: instance.object_uid, compare_gui: true };
    if (include_owner !== undefined) expected.include_owner = include_owner;
    assert.deepEqual(h.calls.at(-1), expected);
  }
  assert.equal(h.calls.length, 3);
});

test('omitted and false GUI opt-ins preserve the legacy request', () => {
  const h = harness(() => ({ ok: true, instance, details: { precision: 1 } }));
  const { compare_gui, ...legacy } = args;
  assert.equal(h.component.pluginParameterDetails(legacy).details.precision, 1);
  assert.equal(h.component.pluginParameterDetails({ ...legacy, compare_gui: false }).details.precision, 1);
  assert.deepEqual(h.calls[0], h.calls[1]);
  assert.equal('compare_gui' in h.calls[0], false);
});

test('invalid GUI comparison requests refuse before resolving a Device', () => {
  const h = harness();
  h.component.pluginOf = () => { throw new Error('invalid comparison must not resolve'); };
  for (const extra of [{ compare_gui: null }, { compare_gui: 'true' }, { compare_gui: 1 }, { compare_gui: {} },
    { plugin: 'Other' }, { param: 'autofill' }, { source: 'controller' }, { source: 'auto' },
    { expected_instance: '' }, { expected_instance: instance.object_uid.toLowerCase() },
    { expected_instance: '00000000-0000-0000-0000-000000000000' }]) {
    const result = h.component.pluginParameterDetails({ ...args, ...extra });
    assert.equal(typeof result.error, 'string');
  }
  assert.equal(h.calls.length, 0);
});

test('GUI comparison requires resolved Chorder identity and closed native request fields', () => {
  const h = harness();
  h.component.pluginOf = () => ({ name: 'Other', device: {} });
  assert.equal(typeof h.component.pluginParameterDetails(args).error, 'string');
  assert.equal(h.calls.length, 0);
  const base = { operation: 'parameter_details', source: 'dsp', param: args.param,
    expected_instance: instance.object_uid, compare_gui: true };
  for (const extra of [{ method: 'invoke' }, { path: 'anything' }, { source: 'controller' },
    { param: 'autofill' }, { compare_gui: null }, { include_owner: null }]) {
    assert.equal(h.component.nativePluginRequest({}, { ...base, ...extra }).outcome, 'not_applied');
  }
  assert.equal(h.calls.length, 0);
});

test('GUI comparison preserves explicit unresolved diagnostics and absent observer roles', () => {
  const h = harness(() => success(unresolved('missing_editor')));
  assert.equal(h.component.pluginParameterDetails(args).gui_comparison.reason, 'missing_editor');
  for (const presence of ['both_absent', 'one_absent']) {
    const r = harness(() => success({ ...compared(), declaredObserverPresence: presence, declaredObserverSame: null }))
      .component.pluginParameterDetails(args);
    assert.equal(r.gui_comparison.declaredObserverPresence, presence);
    assert.equal(r.gui_comparison.declaredObserverSame, null);
  }
  const failed = harness(() => ({ ok: false, outcome: 'not_applied', gui_comparison: unresolved('stale_association'),
    error: { code: 'guard_changed', message: 'The editor changed' } })).component.pluginParameterDetails(args);
  assert.equal(failed.native_outcome, 'not_applied');
  assert.equal(failed.gui_comparison.reason, 'stale_association');
  assert.equal('topParameterSame' in failed.gui_comparison, false);
});

test('malformed GUI reports and failed receipts cannot publish identity claims or mutation uncertainty', () => {
  const mutations = [
    r => { delete r.gui_comparison; }, r => { r.gui_comparison.topParameterSame = 'false'; },
    r => { r.gui_comparison.declaredObserverSame = null; }, r => { r.gui_comparison.originalSame = true; },
    r => { r.gui_comparison.mutation_capability = true; }, r => { r.gui_comparison.coverage.complete = false; },
    r => { r.gui_comparison.coverage.candidate_count = 2; }, r => { r.gui_comparison.coverage.visited_views = 129; },
    r => { r.gui_comparison.editorMatch = 'caption'; }, r => { r.gui_comparison.pointer = 123; },
    r => { r.instance = { ...instance, class_uid: '87654321-4321-4321-4321-CBA987654321' }; },
    r => { r.instance = { ...instance, object_uid: '87654321-4321-4321-4321-CBA987654321' }; },
    r => { r.source = 'controller'; }, r => { r.outcome = 'unknown'; },
    r => { r.error = { message: 'failed despite ok' }; }, r => { r.native_error = 'failure'; },
    r => { r.native_outcome = 'unknown'; }, r => { r.mutation_capability = true; },
    r => { r.verified = true; }, r => { r.effects_verified = true; }, r => { r.token = 'callable'; },
    r => { r.ok = false; r.outcome = 'not_applied'; r.error = { message: 'failed after comparison' }; },
    r => { r.gui_comparison = { ...unresolved('deadline'), topParameterSame: true }; },
  ];
  for (const mutate of mutations) {
    const h = harness(() => { const r = success(); mutate(r); return r; });
    const result = h.component.pluginParameterDetails(args);
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('topParameterSame' in result.gui_comparison, false);
    assert.equal('declaredObserverSame' in result.gui_comparison, false);
    assert.equal(h.calls.length, 1);
  }
  for (const reply of [() => '{', () => null, () => { throw new Error('read transport lost'); }]) {
    const r = harness(reply).component.pluginParameterDetails(args);
    assert.equal(r.native_outcome, 'not_applied');
    assert.equal(r.gui_comparison.status, 'unresolved');
  }
});

test('GUI read context changes invalidate identities at native and outer publication boundaries', () => {
  const h = harness((request, context) => { context.document = 'gui:2'; return success(); });
  const result = h.component.pluginParameterDetails(args);
  assert.equal(result.native_outcome, 'not_applied');
  assert.equal(result.gui_comparison.status, 'unresolved');
  assert.equal('topParameterSame' in result.gui_comparison, false);
  const host = bridgeHarness();
  host.bridge.component = { pluginParameterDetails() {
    host.manager.activeDocument = { title: 'Different' };
    return success();
  } };
  const response = host.request({ op: 'pluginParameterDetails', args });
  assert.equal(response.result.native_outcome, 'not_applied');
  assert.equal(response.result.gui_comparison.status, 'unresolved');
  assert.equal('topParameterSame' in response.result.gui_comparison, false);
});

test('GUI comparison rejects malformed success flags and native error collections', () => {
  const mutations = [
    ...[['failure'], [{ code: 'failed', message: 'comparison failed' }], null, false, 0, '', 'failure', {}]
      .map(errors => r => { r.errors = errors; }),
    ...['true', 1, {}, []].map(ok => r => { r.ok = ok; }),
  ];
  for (const mutate of mutations) {
    const h = harness(() => { const result = success(); mutate(result); return result; });
    const result = h.component.pluginParameterDetails(args);
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('topParameterSame' in result.gui_comparison, false);
    assert.equal('declaredObserverSame' in result.gui_comparison, false);
    assert.equal(h.calls.length, 1);
  }
  const accepted = harness(() => ({ ...success(), errors: [], error: null, native_error: null,
    native_outcome: 'observed' })).component.pluginParameterDetails(args);
  assert.equal(accepted.gui_comparison.status, 'compared');
  assert.equal(accepted.gui_comparison.declaredObserverSame, true);
});

test('legacy details retain their existing native errors payload', () => {
  const h = harness(() => ({ ok: true, instance, errors: ['legacy payload'], details: { precision: 1 } }));
  const { compare_gui, ...legacy } = args;
  for (const input of [legacy, { ...legacy, compare_gui: false }]) {
    const result = h.component.pluginParameterDetails(input);
    assert.equal(result.details.precision, 1);
    assert.equal(result.errors[0], 'legacy payload');
  }
  assert.deepEqual(h.calls[0], h.calls[1]);
});
