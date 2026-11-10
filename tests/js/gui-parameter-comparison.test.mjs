import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';
import { guiDescription } from './gui-description.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: 'D2D9B002-12FC-4F1D-AFDA-70336D1DC108' };
const args = { kind: 'note_effect', track: 'Scratch', slot: 0, plugin: 'Chorder',
  source: 'dsp', param: 'chorder.autofill', expected_instance: instance.object_uid, compare_gui: true };
const common = () => ({ scope: 'exact_device_named_control_candidate', parameter_name: 'chorder.autofill',
  semantic_binding: 'named_candidate_only', mutation_capability: false,
  coverage: { complete: true, candidate_count: 1, visited_views: 8, max_views: 128, max_depth: 32 } });
const compared = () => ({ ...common(), status: 'compared', topParameterSame: false, declaredObserverSame: true,
  declaredObserverPresence: 'both_present', originalSame: null, originalBasis: 'not_compared',
  originalUnknownReason: 'not_observed', editorMatch: 'canonical_iunknown', gui_parameter: guiDescription() });
const unresolved = reason => ({ ...common(), status: 'unresolved', reason,
  coverage: { complete: false, candidate_count: 0, visited_views: 0, max_views: 128, max_depth: 32 } });
const success = (gui = compared()) => ({ ok: true, outcome: 'observed', instance, source: 'dsp', gui_comparison: gui });

function modelDescription(name, value = 0) {
  const descriptor = guiDescription();
  const pitch = name === 'learnInputPitch';
  const typed = value => ({ variant_tag: 1, data: { kind: 'integer', value } });
  const normalized = pitch ? (value + 1) / 128 : value;
  const bits = new DataView(new ArrayBuffer(4));
  bits.setFloat32(0, normalized);
  return { ...descriptor, name, title: name, tag: pitch ? 12 : 11, value, text: String(value),
    min: pitch ? -1 : 0, max: pitch ? 127 : 1, default: pitch ? -1 : 0, normalized,
    native_kind: pitch ? 'Integer' : 'Toggle', native_value: typed(value), native_min: typed(pitch ? -1 : 0),
    native_max: typed(pitch ? 127 : 1), native_default: typed(pitch ? -1 : 0),
    native_normalized: { kind: 'float32', bits_hex: bits.getUint32(0).toString(16).padStart(8, '0').toUpperCase() } };
}

function observerMetadata() {
  return { status: 'observed', report: { coverage: 'bounded_parameter_controller_metadata',
    scope: 'parameter_controller_observation', mutation_capability: false, complete: true,
    limits: { nodes: 1, depth: 0, ancestors: 0, children_per_node: 1 },
    nodes: [{ index: 0, identity: null, child_count: null,
      declared_interfaces: { preset_collector: false, preset_mediator: false, storable: false },
      reflection: { available: true, types: [{ name: 'FakeModelController', namespace: null, class_uid: null, flags: 0,
        methods: [{ name: 'findParameter', arguments: 'name: string', return_type: 'Object' }],
        properties: [{ name: 'paramCount', data_type: 1, type_name: null, has_type_info: false }] }], issues: [] } }],
    edges: [], diagnostics: [] } };
}

function controllerProbe() {
  return { status: 'observed', scope: 'fixed_gui_declared_observer_controller', mutation_capability: false,
    value_semantics: 'sequential_snapshot', interfaces: { controller: { status: 'present' }, provider: { status: 'unsupported' } },
    anchors: { controller: 'same_parameter', provider: 'controller_absent' }, observer_metadata: observerMetadata(),
    binding: { route: 'direct', autofill_same_gui: true, controller_same_observer: true, controller_same_direct_dsp: false },
    parameters: { learnMode: modelDescription('learnMode'), learnInputPitch: modelDescription('learnInputPitch', -1) } };
}

function withProbe(probe = controllerProbe()) { return { ...compared(), controller_probe: probe }; }

function harness(reply = request => success(request.include_owner === true ? withProbe() : compared())) {
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

test('incomplete GUI walks preserve only closed typed failure diagnostics', () => {
  const diagnostics = [
    { kind: 'interface_query_failed', stage: 'control_query', depth: 3, query_status: -2147467259 },
    { kind: 'repeated_view', stage: 'view_identity', depth: 1, query_status: null },
    { kind: 'depth_limit', stage: 'children', depth: 32, query_status: null },
  ];
  for (const walk_failure of diagnostics) {
    const report = { ...unresolved('incomplete_walk'), walk_failure,
      coverage: { ...common().coverage, complete: false, candidate_count: 0, visited_views: 75 } };
    const result = harness(() => success(report)).component.pluginParameterDetails(args);
    assert.equal(result.outcome, 'observed');
    assert.equal(JSON.stringify(result.gui_comparison.walk_failure), JSON.stringify(walk_failure));
    assert.equal('topParameterSame' in result.gui_comparison, false);
  }
});

test('walk failure depth requires enough observed ancestors', () => {
  for (const [kind, stage, depth, count, query_status] of [
    ['depth_limit', 'children', 32, 0, null],
    ['depth_limit', 'children', 32, 32, null],
    ['interface_query_failed', 'control_query', 0, 0, -2147467259],
    ['interface_query_failed', 'control_query', 32, 32, -2147467259],
  ]) {
    const report = { ...unresolved('incomplete_walk'),
      coverage: { ...common().coverage, complete: false, candidate_count: 0, visited_views: count },
      walk_failure: { kind, stage, depth, query_status } };
    const result = harness(() => success(report)).component.pluginParameterDetails(args);
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.gui_comparison.reason, 'inspection_failed');
    assert.equal('walk_failure' in result.gui_comparison, false);
  }
});

test('walk failure diagnostics reject unknown, misplaced or contradictory data', () => {
  const base = () => ({ ...unresolved('incomplete_walk'),
    coverage: { ...common().coverage, complete: false, candidate_count: 0, visited_views: 75 },
    walk_failure: { kind: 'interface_query_failed', stage: 'control_query', depth: 3, query_status: -2147467259 } });
  const mutations = [
    r => { r.walk_failure = null; }, r => { r.walk_failure = []; },
    r => { r.walk_failure.kind = 'private_error'; }, r => { r.walk_failure.stage = 'path'; },
    r => { r.walk_failure.pointer = 123; }, r => { r.walk_failure.message = 'private native detail'; },
    r => { delete r.walk_failure.query_status; }, r => { r.walk_failure.depth = 33; },
    r => { r.walk_failure.depth = -1; }, r => { r.walk_failure.depth = false; },
    r => { r.walk_failure.query_status = null; }, r => { r.walk_failure.query_status = false; },
    r => { r.walk_failure.query_status = -2147483649; }, r => { r.walk_failure.query_status = 2147483648; },
    r => { r.walk_failure.query_status = 0.5; }, r => { r.walk_failure.stage = 'iterator'; },
    r => { r.reason = 'missing_editor'; }, r => { r.coverage.complete = true; },
    r => { r.coverage.max_depth = 16; },
    r => { r.walk_failure = { kind: 'view_limit', stage: 'children', depth: 0, query_status: null }; },
    r => { r.walk_failure = { kind: 'depth_limit', stage: 'children', depth: 31, query_status: null }; },
    r => { r.walk_failure = { kind: 'depth_limit', stage: 'children', depth: 16, query_status: null }; },
    r => { r.walk_failure = { kind: 'repeated_view', stage: 'edge', depth: 1, query_status: null }; },
    r => { r.walk_failure = { kind: 'detached_view', stage: 'edge', depth: 1, query_status: 0 }; },
    r => { r.walk_failure.kind = 'missing_interface'; },
    r => { Object.assign(r, compared()); },
  ];
  for (const mutate of mutations) {
    const report = base(); mutate(report);
    const h = harness(() => success(report));
    const result = h.component.pluginParameterDetails(args);
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.gui_comparison.reason, 'inspection_failed');
    assert.equal('walk_failure' in result.gui_comparison, false);
    assert.equal('topParameterSame' in result.gui_comparison, false);
    assert.equal(h.calls.length, 1);
  }
  for (const [kind, stage, depth, count, query_status] of [
    ['view_limit', 'children', 0, 128, null], ['missing_interface', 'child_query', 1, 2, -2147467262],
    ['missing_iterator', 'iterator', 0, 1, null], ['missing_view', 'root', 0, 0, null],
    ['parameter_metadata', 'parameter_metadata', 2, 3, null], ['context_guard_failed', 'cleanup', 2, 3, null],
    ['inspection_failed', 'parameter_metadata', 1, 2, null],
    ['depth_limit', 'children', 32, 33, null],
    ['interface_query_failed', 'control_query', 31, 32, -2147467259],
    ['interface_query_failed', 'control_query', 32, 33, -2147467259],
  ]) {
    const report = base(); report.coverage.visited_views = count;
    report.walk_failure = { kind, stage, depth, query_status };
    const result = harness(() => success(report)).component.pluginParameterDetails(args);
    assert.equal(result.outcome, 'observed');
    assert.equal(result.gui_comparison.walk_failure.kind, kind);
  }
  const failed = harness(() => ({ ok: false, outcome: 'not_applied', gui_comparison: base(),
    error: { code: 'read_failed', message: 'No comparison' } })).component.pluginParameterDetails(args);
  assert.equal(failed.native_outcome, 'not_applied');
  assert.equal(failed.gui_comparison.walk_failure.kind, 'interface_query_failed');
  assert.equal('topParameterSame' in failed.gui_comparison, false);
});

test('GUI descriptor corruption discards both the descriptor and receiver identity claims', () => {
  for (const mutate of [d => { d.name = 'chorder.autofill'; }, d => { d.native_value.variant_tag = 2; },
    d => { d.native_value.data.value = 1; }, d => { d.native_normalized.bits_hex = '3F800000'; },
    d => { d.support.raw = 1; }, d => { d.pointer = 123; }, d => { d.display_parser = {}; }]) {
    const h = harness(() => { const result = success(); mutate(result.gui_comparison.gui_parameter); return result; });
    const result = h.component.pluginParameterDetails(args);
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('gui_parameter' in result.gui_comparison, false);
    assert.equal('topParameterSame' in result.gui_comparison, false);
    assert.equal(h.calls.length, 1);
  }
});

test('fixed GUI controller metadata requires both existing opt-ins and a compared base', () => {
  for (const include_owner of [undefined, false]) {
    const result = harness(() => success(withProbe())).component.pluginParameterDetails({ ...args, include_owner });
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('controller_probe' in result.gui_comparison, false);
  }
  const missing = harness(() => success()).component.pluginParameterDetails({ ...args, include_owner: true });
  assert.equal(missing.gui_comparison.status, 'unresolved');
  for (const report of [unresolved('missing_editor'), { ...unresolved('missing_editor'), controller_probe: controllerProbe() }]) {
    const result = harness(() => success(report)).component.pluginParameterDetails({ ...args, include_owner: true });
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('controller_probe' in result.gui_comparison, false);
  }
});

test('fixed GUI controller routes and signed selector observations preserve exact native values', () => {
  const routes = [
    probe => probe,
    probe => { probe.binding.route = 'provider'; probe.interfaces.controller.status = 'unsupported';
      probe.interfaces.provider.status = 'present'; probe.anchors = { controller: 'controller_absent', provider: 'same_parameter' }; return probe; },
    probe => { probe.binding.route = 'both'; probe.interfaces.provider.status = 'present';
      probe.anchors.provider = 'same_parameter'; return probe; },
    probe => { probe.interfaces.provider.status = 'present'; probe.anchors.provider = 'controller_absent'; return probe; },
    probe => { probe.interfaces.provider.status = 'present'; probe.anchors.provider = 'identity_mismatch'; return probe; },
  ];
  for (const route of routes) {
    for (const value of [-1, 0, 127]) {
      const probe = route(controllerProbe());
      probe.parameters.learnInputPitch = modelDescription('learnInputPitch', value);
      const h = harness(() => success(withProbe(probe)));
      const result = h.component.pluginParameterDetails({ ...args, include_owner: true });
      assert.equal(result.gui_comparison.status, 'compared');
      assert.deepEqual(JSON.parse(JSON.stringify(result.gui_comparison.controller_probe)), probe);
      assert.equal(result.gui_comparison.controller_probe.parameters.learnInputPitch.value, value);
      assert.equal(result.gui_comparison.controller_probe.parameters.learnInputPitch.min, -1);
      assert.equal(result.gui_comparison.controller_probe.parameters.learnInputPitch.max, 127);
      assert.equal(result.gui_comparison.controller_probe.mutation_capability, false);
      assert.equal(h.calls.length, 1);
      assert.equal(h.calls[0].operation, 'parameter_details');
    }
  }
});

test('fixed GUI controller unresolved facts keep assessed routes but expose no model pair', () => {
  const variants = [
    probe => { probe.reason = 'observer_absent'; probe.interfaces = { controller: { status: 'not_attempted' }, provider: { status: 'not_attempted' } };
      probe.anchors = { controller: 'not_attempted', provider: 'not_attempted' }; probe.observer_metadata = { status: 'absent' }; },
    probe => { probe.reason = 'interface_query_failed'; probe.interfaces.controller = { status: 'failed', query_status: 0 };
      probe.anchors.controller = 'not_attempted'; },
    probe => { probe.reason = 'interface_query_failed'; probe.interfaces.provider = { status: 'failed', query_status: -2147467262 };
      probe.anchors.provider = 'not_attempted'; },
    probe => { probe.reason = 'no_anchored_controller'; probe.anchors.controller = 'identity_mismatch'; },
    probe => { probe.reason = 'ambiguous_controller_routes'; probe.interfaces.provider.status = 'present'; probe.anchors.provider = 'same_parameter'; },
    probe => { probe.reason = 'parameter_missing'; },
  ];
  for (const variant of variants) {
    const probe = controllerProbe();
    probe.status = 'unresolved';
    delete probe.binding;
    delete probe.parameters;
    variant(probe);
    const result = harness(() => success(withProbe(probe))).component.pluginParameterDetails({ ...args, include_owner: true });
    assert.equal(result.gui_comparison.status, 'compared');
    assert.deepEqual(JSON.parse(JSON.stringify(result.gui_comparison.controller_probe)), probe);
    assert.equal('parameters' in result.gui_comparison.controller_probe, false);
    assert.equal('binding' in result.gui_comparison.controller_probe, false);
  }
});

test('unresolved fixed GUI reasons must agree with the completed interface and anchor facts', () => {
  const mutations = [
    p => { p.reason = 'observer_absent'; },
    p => { p.reason = 'interface_query_failed'; },
    p => { p.reason = 'no_anchored_controller'; },
    p => { p.reason = 'ambiguous_controller_routes'; },
    p => { p.reason = 'parameter_missing'; p.anchors.controller = 'parameter_absent'; },
    p => { p.reason = 'stale_association'; },
    p => { p.reason = 'interface_query_failed'; p.interfaces.controller = { status: 'failed', query_status: 0 };
      p.anchors.controller = 'not_attempted'; p.interfaces.provider = { status: 'not_attempted' }; p.anchors.provider = 'not_attempted'; },
    p => { p.reason = 'no_anchored_controller'; p.interfaces.controller = { status: 'failed', query_status: 0 }; p.anchors.controller = 'not_attempted'; },
  ];
  for (const mutate of mutations) {
    const probe = controllerProbe();
    probe.status = 'unresolved';
    delete probe.binding;
    delete probe.parameters;
    mutate(probe);
    const result = harness(() => success(withProbe(probe))).component.pluginParameterDetails({ ...args, include_owner: true });
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('controller_probe' in result.gui_comparison, false);
  }
});

test('fixed GUI observer reflection retains honest bounded partial and unavailable facts', () => {
  for (const change of [
    report => { report.nodes[0].reflection = { available: false, types: [], issues: ['no_object_interface'] }; report.complete = false; },
    report => { report.nodes[0].identity = { id: 'root', object_uid: null, class_uid: instance.class_uid }; report.nodes[0].child_count = 2;
      report.diagnostics = [{ kind: 'child_limit', node: 0 }, { kind: 'depth_limit', node: 0 }, { kind: 'ancestor_limit' }]; report.complete = false; },
    report => { report.nodes[0].reflection.issues = ['method_limit']; report.complete = false; },
  ]) {
    const probe = controllerProbe();
    change(probe.observer_metadata.report);
    const result = harness(() => success(withProbe(probe))).component.pluginParameterDetails({ ...args, include_owner: true });
    assert.equal(result.gui_comparison.status, 'compared');
    assert.deepEqual(JSON.parse(JSON.stringify(result.gui_comparison.controller_probe.observer_metadata)), probe.observer_metadata);
  }
});

test('malformed fixed GUI metadata and contradictory route claims erase the entire comparison', () => {
  const mutations = [
    p => { p.scope = 'arbitrary_controller'; }, p => { p.mutation_capability = true; }, p => { p.value_semantics = 'verified'; },
    p => { p.pointer = 1; }, p => { p.token = 'authority'; }, p => { p.binding.pointer = 1; }, p => { p.binding.autofill_same_gui = false; },
    p => { p.binding.route = 'first'; }, p => { p.binding.route = 'both'; }, p => { p.binding.controller_same_observer = null; },
    p => { p.interfaces.controller = { status: 'failed', query_status: 1.5 }; },
    p => { p.interfaces.controller = { status: 'present', query_status: 0 }; },
    p => { p.interfaces.controller = { status: 'failed', query_status: 0, output: 1 }; },
    p => { p.interfaces.provider.status = 'not_attempted'; p.anchors.provider = 'not_attempted'; },
    p => { p.interfaces.provider.status = 'failed'; p.interfaces.provider.query_status = -1; p.anchors.provider = 'not_attempted'; },
    p => { p.anchors.controller = 'identity_mismatch'; }, p => { p.anchors.provider = 'same_parameter'; },
    p => { p.parameters.learnInputPitch.name = 'chorder.corePitch'; }, p => { p.parameters.learnInputPitch.title = 'Pitch'; },
    p => { p.parameters.learnInputPitch.value = 0; }, p => { p.parameters.learnInputPitch.min = 0; },
    p => { p.parameters.learnInputPitch.native_value.variant_tag = 2; }, p => { p.parameters.learnMode.native_normalized.bits_hex = '3F800000'; },
    p => { delete p.parameters.learnMode; }, p => { p.parameters.learnChord = guiDescription(); },
    p => { p.observer_metadata.status = 'absent'; delete p.observer_metadata.report; },
    p => { p.observer_metadata.report.limits.nodes = 2; }, p => { p.observer_metadata.report.mutation_capability = true; },
    p => { p.observer_metadata.report.pointer = 1; }, p => { p.observer_metadata.report.nodes[0].identity = { id: 'a', object_uid: null, class_uid: null }; },
    p => { p.observer_metadata.report.nodes[0].reflection.types[0].methods[0].address = 1; },
    p => { p.observer_metadata.report.nodes[0].reflection.types[0].properties[0].data_type = '1'; },
    p => { p.observer_metadata.report.nodes[0].reflection.types[0].name = 'x'.repeat(513); },
    p => { p.observer_metadata.report.nodes[0].reflection.available = false; },
    p => { p.observer_metadata.report.nodes[0].reflection.issues = ['not_supported']; },
    p => { p.observer_metadata.report.nodes[0].reflection.types[0].methods = Array(257).fill({ name: 'a', arguments: null, return_type: null }); },
    p => { p.observer_metadata.report.diagnostics = [{ kind: 'depth_limit', node: 1 }]; p.observer_metadata.report.complete = false; },
    p => { p.observer_metadata.report.complete = false; },
    p => { p.status = 'unresolved'; p.reason = 'parameter_missing'; },
  ];
  for (const mutate of mutations) {
    const probe = controllerProbe();
    mutate(probe);
    const result = harness(() => success(withProbe(probe))).component.pluginParameterDetails({ ...args, include_owner: true });
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('controller_probe' in result.gui_comparison, false);
    assert.equal('gui_parameter' in result.gui_comparison, false);
    assert.equal('topParameterSame' in result.gui_comparison, false);
  }
});

test('fixed GUI controller observations disappear on soft native errors or either publication boundary', () => {
  for (const change of [
    result => { result.error = 'native comparison failed'; }, result => { result.errors = ['late read failure']; },
    result => { result.native_error = { code: 'stale' }; }, result => { result.ok = false; result.outcome = 'not_applied'; },
    result => { result.instance.object_uid = '87654321-4321-4321-4321-CBA987654321'; },
  ]) {
    const result = harness(() => { const value = { ...success(withProbe()), instance: { ...instance } }; change(value); return value; })
      .component.pluginParameterDetails({ ...args, include_owner: true });
    assert.equal(result.gui_comparison.status, 'unresolved');
    assert.equal('controller_probe' in result.gui_comparison, false);
  }
  const changed = harness((request, context) => { context.document = 'gui:2'; return success(withProbe()); })
    .component.pluginParameterDetails({ ...args, include_owner: true });
  assert.equal('controller_probe' in changed.gui_comparison, false);
  const host = bridgeHarness();
  host.bridge.component = { pluginParameterDetails() {
    host.manager.activeDocument = { title: 'Different' };
    return success(withProbe());
  } };
  const response = host.request({ op: 'pluginParameterDetails', args: { ...args, include_owner: true } });
  assert.equal(response.result.gui_comparison.status, 'unresolved');
  assert.equal('controller_probe' in response.result.gui_comparison, false);
});
