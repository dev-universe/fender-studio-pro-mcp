import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';

const instance = { object_uid: '12345678-1234-1234-1234-123456789ABC', class_uid: '87654321-4321-4321-4321-CBA987654321' };
const target = { kind: 'instrument', track: 'FSP Verify Keys', plugin: 'Expected', source: 'dsp', expected_instance: instance.object_uid, include_context: true };

function harness() {
  const host = bridgeHarness();
  const calls = [], writes = [];
  const track = { name: target.track, mediaType: 'Music', createIterator: () => ({ next: () => null }) };
  const state = {
    tracks: [track], selected: [track], switches: { start: 0, record: false },
    focused: { label: target.track, mute: 1, record: 0, monitor: false },
    native: { ok: true, instance, source: 'dsp', reported_count: 1 },
    onNative: null, onCapabilities: null, onCleanup: null,
  };
  const list = {
    get numTracks() { return state.tracks.length; },
    get numSelectedTracks() { return state.selected.length; },
    getTrack: index => state.tracks[index], getSelectedTrack: index => state.selected[index],
  };
  const transport = { findParameter: name => Object.hasOwn(state.switches, name) ? { value: state.switches[name] } : null };
  const originalLookup = host.Host.Objects.getObjectByUrl;
  host.Host.Objects.getObjectByUrl = url => url.endsWith('/Environment/TransportPanel') ? transport
    : url.endsWith('/TrackList') ? { mainTrackList: list } : originalLookup(url);
  host.Host.Objects.getObjectByName = name => name === 'FenderMcpNative' ? {
    capabilities() { state.onCapabilities?.(); return '{"ok":true,"api":"fender-mcp-native-v1"}'; },
    request(device, body) {
      calls.push(JSON.parse(body));
      state.onNative?.();
      try { return JSON.stringify(state.native); }
      finally { state.onCleanup?.(); }
    },
  } : null;
  const channel = { isConnected: () => true, getParamValue: name => state.focused[name],
    setParamValue: (...args) => writes.push(args) };
  const root = { find: name => name === 'music' ? { find: name => name === 'noteChannel'
    ? { getElement: index => index === 0 ? channel : null } : null } : null };
  const scope = vm.createContext({ Host: host.Host, include_file() {}, memberTypes() { return {}; },
    PreSonus: { ControlSurfaceComponent: class {}, ParamID: { kLabel: 'label', kRecord: 'record' } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), scope);
  vm.runInContext(asset('bridge/BridgeComponent.js'), scope);
  const component = vm.runInContext('createBridgeComponent()', scope);
  component.hostComponent = { model: { root } };
  component.bridge = host.bridge;
  component.pluginOf = () => ({ name: target.plugin, track: target.track, device: { findParameter() { return null; } } });
  host.bridge.component = component;
  const request = (extra = {}) => {
    const context = host.current();
    return host.request({ op: 'pluginInspect', expectedSession: context.session, expectedDocument: context.document,
      args: { ...target, ...extra } });
  };
  return { ...host, component, track, list, state, calls, writes, requestContext: request };
}

test('Music context brackets unchanged native inspection and copies complete typed observations', () => {
  for (const kind of ['instrument', 'note_effect']) {
    const h = harness();
    const result = h.requestContext({ kind, slot: kind === 'instrument' ? 0 : 15 });
    assert.equal(result.ok, true);
    const context = result.result.fixture_context;
    assert.equal(context.version, 1); assert.equal(context.atomic, false);
    assert.equal(context.scope, 'music_fixture_before_after');
    assert.deepEqual(context.before, context.after);
    assert.deepEqual(context.before.target_track, { exact_match_count: 1, name: target.track, mediaType: 'Music', eventCount: 0, events: [], events_complete: true });
    assert.deepEqual(context.before.selection, { complete: true, tracks: [{ name: target.track, mediaType: 'Music' }] });
    assert.deepEqual(context.before.focused_track, { label: target.track, mute: 1, recordArmed: 0, monitor: false });
    assert.equal(context.after.context.document, result.document);
    assert.equal(context.after.context.session, result.session);
    assert.deepEqual(h.calls, [{ operation: 'inspect', source: 'dsp', expected_instance: instance.object_uid }]);
    assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
  }
});

test('default and false retain the old inspection without context reads', () => {
  const h = bridgeHarness();
  let calls = 0;
  h.bridge.component = { pluginInspect() { calls++; return { old: true }; } };
  for (const args of [{}, { include_context: false }]) {
    const result = h.request({ op: 'pluginInspect', args });
    assert.equal(result.ok, true); assert.deepEqual(result.result, { old: true });
  }
  assert.equal(calls, 2);
  assert.equal(h.request({ op: 'pluginParams', args: { include_context: false } }).ok, false);
});

test('context refuses malformed options and unsupported targets before native dispatch', () => {
  for (const extra of [{ include_context: null }, { include_context: 'true' }, { include_context: 1 },
    { kind: 'effect' }, { kind: 'panner' }, { track: '' }, { track: 'x'.repeat(513) }, { track: 'Keys\n' },
    { plugin: '' }, { source: 'auto' }, { source: undefined }, { expected_instance: undefined },
    { expected_instance: 'stale' }, { expected_instance: '00000000-0000-0000-0000-000000000000' },
    { include_nodes: true }, { node_scope: 'target' }, { slot: 1 }, { slot: false },
    { kind: 'note_effect', slot: 16 }, { kind: 'note_effect', slot: 0.5 }]) {
    const h = harness();
    assert.equal(h.requestContext(extra).ok, false, JSON.stringify(extra));
    assert.equal(h.calls.length, 0); assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
  }
});

test('unavailable or malformed transport, track, event and focused APIs never certify a fixture', () => {
  const changes = [
    h => { delete h.state.switches.start; }, h => { h.state.switches.record = null; },
    h => { h.state.switches.start = '0'; }, h => { h.state.switches.start = 2; },
    h => { h.list.getSelectedTrack = undefined; },
    h => { Object.defineProperty(h.list, 'numSelectedTracks', { value: null }); },
    h => { Object.defineProperty(h.list, 'numTracks', { value: 0.5 }); },
    h => { Object.defineProperty(h.list, 'numTracks', { value: Infinity }); },
    h => { h.list.getTrack = () => null; },
    h => { h.track.createIterator = undefined; }, h => { h.track.createIterator = () => null; },
    h => { h.track.createIterator = () => ({}); },
    h => { h.track.createIterator = () => ({ next: () => undefined }); },
    h => { h.track.createIterator = () => ({ next: () => false }); },
    h => { h.track.createIterator = () => ({ next: () => ({ name: 'event' }) }); },
    h => { h.state.focused.mute = undefined; }, h => { h.state.focused.monitor = NaN; },
    h => { h.state.focused.record = '0'; }, h => { h.state.focused.label = 'Other'; },
    h => { h.component.hostComponent = null; },
  ];
  for (const change of changes) {
    const h = harness(); change(h);
    assert.equal(h.requestContext().ok, false, change.toString());
    assert.equal(h.calls.length, 0); assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
  }
});

test('track names and complete selection preserve exact uniqueness including additional Audio tracks', () => {
  for (const change of [
    h => { h.state.tracks.push({ ...h.track }); },
    h => { h.track.name = target.track.toLowerCase(); },
    h => { h.track.name += ' suffix'; },
    h => { h.track.mediaType = 'Audio'; },
    h => { const audio = { name: 'Extra', mediaType: 'Audio' }; h.state.tracks.push(audio); h.state.selected.push(audio); },
    h => { h.state.selected = [{ ...h.track }]; },
    h => { h.state.selected = []; },
  ]) {
    const h = harness(); change(h);
    assert.equal(h.requestContext().ok, false); assert.equal(h.calls.length, 0);
  }
  const h = harness();
  h.state.tracks.push(h.track); h.state.selected.push(h.track);
  assert.equal(h.requestContext().ok, true);
});

test('native callbacks and synchronous cleanup invalidate changed context and fixture observations', () => {
  const changes = [
    h => { h.state.switches.start = 1; }, h => { h.state.switches.record = 1; },
    h => { h.state.focused.mute = 0; }, h => { h.state.focused.record = 1; },
    h => { h.state.focused.monitor = true; }, h => { h.state.selected = []; },
    h => { h.track.createIterator = () => ({ next: () => ({}) }); },
    h => { h.document.title = 'Changed'; },
    h => { h.manager.activeDocument = { ...h.document }; }, h => { h.manager.activeDocument = null; },
  ];
  for (const callback of ['onCapabilities', 'onNative', 'onCleanup']) {
    for (const change of changes) {
      const h = harness(); h.state[callback] = () => change(h);
      assert.equal(h.requestContext().ok, false, `${callback} ${change}`);
      assert.ok(h.calls.length <= 1); assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
    }
  }
});

test('final receipt context is checked after the post-observation', () => {
  const h = harness();
  const original = h.component.fixtureTrackState.bind(h.component);
  let samples = 0;
  h.component.fixtureTrackState = () => {
    const result = original();
    if (++samples === 2) {
      const context = h.bridge.context.bind(h.bridge);
      let reads = 0;
      h.bridge.context = () => { if (++reads === 2) h.manager.activeDocument = { ...h.document }; return context(); };
    }
    return result;
  };
  const result = h.requestContext();
  assert.equal(result.ok, false); assert.match(result.error, /receipt/);
  assert.equal(h.calls.length, 1); assert.deepEqual(h.writes, []);
});

test('native unknown, contradictory and stale identities cannot receive context proof', () => {
  for (const change of [
    n => { n.ok = false; }, n => { n.source = 'controller'; }, n => { n.error = {}; },
    n => { n.outcome = 'unknown'; }, n => { n.outcome = 'applied_verified'; },
    n => { n.outcome = null; }, n => { n.native_outcome = 'unknown'; },
    n => { n.instance = { ...instance, object_uid: instance.class_uid }; },
    n => { n.instance = { ...instance, class_uid: '' }; },
  ]) {
    const h = harness(); h.state.native = structuredClone(h.state.native); change(h.state.native);
    const result = h.requestContext(); assert.equal(result.ok, false, change.toString());
    assert.equal(result.result?.fixture_context, undefined); assert.deepEqual(h.writes, []);
  }
  const h = harness(); h.state.native.outcome = 'observed';
  assert.equal(h.requestContext().ok, true);
});

test('stale expected bridge context refuses before observation or native dispatch', () => {
  const h = harness();
  const context = h.current();
  for (const extra of [{ expectedDocument: 'stale' }, { expectedSession: 'stale' }]) {
    assert.equal(h.request({ op: 'pluginInspect', args: target, expectedSession: context.session,
      expectedDocument: context.document, ...extra }).ok, false);
  }
  assert.equal(h.calls.length, 0); assert.deepEqual(h.writes, []);
});


test('bound inspection reports the native query failure without claiming an instance change', () => {
  const h = harness();
  h.state.native = { ok: false, outcome: 'not_applied', instance,
    error: { code: 'node_inspection_failed', message: 'graph scope requires the verified AudioInsertSlot/Inserts/Combinator/Graph family' },
    extra: { mutation_capability: true } };
  const result = h.requestContext({ include_context: false });
  assert.equal(result.ok, false);
  assert.equal(result.error, 'native plugin inspection failed: graph scope requires the verified AudioInsertSlot/Inserts/Combinator/Graph family');
  assert.equal(result.result, undefined);
  assert.equal(h.calls.length, 1); assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
});

test('bound inspection distinguishes an absent or malformed native reply from a changed instance', () => {
  for (const [native, error] of [
    [null, 'native plugin inspection unavailable'],
    [undefined, 'native plugin inspection unavailable'],
    [false, 'native plugin inspection response is malformed'],
    [[], 'native plugin inspection response is malformed'],
    [{ ok: 1, instance }, 'native plugin inspection response is malformed'],
    [{ ok: true }, 'native plugin inspection response is malformed'],
    [{ ok: true, instance: { object_uid: instance.object_uid } }, 'native plugin inspection response is malformed'],
    [{ ok: true, instance, error: { message: 'contradictory success' } }, 'native plugin inspection response is malformed'],
  ]) {
    const h = harness();
    h.component.nativePluginRequest = () => native;
    const result = h.requestContext({ include_context: false });
    assert.equal(result.ok, false); assert.equal(result.error, error);
    assert.equal(result.result, undefined); assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
  }
});

test('bound inspection exposes only a bounded plain native error message', () => {
  for (const error of [null, {}, { message: 7 }, { message: { hidden: 'private' } }, { message: '' }]) {
    const h = harness(); h.state.native = { ok: false, error, private_metadata: 'do not stringify' };
    const result = h.requestContext({ include_context: false });
    assert.equal(result.ok, false); assert.equal(result.error, 'native plugin inspection failed');
    assert.equal(result.result, undefined);
  }
  const h = harness();
  h.state.native = { ok: false, error: { message: '\u0000\n\r\t\u007f\u0085' + 'x'.repeat(900) } };
  const result = h.requestContext({ include_context: false });
  assert.equal(result.ok, false);
  assert.equal(result.error, 'native plugin inspection failed: ' + 'x'.repeat(506));
  assert.equal(/[\u0000-\u001f\u007f-\u009f]/.test(result.error), false);
});

test('a successful inspection of a different UID remains refused', () => {
  const h = harness();
  h.component.nativePluginRequest = () => ({ ok: true,
    instance: { ...instance, object_uid: '99999999-9999-9999-9999-999999999999' } });
  const result = h.requestContext({ include_context: false });
  assert.equal(result.ok, false); assert.equal(result.error, 'the expected native plugin instance changed');
  assert.equal(result.result, undefined); assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
  const throughNative = harness();
  throughNative.state.native = { ok: true, instance: { ...instance, object_uid: '99999999-9999-9999-9999-999999999999' } };
  const rejected = throughNative.requestContext({ include_context: false });
  assert.equal(rejected.ok, false); assert.match(rejected.error, /native target instance changed/);
  assert.equal(throughNative.calls.length, 1); assert.deepEqual(throughNative.writes, []);
});

test('unbound inspection preserves legacy native observation and successful bound metadata', () => {
  for (const native of [null, { ok: false, error: { message: 'query failed' }, extra: { retained: true } }]) {
    const h = harness(); h.component.nativePluginRequest = () => native;
    const result = h.requestContext({ include_context: false, expected_instance: undefined });
    assert.equal(result.ok, true);
    if (native === null) assert.equal(Object.hasOwn(result.result, 'native'), false);
    else assert.deepEqual(result.result.native, native);
    assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
  }
  const h = harness(); h.state.native.extra = { retained: ['unknown', { count: 2 }] };
  const result = h.requestContext({ include_context: false });
  assert.equal(result.ok, true); assert.deepEqual(result.result.native.extra, h.state.native.extra);
  assert.deepEqual(h.calls, [{ operation: 'inspect', source: 'dsp', expected_instance: instance.object_uid }]);
  assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
});


test('bound native error prefixes preserve complete supplementary characters within 512 UTF-16 units', () => {
  for (const [message, expected] of [
    ['x'.repeat(510) + '🎸', 'x'.repeat(510) + '🎸'],
    ['x'.repeat(511) + '🎸', 'x'.repeat(511)],
    ['x'.repeat(512) + '🎸', 'x'.repeat(512)],
    ['🎸'.repeat(256), '🎸'.repeat(256)],
    ['🎸'.repeat(257), '🎸'.repeat(256)],
    ['\u0000\n\r\t\u007f\u0085' + 'x'.repeat(504) + '🎸', 'x'.repeat(504) + '🎸'],
  ]) {
    const h = harness();
    h.state.native = { ok: false, outcome: 'not_applied', error: { message } };
    const result = h.requestContext({ include_context: false });
    assert.equal(result.ok, false);
    assert.equal(result.error, 'native plugin inspection failed: ' + expected);
    assert.equal(result.error.slice('native plugin inspection failed: '.length).length <= 512, true);
    assert.equal(result.result, undefined); assert.deepEqual(h.writes, []); assert.deepEqual(h.effects, []);
  }
});
