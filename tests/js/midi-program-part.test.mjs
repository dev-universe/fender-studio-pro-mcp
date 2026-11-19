import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness, mailbox, musicContext } from './host.mjs';

const fixture = 'mcp-midi-program-12345678-1234-4000-8000-123456789abc.mid';
const partName = fixture.replace('.mid', ' CH1');
const instance = '12345678-1234-1234-1234-123456789ABC';
const requestArgs = {
  track: 'Keys', expected_instance: instance, fixture, bank_msb: 0, bank_lsb: 0, program: 5,
};

function harness() {
  const host = bridgeHarness();
  const song = musicContext();
  const state = {
    selected: null, recordArmed: false, switches: { start: 0, record: 0, loop: 0, primaryTime: 0 },
    classUid: '91AF218E-111A-44CB-ABFE-F7CB065FF157', noteRegion: null, numeric: 0, onImport() {},
  };
  const iterator = list => {
    let index = 0;
    return { done: () => index >= list.length, next: () => list[index++] ?? null };
  };
  const scope = vm.createContext({ Host: host.Host, McpEditConfig: { mailbox } });
  scope.include_file = name => {
    if (name !== 'McpEditConfig.js') vm.runInContext(asset(`edit-tasks/package/${name}`), scope);
  };
  vm.runInContext(asset('edit-tasks/package/McpTrackEdit.js'), scope);
  vm.runInContext(asset('edit-tasks/package/McpEdit.js'), scope);
  const trackTask = vm.runInContext('createMcpTrackEdit()', scope);
  const noteTask = vm.runInContext('createMcpEdit()', scope);
  host.files.set(mailbox + fixture, 'private fixture');
  const list = {
    numTracks: 3, numSelectedTracks: 1,
    getTrack: index => [song.keys, song.drums, song.audio][index],
    getSelectedTrack: index => index === 0 ? song.keys : null,
  };
  const originalLookup = host.Host.Objects.getObjectByUrl;
  const transport = { findParameter: name => ({ value: state.switches[name] }) };
  host.Host.Objects.getObjectByUrl = url => url.endsWith('/Environment/TransportPanel') ? transport
    : url.endsWith('/TrackList') ? { mainTrackList: list } : originalLookup(url);
  host.Host.Objects.getObjectByName = name => name === 'FenderMcpNative' ? {
    documentContext() {
      return JSON.stringify({
        ok: true, api: 'fender-mcp-native-v1', context_api: 'native_document_context_v1',
        token: 'ndc1:0000000000000001:1', open: true, title: 'Scratch', fileUrl: 'file:///scratch/song.song',
      });
    },
  } : null;
  host.bridge.component = {
    fixtureTrackState: () => ({ label: 'Keys', mute: 0, recordArmed: state.recordArmed, monitor: false }),
    pluginInspect: () => ({ plugin: 'Twin 3', native: { ok: true, source: 'dsp',
      instance: { object_uid: instance, class_uid: state.classUid, native_class: 'AudioSynth' } } }),
  };
  song.context.editor = { createSelectFunctions: () => ({
    select(event) { assert.equal(this.selectExclusive, true); state.selected = event; },
  }) };
  const edits = [];
  song.context.functions.importFile = (file, time, track, flags) => {
    assert.equal(file.url, mailbox + fixture);
    assert.equal(time.seconds, 0);
    assert.equal(track, song.keys);
    assert.equal(flags, 0);
    const part = {
      name: partName, startTime: { seconds: 0 }, endTime: { seconds: 1.281 }, isMuted: false,
      notes: [], createSequenceIterator() { return iterator(this.notes); },
    };
    part.notes.push({
      pitch: 60, velocity: 1 / 127, startTime: { time: 2.5, musical: 2.5, seconds: 1.25 },
      endTime: { seconds: 1.281 }, region: state.noteRegion ?? part,
    });
    track.events.push(part);
    edits.push('import');
    state.onImport(part);
    return state.numeric;
  };
  const handlers = {
    'Track/Fender MCP Track Edit': () => trackTask.performEdit(song.context),
    'Musical Functions/Fender MCP Edit': () => noteTask.performEdit({
      iterator: iterator(state.selected.notes),
      functions: { deleteEvent(note) { state.selected.notes.splice(state.selected.notes.indexOf(note), 1); edits.push('delete'); } },
    }),
    'Edit/Deselect All': () => { state.selected = null; },
  };
  host.Host.GUI.Commands.findCommand = (category, command) => handlers[`${category}/${command}`] ?? null;
  host.Host.GUI.Commands.interpretCommand = (category, command) => {
    const execute = handlers[`${category}/${command}`];
    assert.ok(execute, `unexpected command ${category}/${command}`);
    execute();
    return true;
  };
  const scopeIdentity = host.current();
  const request = (extra = {}, op = 'midiProgramPartAdd') => host.request({
    op, expectedSession: scopeIdentity.session, expectedDocument: scopeIdentity.document,
    args: op === 'midiProgramPartAdd' ? { ...requestArgs, ...extra } : extra,
  });
  return { ...host, request, state, song, edits, list };
}

test('the note-free part requires independent witnesses beyond the numeric importer return', () => {
  const h = harness();
  const response = h.request();
  assert.equal(response.ok, true);
  assert.deepEqual(h.edits, ['import', 'delete']);
  assert.equal(h.song.keys.events[0].notes.length, 0);
  assert.deepEqual(response.result.part, { name: partName, start: 0, end: 1.281, note_count: 0 });
  assert.equal(response.result.stage, 'complete');
  assert.equal(response.result.import_return_type, 'number');
  assert.equal(response.result.atomic, false);
  assert.equal(response.result.program_events_verified, false);
  assert.equal(response.result.delivery_verified, false);
  assert.equal(h.state.selected, null);
});

test('preflight rejects wrong or nonempty targets, untrusted paths and raw task dispatch', () => {
  for (const alter of [
    h => { h.state.switches.start = 1; }, h => { h.state.switches.loop = 1; },
    h => { h.state.recordArmed = true; }, h => { h.state.classUid = 'wrong class'; },
    h => { h.song.keys.events.push({}); }, h => { h.list.numSelectedTracks = 0; },
  ]) {
    const h = harness();
    alter(h);
    assert.equal(h.request().result.native_outcome, 'not_applied');
    assert.deepEqual(h.edits, []);
  }
  const h = harness();
  for (const invalid of [{ fixture: '../fixture.mid' }, { fixture: fixture + '\n' }, { bank_lsb: 128 }, { program: 1.5 }]) {
    assert.equal(h.request(invalid).result.native_outcome, 'not_applied');
  }
  assert.equal(h.request({ ops: [{ op: 'importMidiProgramFixture', track: 'Keys', fixture }] }, 'trackTask').ok, false);
  assert.deepEqual(h.edits, []);
});

test('post-write mismatches report unknown and leave unverified notes untouched', () => {
  for (const [alter, stage] of [
    [h => { h.state.numeric = undefined; }, 'import'],
    [h => { h.state.onImport = () => { h.state.classUid = 'changed route'; }; }, 'seed_verification'],
    [h => { h.state.onImport = part => { part.notes.push({ pitch: 64 }); }; }, 'seed_verification'],
    [h => { h.state.noteRegion = { name: 'some other part' }; }, 'note_cleanup'],
  ]) {
    const h = harness();
    alter(h);
    const response = h.request();
    assert.equal(response.result.native_outcome, 'unknown');
    assert.equal(response.result.stage, stage);
    assert.deepEqual(h.edits, ['import']);
    assert.ok(h.song.keys.events[0].notes.length > 0);
  }
});
