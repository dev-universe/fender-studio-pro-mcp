import test from 'node:test';
import assert from 'node:assert/strict';
import { bridgeHarness, editTaskHarness } from './host.mjs';

function fixture(kind = 'effect') {
  const task = editTaskHarness();
  const category = kind === 'effect' ? 'AudioEffect' : 'AudioSynth';
  const name = 'Shared name';
  const ids = ['{5D172B6A-4841-49AA-8B14-5A8998465C38}', '{F8A2BFA1-2C5B-482C-AEA3-3925E25D8A29}'];
  const classes = ids.map(id => ({ toString: () => id }));
  const rows = classes.map(cls => ({ cls, name }));
  const menuState = { min: 0, max: () => rows.length - 1, select: index => rows[index] };
  task.Host.Classes = {
    createInstance(type) {
      assert.equal(type, 'Host:PlugInMenuParam');
      let row;
      return {
        setCategory(value) { assert.equal(value, category); },
        get min() { return menuState.min; },
        get max() { return menuState.max(); },
        setValue(index) { row = menuState.select(index); },
        get string() { return row?.name; },
        getSelectedClass() { return row?.cls; },
      };
    },
  };
  task.Host.Engine = { TrackFormats: { findEqual: () => ({}) } };
  const mutations = [];
  const track = { name: 'Synth', channel: { label: 'Synth' } };
  const inserts = {}, synths = {};
  const slot = {};
  const mixer = { getChannelList: () => ({
    numChannels: 1,
    getChannel: () => ({ label: 'Scratch', mediaType: 'Audio', find: key => key === 'Inserts' ? inserts : null }),
  }) };
  const context = {
    mainTrackList: { getInsertPosition: () => 0 },
    functions: {
      root: {
        environment: { find: key => ({ MixerConsole: mixer, Synths: synths })[key] },
        createFunctions(key) {
          if (key === 'DeviceEditFunctions') return { insertDevice(folder, cls) { mutations.push({ folder, cls }); return slot; } };
          if (key === 'MusicPartFunctions') return { connectTrackWithInstrument(t, s) { mutations.push({ track: t, slot: s }); return true; } };
          return null;
        },
      },
      addMediaTrack(index, name) { mutations.push({ index, name }); return track; },
    },
  };
  const op = kind === 'effect' ? { op: 'addPlugin', channel: 'Scratch', plugin: name } : { op: 'addInstrumentTrack', instrument: name };
  return {
    task, category, ids, classes, rows, menuState, context, op, mutations, inserts, synths, track, slot,
    run(edits = {}) { return task.run([{ ...op, ...edits }], context).results[0]; },
  };
}

test('addPlugin uses the exact class object behind a catalog ID shared by repeated menu aliases', () => {
  const h = fixture();
  h.rows.push({ ...h.rows[1] });
  const bridge = bridgeHarness();
  bridge.Host.Classes = h.task.Host.Classes;
  const catalog = bridge.request({ op: 'pluginCatalog', args: { category: h.category } }).result;
  assert.equal(catalog.complete, true);
  assert.equal(catalog.plugins.length, 2);
  const selected = catalog.plugins[1];
  const result = h.run({ plugin: selected.name, classId: selected.classId });
  assert.deepEqual(result, { op: 'addPlugin', channel: 'Scratch', added: selected.name, classId: h.ids[1] });
  assert.deepEqual(h.mutations, [{ folder: h.inserts, cls: h.classes[1] }]);
  assert.equal(h.mutations[0].cls, h.classes[1]);
});

test('name-only adds keep the original first matching class and result shape', () => {
  const h = fixture();
  const result = h.run({ plugin: 'shared name' });
  assert.deepEqual(result, { op: 'addPlugin', channel: 'Scratch', added: 'Shared name' });
  assert.equal(h.mutations.length, 1);
  assert.equal(h.mutations[0].cls, h.classes[0]);
});

test('explicit selection refuses stale and malformed IDs or mismatched display names', () => {
  for (const edits of [
    { classId: '{stale}' }, { classId: null }, { classId: '' }, { classId: 42 },
    { classId: 'id\u0085' }, { classId: 'id\u3000' }, { classId: 'é'.repeat(129) },
    { classId: '{F8A2BFA1-2C5B-482C-AEA3-3925E25D8A29}', plugin: 'shared name' },
  ]) {
    const h = fixture();
    assert.equal(typeof h.run(edits).error, 'string', JSON.stringify(edits));
    assert.deepEqual(h.mutations, []);
  }
});

test('explicit selection refuses an incomplete or unreadable menu and a changed final selection', () => {
  for (const alter of [
    h => { h.menuState.max = () => 8192; },
    h => { h.menuState.min = Infinity; },
    h => { h.menuState.max = () => 1.5; },
    h => { h.menuState.select = () => { throw new Error('unreadable'); }; },
    h => { let visits = 0; h.menuState.select = index => h.rows[visits++ < 2 ? index : 0]; },
  ]) {
    const h = fixture();
    alter(h);
    assert.equal(typeof h.run({ classId: h.ids[1] }).error, 'string');
    assert.deepEqual(h.mutations, []);
  }
});

test('explicit instruments resolve their class before allocating or connecting a track', () => {
  const h = fixture('instrument');
  assert.equal(typeof h.run({ classId: '{stale}' }).error, 'string');
  assert.deepEqual(h.mutations, []);
  const result = h.run({ classId: h.ids[1] });
  assert.deepEqual(result, { op: 'addInstrumentTrack', track: 'Synth', instrument: 'Shared name', connected: true, channel: 'Synth', classId: h.ids[1] });
  assert.deepEqual(h.mutations, [
    { index: 0, name: 'Shared name' }, { folder: h.synths, cls: h.classes[1] }, { track: h.track, slot: h.slot },
  ]);
  assert.equal(h.mutations[1].cls, h.classes[1]);
});

test('explicit class selection retains the EditTask document guard', () => {
  const h = fixture();
  h.task.arm([{ ...h.op, classId: h.ids[1] }], { request: { expectedDocument: 'another-document' } });
  assert.match(h.task.invoke(h.context).error, /context does not match/);
  assert.deepEqual(h.mutations, []);
});
