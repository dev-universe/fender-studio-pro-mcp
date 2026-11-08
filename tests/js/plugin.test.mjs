import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

function pluginHarness({ className = 'Mojito', instanceName = 'Mojito2', missingBypass = false, wrapperOnly = false, missingDevice = false, mappedMixFx = false } = {}) {
  const writes = [];
  const channelWrites = [];
  const editorRequests = [];
  const parameter = (name, value, min, max) => ({
    name, value, min, max,
    getNormalized() { return (this.value - this.min) / (this.max - this.min); },
    setValue(value, notify) { writes.push({ name, method: 'raw', value, notify }); this.value = value; },
    setNormalized(value, notify) { writes.push({ name, method: 'normalized', value, notify }); this.value = this.min + value * (this.max - this.min); },
  });
  const gain = parameter('gain', 50, 0, 100);
  const bypass = parameter('bypass', 0, 0, 1);
  const device = { findParameter: (name) => !wrapperOnly && name === 'gain' ? gain : null };
  const slotComponent = {
    find: (name) => name === 'Device' && !missingDevice ? device : null,
    findParameter: (name) => name === 'bypass' && !missingBypass ? bypass : (wrapperOnly && name === 'gain' ? gain : null),
  };
  const slot = {
    component: slotComponent,
    isConnected: () => true,
    getParamValue: (name) => name === 'deviceName' ? instanceName : null,
    getPlugInClassName: () => className,
    matchesPlugInClass: (expected) => expected === 'class-id',
    invokeMethod: (...args) => { editorRequests.push(args); },
  };
  const slots = { getElement: (index) => index === 0 ? slot : null };
  const channel = {
    isConnected: () => true,
    getParamValue: (name) => name === 'label' ? 'Scratch channel' : null,
    setParamValue: (name, value) => channelWrites.push({ name, value }),
    find: (name) => name === 'inserts' || name === 'noteInserts' || (mappedMixFx && name === 'mixfx') ? slots : null,
  };
  const channels = { getElement: (index) => index === 0 ? channel : null };
  const music = { find: (name) => name === 'instrument' ? slots : (name === 'noteChannel' ? channels : null) };
  const mixer = { find: (name) => name === 'channels' ? channels : null };
  const root = { find: (name) => name === 'music' ? music : (name === 'mixer' ? mixer : null) };
  const scope = vm.createContext({
    include_file() {},
    PreSonus: {
      ControlSurfaceComponent: class {},
      ParamID: { kLabel: 'label', kInsertName: 'deviceName', kInsertBypass: 'rackBypass' },
    },
  });
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope, { filename: 'BridgeComponent.js' });
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.hostComponent = { model: { root } };
  component.bridge = { tracks: () => [{ name: 'Keys', mediaType: 'Music', selected: true }] };
  const target = { kind: 'instrument', track: 'Keys', plugin: className, param: 'gain' };
  const insert = { kind: 'effect', channel: 'Scratch channel', slot: 0, plugin: className };
  return { component, gain, bypass, writes, channelWrites, target, insert, slot, editorRequests };
}

test('an instrument class selects Mojito2 and returns changed and restored values', () => {
  const host = pluginHarness();
  const changed = host.component.setPluginParam({ ...host.target, value: 60 });
  assert.equal(changed.error, undefined);
  assert.equal(changed.plugin, 'Mojito');
  assert.equal(changed.before.value, 50);
  assert.equal(changed.after.value, 60);
  assert.equal(host.gain.value, 60);
  const restored = host.component.setPluginParam({ ...host.target, value: changed.before.value });
  assert.equal(restored.after.value, 50);
  assert.equal(host.gain.value, 50);
  assert.deepEqual(host.writes, [
    { name: 'gain', method: 'raw', value: 60, notify: true },
    { name: 'gain', method: 'raw', value: 50, notify: true },
  ]);
});

test('missing or mismatched expected plugin identity rejects parameter and bypass writes', () => {
  for (const plugin of [undefined, '', '  ', 'Chorder']) {
    const host = pluginHarness();
    const parameter = host.component.setPluginParam({ ...host.target, plugin, value: 60 });
    const bypass = host.component.setInsertBypass({ ...host.insert, plugin, bypassed: true });
    assert.match(parameter.error, /expected plugin|identity does not match/);
    assert.match(bypass.error, /expected plugin|identity does not match/);
    assert.equal(host.gain.value, 50);
    assert.equal(host.bypass.value, 0);
    assert.deepEqual(host.writes, []);
    assert.deepEqual(host.channelWrites, []);
  }
});

test('raw numeric values outside the parameter range reject without calling a setter', () => {
  const host = pluginHarness();
  for (const value of [-1, 101, NaN, Infinity, -Infinity, '50']) {
    const result = host.component.setPluginParam({ ...host.target, value });
    assert.match(result.error, /outside the supported range/);
  }
  assert.equal(host.gain.value, 50);
  assert.deepEqual(host.writes, []);
});

test('normalized values outside zero through one reject without calling a setter', () => {
  const host = pluginHarness();
  for (const normalized of [-0.1, 1.1, Infinity, -Infinity, '0.5']) {
    const result = host.component.setPluginParam({ ...host.target, normalized });
    assert.match(result.error, /normalized must be 0\.\.1/);
  }
  assert.equal(host.gain.value, 50);
  assert.deepEqual(host.writes, []);
});

test('native wrapper parameters remain readable and writable when Device has no parameter', () => {
  const host = pluginHarness({ wrapperOnly: true });
  const read = host.component.pluginParams({ ...host.target, names: ['gain', 'bypass', 'missing'] });
  assert.equal(read.error, undefined);
  assert.deepEqual(Array.from(read.params, (parameter) => parameter.name), ['gain', 'bypass']);
  assert.deepEqual(Array.from(read.missing), ['missing']);
  const result = host.component.setPluginParam({ ...host.target, param: 'bypass', normalized: 1 });
  assert.equal(result.before.value, 0);
  assert.equal(result.after.value, 1);
  assert.equal(host.bypass.value, 1);
  assert.deepEqual(host.writes, [{ name: 'bypass', method: 'normalized', value: 1, notify: true }]);
});

test('an instrument with parameters directly on its component needs no Device child', () => {
  const host = pluginHarness({ wrapperOnly: true, missingDevice: true });
  const result = host.component.setPluginParam({ ...host.target, value: 25 });
  assert.equal(result.error, undefined);
  assert.equal(result.after.value, 25);
  assert.equal(host.gain.value, 25);
});

test('slot bypass changes and restores the native component bypass parameter', () => {
  const host = pluginHarness({ className: 'Compressor', instanceName: 'Compressor2' });
  const changed = host.component.setInsertBypass({ ...host.insert, bypassed: true });
  assert.equal(changed.error, undefined);
  assert.equal(changed.plugin, 'Compressor');
  assert.equal(changed.before, false);
  assert.equal(changed.after, true);
  assert.equal(host.bypass.value, 1);
  assert.equal(host.component.inserts({ channel: 'Scratch channel' })[0].inserts[0].bypassed, true);
  const restored = host.component.setInsertBypass({ ...host.insert, bypassed: false });
  assert.equal(restored.before, true);
  assert.equal(restored.after, false);
  assert.equal(host.bypass.value, 0);
  assert.deepEqual(host.writes, [
    { name: 'bypass', method: 'raw', value: 1, notify: true },
    { name: 'bypass', method: 'raw', value: 0, notify: true },
  ]);
  assert.deepEqual(host.channelWrites, []);
});

test('an unavailable native slot bypass reports an error and an unknown read value', () => {
  const host = pluginHarness({ missingBypass: true });
  assert.equal(host.component.inserts({ channel: 'Scratch channel' })[0].inserts[0].bypassed, null);
  const result = host.component.setInsertBypass({ ...host.insert, bypassed: true });
  assert.match(result.error, /slot bypass is not available/);
  assert.equal(result.after, undefined);
  assert.deepEqual(host.writes, []);
  assert.deepEqual(host.channelWrites, []);
});

test('ordinary effect targets read and change the expected insert class', () => {
  const host = pluginHarness({ className: 'Compressor', instanceName: 'Compressor2' });
  const read = host.component.pluginParams({ ...host.insert, names: ['gain'] });
  assert.equal(read.error, undefined);
  assert.equal(read.plugin, 'Compressor');
  assert.equal(read.params[0].value, 50);
  const changed = host.component.setPluginParam({ ...host.insert, param: 'gain', normalized: 0.25 });
  assert.equal(changed.after.value, 25);
  assert.equal(host.gain.value, 25);
});

for (const kind of ['mix_fx', 'panner']) {
  test(`unmapped ${kind} targets reject reads and writes without changing an insert`, () => {
    const host = pluginHarness();
    const target = { ...host.insert, kind, param: 'gain', value: 60 };
    assert.match(host.component.pluginParams({ ...target, names: ['gain'] }).error, /no verified live parameter path|no plug-in/);
    assert.match(host.component.setPluginParam(target).error, /no verified live parameter path|no plug-in/);
    assert.match(host.component.setInsertBypass({ ...target, bypassed: true }).error, /no verified live parameter path|no plug-in/);
    assert.equal(host.gain.value, 50);
    assert.equal(host.bypass.value, 0);
    assert.deepEqual(host.writes, []);
    assert.deepEqual(host.channelWrites, []);
  });
}

for (const [kind, context] of [['instrument', 'Instrument'], ['note_effect', 'Note FX']]) {
  test(`${kind} editor opening uses the native focus context and reports an unverified request`, () => {
    const host = pluginHarness();
    const result = host.component.pluginEditorOpen({ ...host.target, kind, slot: 0 });
    assert.equal(result.error, undefined);
    assert.equal(result.plugin, 'Mojito');
    assert.equal(result.requested, true);
    assert.equal(result.verified, false);
    assert.equal(result.visible, undefined);
    assert.deepEqual(host.editorRequests, [['openEditor', context, false]]);
  });
}

for (const kind of [undefined, 'effect', 'mix_fx']) {
  test(`${kind || 'default effect'} editor opening invokes only the finite native open method`, () => {
    const host = pluginHarness({ mappedMixFx: true });
    const result = host.component.pluginEditorOpen({ ...host.insert, kind });
    assert.equal(result.error, undefined);
    assert.equal(result.requested, true);
    assert.equal(result.verified, false);
    assert.deepEqual(host.editorRequests, [['openEditor']]);
  });
}

test('editor opening refuses missing and mismatched plugin identity before native dispatch', () => {
  for (const plugin of [undefined, '', ' ', 'Chorder']) {
    const host = pluginHarness();
    const result = host.component.pluginEditorOpen({ ...host.target, plugin });
    assert.match(result.error, /expected plugin|identity does not match/);
    assert.deepEqual(host.editorRequests, []);
  }
});

test('editor opening reports unavailable native method without pretending a request was made', () => {
  const host = pluginHarness();
  host.slot.invokeMethod = undefined;
  const result = host.component.pluginEditorOpen(host.target);
  assert.match(result.error, /editor is not available/);
  assert.equal(result.requested, undefined);
  assert.deepEqual(host.editorRequests, []);
});

test('the bridge requires a current document token before forwarding editor opening', () => {
  const host = pluginHarness();
  const bridge = bridgeHarness();
  bridge.bridge.component = host.component;
  const refused = bridge.request({ op: 'pluginEditorOpen', args: host.insert });
  assert.equal(refused.ok, false);
  assert.equal(refused.code, 'context_required');
  assert.deepEqual(host.editorRequests, []);
  const requested = bridge.writeRequest({ op: 'pluginEditorOpen', args: host.insert });
  assert.equal(requested.ok, true);
  assert.equal(requested.result.requested, true);
  assert.equal(requested.result.verified, false);
  assert.deepEqual(host.editorRequests, [['openEditor']]);
});

function pannerHarness({ missing = [], missingPanner = false } = {}) {
  const host = pluginHarness();
  const pannerWrites = [];
  const nativeFinds = [];
  const parameterLookups = [];
  const params = new Map(Object.entries({ pannerType: 0, stereoMode: 0, panX1: 0.5, panX2: 0.5, panWidth: 0.75 }).map(([name, value]) => [name, {
    name, value, min: 0, max: name === 'pannerType' ? 2 : 1,
    getNormalized() { return this.value / this.max; },
    setValue(value, notify) { pannerWrites.push({ name, value, notify }); this.value = value; },
  }]));
  const panner = { findParameter: (name) => {
    parameterLookups.push(name);
    return missing.includes(name) ? null : params.get(name);
  } };
  const nativeChannel = { find: (name) => {
    nativeFinds.push(name);
    return name === 'Panner' && !missingPanner ? panner : null;
  } };
  host.component.bridge.mixerChannel = (name) => name === 'Scratch channel'
    ? nativeChannel : { bridgeError: 'no unique native channel named ' + name };
  return { ...host, params, pannerWrites, nativeFinds, parameterLookups };
}

test('standard panner reads only the five named native channel parameters', () => {
  const host = pannerHarness();
  const result = host.component.panner({ channel: 'Scratch channel' });
  const expected = ['pannerType', 'stereoMode', 'panX1', 'panX2', 'panWidth'];
  assert.equal(result.error, undefined);
  assert.equal(result.channel, 'Scratch channel');
  assert.deepEqual(Array.from(result.params, (parameter) => parameter.name), expected);
  assert.deepEqual(Array.from(result.missing), []);
  assert.deepEqual(host.parameterLookups, expected);
  assert.deepEqual(host.nativeFinds, ['Panner']);
  assert.deepEqual(host.writes, []);
  assert.deepEqual(host.pannerWrites, []);
});

test('standard panner writes and restores the native value without changing inserts', () => {
  const host = pannerHarness();
  const changed = host.component.setPanner({ channel: 'Scratch channel', param: 'panWidth', value: 0.25 });
  assert.equal(changed.before.value, 0.75);
  assert.equal(changed.before.min, 0);
  assert.equal(changed.before.max, 1);
  assert.equal(changed.after.value, 0.25);
  assert.equal(host.params.get('panWidth').value, 0.25);
  const restored = host.component.setPanner({ channel: 'Scratch channel', param: 'panWidth', value: changed.before.value });
  assert.equal(restored.after.value, 0.75);
  assert.equal(host.params.get('panWidth').value, 0.75);
  assert.deepEqual(host.pannerWrites, [
    { name: 'panWidth', value: 0.25, notify: true },
    { name: 'panWidth', value: 0.75, notify: true },
  ]);
  assert.deepEqual(host.writes, []);
  assert.deepEqual(host.channelWrites, []);
  assert.equal(host.gain.value, 50);
  assert.equal(host.bypass.value, 0);
});

test('unsupported standard panner names and raw values reject before a native write', () => {
  const host = pannerHarness();
  for (const param of ['gain', 'Panner/panWidth', 'arbitrary']) {
    assert.match(host.component.setPanner({ channel: 'Scratch channel', param, value: 0.5 }).error, /unsupported standard panner parameter/);
  }
  for (const value of [-0.1, 1.1, NaN, Infinity, '0.5']) {
    assert.match(host.component.setPanner({ channel: 'Scratch channel', param: 'panWidth', value }).error, /finite raw panner value|outside the supported range/);
  }
  assert.equal(host.params.get('panWidth').value, 0.75);
  assert.deepEqual(host.pannerWrites, []);
  assert.deepEqual(host.writes, []);
});

test('missing standard panner parameters remain explicit and cannot be written', () => {
  const host = pannerHarness({ missing: ['panWidth'] });
  const read = host.component.panner({ channel: 'Scratch channel' });
  assert.equal(read.params.length, 4);
  assert.deepEqual(Array.from(read.missing), ['panWidth']);
  const write = host.component.setPanner({ channel: 'Scratch channel', param: 'panWidth', value: 0.5 });
  assert.match(write.error, /not writable/);
  assert.equal(write.after, undefined);
  assert.deepEqual(host.pannerWrites, []);
});

test('a missing native panner or nonunique channel never falls back to an insert', () => {
  const missing = pannerHarness({ missingPanner: true });
  assert.match(missing.component.panner({ channel: 'Scratch channel' }).error, /panner is not available/);
  assert.match(missing.component.setPanner({ channel: 'Scratch channel', param: 'panWidth', value: 0.5 }).error, /panner is not available/);
  assert.deepEqual(missing.writes, []);
  assert.deepEqual(missing.pannerWrites, []);
  const wrong = pannerHarness();
  assert.match(wrong.component.panner({ channel: 'Other' }).error, /no unique native channel/);
  assert.match(wrong.component.setPanner({ channel: 'Other', param: 'panWidth', value: 0.5 }).error, /no unique native channel/);
  assert.deepEqual(wrong.nativeFinds, []);
  assert.deepEqual(wrong.writes, []);
  assert.deepEqual(wrong.pannerWrites, []);
});

test('unavailable native panner ranges or setters reject without changing the parameter', () => {
  for (const change of [
    (parameter) => { parameter.min = undefined; },
    (parameter) => { parameter.max = NaN; },
    (parameter) => { parameter.min = 2; },
    (parameter) => { parameter.setValue = undefined; },
  ]) {
    const host = pannerHarness();
    change(host.params.get('panWidth'));
    const result = host.component.setPanner({ channel: 'Scratch channel', param: 'panWidth', value: 0.5 });
    assert.match(result.error, /not writable|no supported native range/);
    assert.equal(host.params.get('panWidth').value, 0.75);
    assert.deepEqual(host.pannerWrites, []);
  }
});

test('standard panner reads need no document token and writes require the current document', () => {
  const host = pannerHarness();
  const bridge = bridgeHarness();
  bridge.bridge.component = host.component;
  const read = bridge.request({ op: 'panner', args: { channel: 'Scratch channel' } });
  assert.equal(read.ok, true);
  assert.equal(read.result.params.length, 5);
  const args = { channel: 'Scratch channel', param: 'panWidth', value: 0.5 };
  assert.equal(bridge.request({ op: 'setPanner', args }).code, 'context_required');
  assert.deepEqual(host.pannerWrites, []);
  const write = bridge.writeRequest({ op: 'setPanner', args });
  assert.equal(write.ok, true);
  assert.equal(write.result.before.value, 0.75);
  assert.equal(write.result.after.value, 0.5);
  assert.equal(host.params.get('panWidth').value, 0.5);
});
