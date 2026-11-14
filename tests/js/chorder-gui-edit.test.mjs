import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';
import { guiDescription } from './gui-description.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: 'D2D9B002-12FC-4F1D-AFDA-70336D1DC108' };
const args = { track: 'Scratch', slot: 0, enabled: true, expected_instance: instance.object_uid };
const success = (kind = 'integer', enabled = true) => ({ ok: true, outcome: 'applied_unverified',
  operation: 'edit_chorder_autofill', instance, binding_source: 'dsp', logical_parameter: 'chorder.autofill', receiver: 'editor_control',
  requested_enabled: enabled, edit_phase: 'closed_checked', gui_parameter: { name: 'autofill', before: guiDescription(enabled ? 0 : 1, kind),
    after: guiDescription(enabled ? 1 : 0, kind) }, gui_value_verified: true, verification_scope: 'gui_receiver_current_value',
  persistence_verified: false, effects_verified: false, dsp_propagation_verified: false });

function harness(reply = () => success(), capability = 'chorder_autofill_v1') {
  const calls = [], targets = [];
  const context = { open: true, document: 'guiedit:1', session: 'guiedit' };
  const service = {
    capabilities() { return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1', gui_edit_api: capability }); },
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
  component.pluginOf = target => { targets.push(target); return { name: 'Chorder', device: {} }; };
  const native = vm.runInContext('new NativePluginControls({})', scope);
  return { component, context, calls, targets, native, service };
}

test('finite GUI edit fixes receiver policy and keeps typed GUI-only verification unpromoted', () => {
  for (const kind of ['integer', 'float']) for (const enabled of [false, true]) {
    const h = harness(() => success(kind, enabled));
    const result = h.component.editChorderAutofill({ ...args, enabled });
    assert.deepEqual(h.calls, [{ operation: 'edit_chorder_autofill', enabled, expected_instance: instance.object_uid }]);
    assert.equal(JSON.stringify(h.targets), JSON.stringify([{ kind: 'note_effect', track: 'Scratch', slot: 0, plugin: 'Chorder' }]));
    assert.equal(result.native_outcome, 'applied_unverified');
    assert.equal(result.gui_value_verified, true);
    assert.equal(result.gui_parameter.after.native_value.data.kind, kind);
    for (const flag of ['persistence_verified', 'effects_verified', 'dsp_propagation_verified']) assert.equal(result[flag], false);
    for (const field of ['before', 'after', 'source', 'native_verified', 'verified']) assert.equal(field in result, false);
  }
});

test('GUI edit invalid intent refuses before resolving a target or dispatching', () => {
  const h = harness(); h.component.pluginOf = () => { throw new Error('invalid request resolved'); };
  const invalid = [{ enabled: undefined }, { enabled: null }, { enabled: 1 }, { enabled: 'true' }, { enabled: {} },
    { track: '' }, { track: '  ' }, { track: 'x\0y' }, { track: 'x'.repeat(257) },
    { slot: undefined }, { slot: -1 }, { slot: 16 }, { slot: 0.5 }, { slot: '0' },
    { expected_instance: '' }, { expected_instance: instance.object_uid.toLowerCase() },
    { expected_instance: '00000000-0000-0000-0000-000000000000' },
    ...['source', 'param', 'plugin', 'kind', 'path', 'value', 'normalized', 'edit_lifecycle'].map(key => ({ [key]: 'unexpected' }))];
  for (const extra of invalid) assert.equal(h.component.editChorderAutofill({ ...args, ...extra }).native_outcome, 'not_applied');
  assert.equal(h.calls.length, 0);
});

test('GUI edit requires exact resolved class and versioned capability without fallback', () => {
  const wrong = harness(); wrong.component.pluginOf = () => ({ name: 'Other', device: {} });
  assert.equal(wrong.component.editChorderAutofill(args).native_outcome, 'not_applied');
  assert.equal(wrong.calls.length, 0);
  for (const capability of [null, '', 'old', true]) {
    const h = harness(undefined, capability);
    assert.equal(h.component.editChorderAutofill(args).native_outcome, 'not_applied');
    assert.equal(h.calls.length, 0);
  }
  const h = harness();
  for (const extra of [{ path: 'autofill' }, { source: 'dsp' }, { enabled: 1 }, { expected_instance: '' }])
    assert.equal(h.component.nativePluginRequest({}, { operation: 'edit_chorder_autofill', enabled: true,
      expected_instance: instance.object_uid, ...extra }).outcome, 'not_applied');
  assert.equal(h.calls.length, 0);
});

test('forged GUI success fields and direct facade evidence become unknown after exactly one dispatch', () => {
  const mutate = [r => { r.ok = 1; }, r => { r.outcome = 'applied_verified'; }, r => { r.edit_phase = 'end_dispatched'; },
    r => { r.instance.class_uid = '12345678-1234-1234-1234-123456789ABC'; },
    r => { r.instance.object_uid = '87654321-4321-4321-4321-CBA987654321'; },
    r => { r.receiver = 'dsp'; }, r => { r.logical_parameter = 'autofill'; }, r => { r.binding_source = 'controller'; },
    r => { r.requested_enabled = false; }, r => { r.gui_value_verified = 'true'; },
    r => { r.verification_scope = 'dsp'; }, r => { r.persistence_verified = true; }, r => { r.effects_verified = true; },
    r => { r.dsp_propagation_verified = true; }, r => { r.before = { value: 0 }; r.after = { value: 1 }; },
    r => { r.native_verified = true; }, r => { r.error = { code: 'failed', message: 'conflict' }; },
    r => { r.gui_parameter.name = 'chorder.autofill'; }, r => { r.gui_parameter.after.name = 'chorder.autofill'; },
    r => { r.gui_parameter.after.native_value.data.value = 0; }, r => { r.gui_parameter.after.value = 0; },
    r => { r.gui_parameter.after.native_value.variant_tag = 2; }, r => { r.gui_parameter.after.native_kind = 'Float'; },
    r => { r.gui_parameter.after.max = 2; }, r => { r.gui_parameter.after.native_max.data.value = 2; },
    r => { r.gui_parameter.after.native_normalized.bits_hex = '00000000'; },
    r => { r.gui_parameter.after.support.raw = false; }, r => { r.gui_parameter.after.extra = true; },
    r => { for (const stage of ['before', 'after']) for (const key of ['native_value', 'native_min', 'native_max']) r.gui_parameter[stage][key].variant_tag |= 4096; },
    r => { delete r.gui_parameter; }];
  for (const change of mutate) {
    const h = harness(() => { const result = JSON.parse(JSON.stringify(success())); change(result); return result; });
    const result = h.component.editChorderAutofill(args);
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.gui_value_verified, false);
    assert.equal('gui_parameter' in result, false);
    assert.equal(h.calls.length, 1);
  }
});

test('GUI float carrier bits cannot disagree with scalar numbers or requested state', () => {
  for (const mutate of [r => { r.gui_parameter.after.native_value.data.bits_hex = '0000000000000000'; },
    r => { r.gui_parameter.after.native_min.data.bits_hex = '3FF0000000000000'; },
    r => { r.gui_parameter.after.native_value.data.bits_hex = '7FF0000000000000'; },
    r => { r.gui_parameter.after.native_value.data.bits_hex = '3ff0000000000000'; }]) {
    const h = harness(() => { const r = success('float'); mutate(r); return r; });
    assert.equal(h.component.editChorderAutofill(args).native_outcome, 'unknown');
    assert.equal(h.calls.length, 1);
  }
});

test('known pre-begin refusal and unknown lifecycle phases keep no GUI positives', () => {
  for (const phase of [undefined, 'prepared']) {
    const h = harness(() => ({ ok: false, outcome: 'not_applied', ...(phase && { edit_phase: phase }),
      error: { code: 'missing_editor', message: 'No editor' } }));
    const result = h.component.editChorderAutofill(args);
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.gui_value_verified, undefined);
    assert.equal(h.calls.length, 1);
  }
  for (const phase of ['begin_dispatched', 'value_dispatched', 'end_dispatched', 'closed_checked']) {
    const h = harness(() => ({ ok: false, outcome: 'unknown', edit_phase: phase,
      error: { code: 'stale_binding', message: 'The receiver changed' } }));
    assert.equal(h.component.editChorderAutofill(args).native_outcome, 'unknown');
    assert.equal(h.calls.length, 1);
    const contradictory = harness(() => ({ ok: false, outcome: 'not_applied', edit_phase: phase,
      error: { code: 'stale_binding', message: 'The receiver changed' } }));
    assert.equal(contradictory.component.editChorderAutofill(args).native_outcome, 'unknown');
  }
});

test('malformed transport and context changes never trigger a GUI retry or original write', () => {
  for (const reply of [() => '{', () => null, () => { throw new Error('lost'); },
    (request, context) => { context.document = 'guiedit:2'; return success(); }]) {
    const h = harness(reply); const result = h.component.editChorderAutofill(args);
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.gui_value_verified, false);
    assert.equal('gui_parameter' in result, false);
    assert.equal(h.calls.length, 1);
  }
});

test('GUI edit bridge operation requires document context and strips late success evidence', () => {
  const h = bridgeHarness();
  h.bridge.component = { editChorderAutofill() { return success(); } };
  assert.equal(h.request({ op: 'editChorderAutofill', args }).code, 'context_required');
  const receipt = h.writeRequest({ op: 'editChorderAutofill', args });
  assert.equal(receipt.result.outcome, 'applied_unverified');
  h.bridge.component = { editChorderAutofill() { h.manager.activeDocument = { title: 'Changed' }; return success(); } };
  const stale = h.writeRequest({ op: 'editChorderAutofill', args });
  assert.equal(stale.result.native_outcome, 'unknown');
  assert.equal(stale.result.gui_value_verified, false);
  assert.equal('gui_parameter' in stale.result, false);
});

test('GUI metadata preserves descriptive unsupported profiles without granting edit authority', () => {
  const h = harness();
  for (const kind of ['integer', 'float']) {
    const descriptor = guiDescription(0, kind);
    assert.equal(h.native.guiDescription(descriptor), true);
    descriptor.native_kind = 'Float'; descriptor.readOnly = true;
    descriptor.writable = false;
    descriptor.native_support = { raw_number: false, normalized: false, display_text: false };
    descriptor.support = { raw: false, normalized: false, text: false };
    assert.equal(h.native.guiDescription(descriptor), true);
    assert.equal(h.native.guiToggleProfile(descriptor), false);
  }
  const descriptor = guiDescription(); descriptor.extra = true;
  assert.equal(h.native.guiDescription(descriptor), false);
});

test('GUI typed Float validation distinguishes exact bits across the finite IEEE754 range', () => {
  const h = harness(), buffer = new ArrayBuffer(8), view = new DataView(buffer);
  const words = [[0, 0], [0x80000000, 0], [0, 1], [0x80000000, 1], [0x00100000, 0], [0x7fefffff, 0xffffffff],
    [0xffefffff, 0xffffffff], [0x3ff00000, 0], [0x7ff00000, 0], [0x7ff00000, 1]];
  let seed = 0x13579bdf;
  for (let i = 0; i < 256; i++) {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; const high = seed;
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; words.push([high, seed]);
  }
  for (const [high, low] of words) {
    view.setUint32(0, high); view.setUint32(4, low);
    const value = view.getFloat64(0);
    const bits_hex = high.toString(16).padStart(8, '0').toUpperCase() + low.toString(16).padStart(8, '0').toUpperCase();
    const typed = { variant_tag: 2, data: { kind: 'float', value, bits_hex } };
    assert.equal(h.native.guiTypedScalar(typed, value), Number.isFinite(value), bits_hex);
    if (Number.isFinite(value)) assert.equal(h.native.guiTypedScalar(typed, value === 1 ? 0 : 1), false, bits_hex);
  }
});

test('GUI typed descriptions keep unsafe and opaque observation data outside edit authority', () => {
  const h = harness();
  const unsafe = guiDescription();
  unsafe.native_value = { variant_tag: 1, data: { kind: 'integer', exact_decimal: '9223372036854775807' } };
  unsafe.value = null; unsafe.writable = false;
  unsafe.support = { raw: false, normalized: false, text: false };
  assert.equal(h.native.guiDescription(unsafe), true);
  assert.equal(h.native.guiToggleProfile(unsafe), false);
  unsafe.native_value.data.exact_decimal = '9223372036854775808';
  assert.equal(h.native.guiDescription(unsafe), false);
  const opaque = guiDescription(); opaque.native_value = { variant_tag: 4, data: { kind: 'opaque_object', present: true } };
  opaque.value = null; opaque.writable = false;
  assert.equal(h.native.guiDescription(opaque), true);
  assert.equal(h.native.guiToggleProfile(opaque), false);
});
