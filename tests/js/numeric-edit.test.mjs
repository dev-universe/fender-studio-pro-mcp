import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: 'D2D9B002-12FC-4F1D-AFDA-70336D1DC108' };
const args = { kind: 'note_effect', track: 'Scratch', slot: 0, plugin: 'Chorder',
  source: 'dsp', param: 'chorder.autofill', value: 1, expected_instance: instance.object_uid, edit_lifecycle: 'native_user' };
const observed = () => ({ ok: true, outcome: 'observed', instance, source: 'dsp', edit_phase: 'closed_checked',
  verified: true, before: { value: 0 }, after: { value: 1 } });

function harness(reply = observed, capability = 'chorder_autofill_v1') {
  const calls = [];
  const context = { open: true, document: 'numeric:1', session: 'numeric' };
  const service = {
    capabilities() { return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1', numeric_edit_api: capability }); },
    request(device, body) {
      const request = JSON.parse(body);
      calls.push(request);
      if (request.operation === 'inspect') return JSON.stringify({ ok: true, instance });
      const result = reply(request, context);
      return typeof result === 'string' ? result : JSON.stringify(result);
    },
  };
  const scope = vm.createContext({ include_file() {}, memberTypes() { return {}; }, PreSonus: { ControlSurfaceComponent: class {} },
    Host: { Objects: { getObjectByName() { return service; } } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: { ...context }, context() { return { ...context }; } };
  const device = { findParameter() { throw new Error('numeric edit cannot fall back'); } };
  component.pluginOf = () => ({ name: 'Chorder', device });
  return { component, calls, context, service, device };
}

test('numeric edit opt-in sends one closed native mutation and preserves live readback only', () => {
  const h = harness();
  const result = h.component.setPluginParam(args);
  assert.deepEqual(h.calls, [{ operation: 'inspect', expected_instance: instance.object_uid },
    { operation: 'edit_numeric_parameter', param: 'chorder.autofill', value: 1, expected_instance: instance.object_uid }]);
  assert.equal(result.native_verified, true);
  assert.equal(result.edit_phase, 'closed_checked');
  assert.equal(result.persistent, undefined);
  assert.equal(result.after.value, 1);
  const mismatch = harness(() => ({ ...observed(), verified: false, after: { value: 0 } })).component.setPluginParam(args);
  assert.equal(mismatch.native_verified, false);
  assert.equal(mismatch.edit_phase, 'closed_checked');
});

test('omitting the selector preserves the legacy native request', () => {
  const h = harness();
  const { edit_lifecycle, ...legacy } = args;
  assert.equal(h.component.setPluginParam(legacy).native_verified, true);
  assert.deepEqual(h.calls, [{ operation: 'inspect', expected_instance: instance.object_uid },
    { operation: 'set_parameter', source: 'dsp', param: 'chorder.autofill', mode: 'raw', value: 1, expected_instance: instance.object_uid }]);
});

test('invalid edit selectors and combinations refuse before Device resolution', () => {
  const h = harness();
  h.component.pluginOf = () => { throw new Error('invalid edit must not resolve a Device'); };
  for (const extra of [
    { edit_lifecycle: null }, { edit_lifecycle: false }, { edit_lifecycle: 'other' }, { edit_lifecycle: {} },
    { source: undefined }, { source: 'controller' }, { source: 'auto' },
    { expected_instance: undefined }, { expected_instance: '' }, { expected_instance: instance.object_uid.toLowerCase() },
    { expected_instance: '00000000-0000-0000-0000-000000000000' },
    { value: undefined }, { value: null }, { value: '1' }, { value: true }, { value: 0.5 }, { value: 2 },
    { value: NaN }, { value: Infinity }, { text: 'On' }, { normalized: 1 }, { float_bits: '3FF0000000000000' },
    { param: '' }, { param: 'x'.repeat(513) }, { param: 'x\0y' },
  ]) assert.equal(h.component.setPluginParam({ ...args, ...extra }).native_outcome, 'not_applied');
  assert.equal(h.calls.length, 0);
});

test('unsupported capability refuses the edit without mutation or fallback', () => {
  for (const capability of [undefined, null, 'old']) {
    const h = harness(observed, capability);
    if (capability === undefined) h.service.capabilities = () => '{"ok":true,"api":"fender-mcp-native-v1"}';
    assert.equal(h.component.setPluginParam(args).native_outcome, 'not_applied');
    assert.equal(h.calls.filter(call => call.operation === 'edit_numeric_parameter').length, 0);
  }
});

test('malformed edit replies after dispatch remain unknown with no retry', () => {
  const malformed = [() => null, () => false, () => '{',
    () => ({ ...observed(), ok: 1 }), () => ({ ...observed(), ok: undefined }),
    () => ({ outcome: 'not_applied', edit_phase: 'prepared', error: { code: 'refused', message: 'Unavailable' } }),
    () => ({ ...observed(), edit_phase: 'end_dispatched' }), () => ({ ...observed(), verified: undefined }),
    () => ({ ...observed(), before: null }), () => ({ ...observed(), after: [] }),
    () => ({ ...observed(), source: 'controller' }), () => ({ ...observed(), outcome: 'applied_verified' }),
    () => ({ ...observed(), instance: { ...instance, object_uid: '87654321-4321-4321-4321-CBA987654321' } }),
    () => ({ ok: false, outcome: 'not_applied', edit_phase: 'begin_dispatched', error: { code: 'guard', message: 'changed' } }),
    () => { throw new Error('transport lost'); },
  ];
  for (const reply of malformed) {
    const h = harness(reply);
    const result = h.component.setPluginParam(args);
    assert.equal(result.native_outcome, 'unknown');
    assert.notEqual(result.native_verified, true);
    assert.equal(h.calls.filter(call => call.operation === 'edit_numeric_parameter').length, 1);
  }
});

test('known pre-begin refusal and observed unknown phases remain distinct', () => {
  const failure = (phase, outcome) => ({ ok: false, outcome, edit_phase: phase, source: 'dsp', instance,
    error: { code: 'guard_changed', message: 'The binding changed' } });
  assert.equal(harness(() => failure('prepared', 'not_applied')).component.setPluginParam(args).native_outcome, 'not_applied');
  for (const phase of ['begin_dispatched', 'value_dispatched', 'end_dispatched', 'closed_checked']) {
    const result = harness(() => failure(phase, 'unknown')).component.setPluginParam(args);
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.edit_phase, phase);
  }
});

test('a changed document after native edit preserves unknown and the observed phase', () => {
  const h = harness((request, context) => { context.document = 'numeric:2'; return observed(); });
  const result = h.component.setPluginParam(args);
  assert.equal(result.native_outcome, 'unknown');
  assert.equal(result.edit_phase, 'closed_checked');
  assert.equal(h.calls.filter(call => call.operation === 'edit_numeric_parameter').length, 1);
});

test('closed native edit grammar rejects extra selectors before native invocation', () => {
  const h = harness();
  const body = { operation: 'edit_numeric_parameter', param: args.param, value: 1, expected_instance: instance.object_uid };
  for (const extra of [{ source: 'dsp' }, { mode: 'raw' }, { path: 'file' }, { value: 0.5 }, { expected_instance: '' }]) {
    const result = h.component.nativePluginRequest(h.device, { ...body, ...extra });
    assert.equal(result.outcome, 'not_applied');
  }
  assert.equal(h.calls.length, 0);
});

test('edit opt-in remains a document-bound bridge mutation', () => {
  const host = bridgeHarness();
  let calls = 0;
  host.bridge.component = { setPluginParam() { calls++; return { edit_phase: 'closed_checked', native_verified: true }; } };
  assert.equal(host.request({ op: 'setPluginParam', args }).code, 'context_required');
  assert.equal(host.writeRequest({ op: 'setPluginParam', args, expectedDocument: 'stale' }).code, 'document_changed');
  assert.equal(calls, 0);
  assert.equal(host.writeRequest({ op: 'setPluginParam', args }).result.edit_phase, 'closed_checked');
  assert.equal(calls, 1);
});
