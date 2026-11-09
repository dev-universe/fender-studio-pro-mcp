import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset } from './host.mjs';

function splitterHarness() {
  const writes = [], paths = [], lookups = [];
  const param = (name, value, min, max) => ({ name, value, min, max,
    setValue(next, notify) { writes.push({ name, value: next, notify }); this.value = next; },
  });
  const params = { numOutputs: param('numOutputs', 2, 2, 5), splitMode: param('splitMode', 0, 0, 2) };
  for (let i = 1; i <= 5; i++) {
    params[`gain[${i}]`] = param(`gain[${i}]`, 1, 0, 3.1622777);
    params[`mute[${i}]`] = param(`mute[${i}]`, 0, 0, 1);
    if (i < 5) params[`splitFreq[${i}]`] = param(`splitFreq[${i}]`, 240, 24, 16000);
  }
  const node = { name: 'Node01', title: 'Splitter1', findParameter(name) { lookups.push(name); return params[name] || null; } };
  const other = { name: 'Node02', title: 'Pro EQ', findParameter: () => null };
  const graph = { numChildren: 2, 'children[0]': node, 'children[1]': other };
  const channel = { find(path) { paths.push(path); return path === 'Inserts/Combinator/Graph' ? graph : null; } };
  const owner = {
    bridge: { mixerChannel: (label) => label === 'Scratch' ? channel : { bridgeError: 'channel not unique' } },
    paramInfo: (p) => ({ name: p.name, value: p.value, min: p.min, max: p.max, text: null, normalized: (p.value - p.min) / (p.max - p.min) }),
  };
  const scope = vm.createContext({});
  vm.runInContext(asset('bridge/NativeControls.js'), scope);
  const NativeControls = vm.runInContext('NativeControls', scope);
  const controls = new NativeControls(owner);
  const target = { channel: 'Scratch', node: 'Node01', label: 'Splitter1' };
  return { controls, owner, graph, node, other, params, paths, writes, lookups, target };
}

test('Splitter reads classify both native parameters and list only active branch controls', () => {
  const host = splitterHarness();
  const result = host.controls.splitters({ channel: 'Scratch', path: 'ignored' });
  assert.equal(result.error, undefined);
  assert.equal(result.scope, 'top_level');
  assert.equal(result.splitters.length, 1);
  const splitter = result.splitters[0];
  assert.equal(splitter.node, 'Node01');
  assert.equal(splitter.label, 'Splitter1');
  assert.equal(splitter.type, 'Splitter');
  assert.deepEqual(Array.from(splitter.params, (p) => p.name), ['numOutputs', 'splitMode', 'gain[1]', 'mute[1]', 'gain[2]', 'mute[2]', 'splitFreq[1]']);
  assert.deepEqual(host.paths, ['Inserts/Combinator/Graph']);
  assert.deepEqual(host.writes, []);
  host.other.findParameter = (name) => name === 'numOutputs' ? host.params.numOutputs : null;
  assert.equal(host.controls.splitters({ channel: 'Scratch' }).splitters.length, 1);
});

test('Splitter gain change reads back and restores its original native value', () => {
  const host = splitterHarness();
  const changed = host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 });
  assert.equal(changed.error, undefined);
  assert.equal(changed.before.value, 1);
  assert.equal(changed.after.value, 0.5);
  assert.equal(changed.verified, true);
  const restored = host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: changed.before.value });
  assert.equal(restored.after.value, 1);
  assert.equal(restored.verified, true);
  assert.deepEqual(host.writes, [{ name: 'gain[1]', value: 0.5, notify: true }, { name: 'gain[1]', value: 1, notify: true }]);
});

test('missing, ambiguous and stale Splitter identity rejects before mutation', () => {
  for (const override of [{ node: undefined }, { label: undefined }, { label: '' }, { node: 'stale' }, { label: 'Splitter2' }, { channel: 'missing' }]) {
    const host = splitterHarness();
    const result = host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5, ...override });
    assert.equal(typeof result.error, 'string');
    assert.deepEqual(host.writes, []);
  }
  const host = splitterHarness();
  host.other.name = host.node.name;
  assert.match(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).error, /ambiguous/);
  assert.deepEqual(host.writes, []);
  host.other.name = 'Node02';
  delete host.params.splitMode;
  assert.match(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).error, /not a verified Splitter/);
  assert.deepEqual(host.writes, []);
});

test('Splitter verification agrees with Rust for native booleans and numeric tolerance', () => {
  const host = splitterHarness();
  host.params['mute[1]'].setValue = function () { this.value = true; };
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'mute[1]', value: 1 }).verified, true);
  host.params['gain[1]'].setValue = function () { this.value = 0.5000012; };
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).verified, true);
  host.params['gain[1]'].setValue = function () { this.value = 0.500002; };
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).verified, false);
});

test('Splitter guards finite values, known names, integer controls and active branches', () => {
  const cases = [
    ['gain[1]', NaN], ['gain[1]', Infinity], ['gain[1]', -Infinity], ['gain[1]', '0.5'],
    ['gain[1]', -1], ['gain[1]', 3.2], ['splitFreq[1]', 23], ['splitFreq[1]', 16001],
    ['numOutputs', 2.5], ['numOutputs', 1], ['numOutputs', 6], ['splitMode', 1.5], ['splitMode', 3],
    ['mute[1]', 0.5], ['mute[1]', 2], ['gain[3]', 1], ['mute[3]', 1], ['splitFreq[2]', 240],
    ['gain[6]', 1], ['mute[0]', 0], ['splitFreq[5]', 240], ['Device/gain[1]', 1], ['__proto__', 1],
  ];
  for (const [param, value] of cases) {
    const host = splitterHarness();
    const result = host.controls.setSplitter({ ...host.target, param, value });
    assert.equal(typeof result.error, 'string', `${param}=${String(value)}`);
    assert.deepEqual(host.writes, []);
  }
});

test('Splitter count and mode changes expose new active branches and allow restore', () => {
  const host = splitterHarness();
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'numOutputs', value: 5 }).after.value, 5);
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'splitMode', value: 2 }).after.value, 2);
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'mute[5]', value: 1 }).after.value, 1);
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'splitFreq[4]', value: 1000 }).after.value, 1000);
  const read = host.controls.splitters({ channel: 'Scratch' }).splitters[0];
  assert.equal(read.params.length, 16);
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'numOutputs', value: 2 }).after.value, 2);
  assert.equal(host.controls.setSplitter({ ...host.target, param: 'splitMode', value: 0 }).after.value, 0);
});

test('incomplete or oversized graph reads are explicit and never authorize a write', () => {
  const host = splitterHarness();
  host.graph.numChildren = 129;
  const read = host.controls.splitters({ channel: 'Scratch' });
  assert.equal(read.truncated, true);
  assert.equal(read.diagnostics.length, 126);
  assert.match(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).error, /complete bounded/);
  host.graph.numChildren = 2;
  delete host.graph['children[1]'];
  assert.match(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).error, /complete bounded/);
  host.graph.numChildren = Infinity;
  assert.match(host.controls.splitters({ channel: 'Scratch' }).error, /child count/);
  assert.deepEqual(host.writes, []);
});

test('missing native ranges, setters and failed readback do not pretend verified writes', () => {
  const host = splitterHarness();
  host.params['gain[1]'].max = undefined;
  assert.match(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).error, /native.*range/);
  host.params['gain[1]'].max = 3.1622777;
  host.params['gain[1]'].setValue = undefined;
  assert.match(host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 }).error, /setter/);
  assert.deepEqual(host.writes, []);
  host.params['gain[1]'].setValue = () => {};
  const unchanged = host.controls.setSplitter({ ...host.target, param: 'gain[1]', value: 0.5 });
  assert.equal(unchanged.verified, false);
  assert.equal(unchanged.after.value, 1);
});
