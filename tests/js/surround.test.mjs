import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset } from './host.mjs';

function fixture() {
  const writes = [];
  const azimuth = { name: 'azimuth', value: 0, min: -180, max: 180,
    setValue(v) { writes.push(v); this.value = v; } };
  const bypass = { name: 'bypass', value: 0, min: 0, max: 1,
    setValue(v) { this.value = v; } };
  const device = { name: 'Surround Panner', findParameter: n => n === 'azimuth' ? azimuth : null };
  const slot = { find: p => p === 'Device' ? device : null,
    findParameter: n => n === 'deviceName' ? { value: 'Surround Panner' } : n === 'bypass' ? bypass : null };
  const paths = [];
  const channel = { find(p) { paths.push(p); return p === 'Panner/AudioPanFX/FXSlot' ? slot : null; } };
  const scope = vm.createContext({ include_file() {}, PreSonus: { ControlSurfaceComponent: class {} } });
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.bridge = { mixerChannel: () => channel };
  const target = { kind: 'panner', channel: '5.1 Scratch', slot: 0, plugin: 'Surround Panner' };
  return { component, target, writes, azimuth, device, slot, paths };
}

test('Surround Panner uses its native PanFX slot for read, write and restore', () => {
  const f = fixture();
  const read = f.component.pluginParams({ ...f.target, names: ['azimuth', 'bypass', 'missing'] });
  assert.deepEqual(Array.from(read.params, p => p.name), ['azimuth', 'bypass']);
  assert.deepEqual(Array.from(read.missing), ['missing']);
  const changed = f.component.setPluginParam({ ...f.target, param: 'azimuth', value: 30 });
  assert.equal(changed.before.value, 0);
  assert.equal(changed.after.value, 30);
  const restored = f.component.setPluginParam({ ...f.target, param: 'azimuth', value: 0 });
  assert.equal(restored.after.value, 0);
  assert.deepEqual(f.writes, [30, 0]);
  assert.ok(f.paths.every(p => p === 'Panner/AudioPanFX/FXSlot'));
});

test('Surround Panner refuses mismatched identity, absent devices and additional slots', () => {
  const f = fixture();
  for (const target of [{ ...f.target, plugin: 'Other' }, { ...f.target, slot: 1 }]) {
    assert.ok(f.component.setPluginParam({ ...target, param: 'azimuth', value: 30 }).error);
  }
  f.device.name = 'Replaced Panner';
  assert.match(f.component.pluginParams({ ...f.target, names: ['azimuth'] }).error, /identity/);
  f.slot.find = () => null;
  assert.match(f.component.pluginParams({ ...f.target, names: ['azimuth'] }).error, /no instantiated/);
  assert.deepEqual(f.writes, []);
});
