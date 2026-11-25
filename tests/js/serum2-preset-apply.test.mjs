import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const fixture = 'mcp-serum2-preset-12345678-1234-4234-8234-123456789abc.vstpreset';
const presetPath = '/private/tmp/Scratch Studio/' + fixture;
const instance = '12345678-1234-1234-1234-123456789ABC';
const digest = 'a'.repeat(64);
const fx = { target: { variant: 'fx', channel: 'Scratch', slot: 0 },
  expected_instance: instance, fixture, sha256: digest };

test('preset dispatch resolves only a private staged basename with stopped transport', () => {
  const host = bridgeHarness();
  const calls = [];
  host.bridge.component = {
    serum2PresetApply(args, path) { calls.push([args, path]); return { ok: true }; },
  };
  host.bridge.mailbox.url = name => ({ url: 'file:///private/tmp/Scratch%20Studio/' + name });
  host.bridge.song = () => ({ transport: { playing: false, recording: false } });
  assert.equal(host.bridge.serum2PresetApply({ ...fx, fixture: '../x.vstpreset' }).native_outcome, 'not_applied');
  assert.equal(host.bridge.serum2PresetApply(fx).ok, true);
  assert.deepEqual(calls, [[fx, presetPath]]);
  host.bridge.song = () => ({ transport: { playing: true, recording: false } });
  assert.equal(host.bridge.serum2PresetApply(fx).native_outcome, 'not_applied');
  assert.equal(calls.length, 1);
});

function componentHarness(options = {}) {
  const calls = [], device = {};
  const current = { open: true, document: 'serum:1', session: 'serum' };
  const service = {
    capabilities() { return JSON.stringify({ ok: true, api: 'fender-mcp-native-v1',
      serum2_preset_apply_api: options.old ? undefined : 'vst3_single_component_v1' }); },
    request(...parts) {
      calls.push(parts);
      if (options.lost) throw new Error('Lost native reply');
      return JSON.stringify({ ok: true, outcome: 'applied_unverified', verified: false,
        sha256: options.wrongDigest ? 'b'.repeat(64) : digest,
        instance: { object_uid: instance, class_uid: '56534558-6673-5173-6572-756D20322066' } });
    },
  };
  const scope = vm.createContext({ include_file() {}, PreSonus: { ControlSurfaceComponent: class {} },
    Host: { Url: value => ({ value }), Objects: { getObjectByName: () => service } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { requestContext: current, context: () => ({ ...current }) };
  component.pluginOf = selector => {
    assert.deepEqual(JSON.parse(JSON.stringify(selector)), { channel: 'Scratch', slot: 0, plugin: 'Serum 2 FX' });
    return { name: 'Serum 2 FX', device };
  };
  return { component, calls, device };
}

test('preset application binds the exact native instance, staged digest and URL auxiliary', () => {
  const h = componentHarness();
  const receipt = h.component.serum2PresetApply(fx, presetPath);
  assert.equal(receipt.native_outcome, 'applied_unverified');
  assert.equal(receipt.verified, false);
  assert.equal(receipt.sha256, digest);
  assert.equal(receipt.instance.object_uid, instance);
  const [target, body, auxiliary] = h.calls[0];
  assert.equal(target, h.device);
  assert.equal(auxiliary.value, '');
  assert.deepEqual(JSON.parse(body), { operation: 'serum2_preset_apply',
    expected_instance: instance, path: presetPath, sha256: digest });
});

test('old extensions refuse before dispatch and malformed or lost replies are unknown', () => {
  const old = componentHarness({ old: true });
  assert.equal(old.component.serum2PresetApply(fx, presetPath).native_outcome, 'not_applied');
  assert.equal(old.calls.length, 0);
  for (const options of [{ wrongDigest: true }, { lost: true }]) {
    const h = componentHarness(options);
    assert.equal(h.component.serum2PresetApply(fx, presetPath).native_outcome, 'unknown');
    assert.equal(h.calls.length, 1);
  }
});
