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
    selected: [], recordArmed: false, switches: { start: 0, record: 0, loop: 0, primaryTime: 0 },
    classUid: '91AF218E-111A-44CB-ABFE-F7CB065FF157', noteRegion: null, numeric: 0, integerNoteDone: false,
    document: { token: 'ndc1:0000000000000001:1', title: 'Scratch', fileUrl: 'file:///scratch/song.song' },
    nextDocument: null,
    onImport() {}, onSelect() {}, onInspect() {}, onTrackTask() {},
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
      const document = state.nextDocument ?? state.document;
      state.nextDocument = null;
      return JSON.stringify({
        ok: true, api: 'fender-mcp-native-v1', context_api: 'native_document_context_v1',
        open: true, ...document,
      });
    },
  } : null;
  host.bridge.component = {
    fixtureTrackState: () => ({ label: 'Keys', mute: 0, recordArmed: state.recordArmed, monitor: false }),
    pluginInspect() {
      state.onInspect();
      return { plugin: 'Twin 3', native: { ok: true, source: 'dsp',
        instance: { object_uid: instance, class_uid: state.classUid, native_class: 'AudioSynth' } } };
    },
  };
  const edits = [], selectionActions = [];
  song.context.editor = {
    selection: {
      isEmpty: () => state.selected.length === 0 ? 1 : 0,
      isMultiple: () => state.selected.length > 1 ? 1 : 0,
      isSelected: event => state.selected.includes(event) ? 1 : 0,
    },
    createSelectFunctions: () => ({
      select(event) {
        assert.equal(this.selectExclusive, false);
        state.selected.push(event);
        selectionActions.push('select');
        state.onSelect();
      },
    }),
    unselect(event) {
      assert.equal(event, song.keys.events[0]);
      state.selected = state.selected.filter(selected => selected !== event);
      selectionActions.push('unselect');
    },
  };
  song.context.functions.importFile = (file, time, track, flags) => {
    assert.equal(file.url, mailbox + fixture);
    assert.equal(time.seconds, 0);
    assert.equal(track, song.keys);
    assert.equal(flags, 0);
    const part = {
      name: partName, startTime: { seconds: 0 }, endTime: { seconds: 1.281 }, isMuted: false,
      notes: [], createSequenceIterator() {
        const sequence = iterator(this.notes);
        return state.integerNoteDone ? { next: sequence.next, done: () => Number(sequence.done()) } : sequence;
      },
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
    'Track/Fender MCP Track Edit': () => {
      state.onTrackTask(host.read('track-edit-request.json').ops[0]);
      return trackTask.performEdit(song.context);
    },
    'Musical Functions/Fender MCP Edit': () => noteTask.performEdit({
      iterator: iterator(state.selected.flatMap(event => event.notes)),
      functions: { deleteEvent(note) { note.region.notes.splice(note.region.notes.indexOf(note), 1); edits.push('delete'); } },
    }),
    'Edit/Deselect All': () => { state.selected = []; selectionActions.push('global_deselect'); },
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
  return { ...host, request, state, song, edits, list, selectionActions };
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
  assert.deepEqual(h.state.selected, []);
  assert.deepEqual(h.selectionActions, ['select', 'unselect']);
});

test('track events use exact null and note sequences admit bounded integer exhaustion', () => {
  for (const done of [undefined, () => 1]) {
    const h = harness();
    h.song.keys.createIterator = () => {
      let index = 0;
      return { next: () => h.song.keys.events[index++] ?? null, done };
    };
    h.state.integerNoteDone = true;
    const response = h.request();
    assert.equal(response.result.stage, 'complete');
    assert.deepEqual(h.edits, ['import', 'delete']);
    assert.deepEqual(h.selectionActions, ['select', 'unselect']);
  }
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
  assert.equal(h.request({ ops: [{ op: 'midiProgramSelection', track: 'Keys', partName, action: 'release' }] }, 'trackTask').ok, false);
  assert.deepEqual(h.edits, []);
});

test('an existing same-document event selection is preserved before import', () => {
  const h = harness();
  const peer = { name: 'Other event', notes: [] };
  h.state.selected = [peer];
  const response = h.request();
  assert.equal(response.result.native_outcome, 'not_applied');
  assert.equal(response.result.detail_code, 'selection_not_empty');
  assert.match(response.result.reason, /empty arrangement selection/);
  assert.deepEqual(h.edits, []);
  assert.deepEqual(h.selectionActions, []);
  assert.deepEqual(h.state.selected, [peer]);
});

test('an unobservable arrangement selection prevents import', () => {
  const h = harness();
  h.song.context.editor.selection.isEmpty = () => undefined;
  const response = h.request();
  assert.equal(response.result.native_outcome, 'not_applied');
  assert.equal(response.result.detail_code, 'selection_unavailable');
  assert.deepEqual(h.edits, []);
  assert.deepEqual(h.selectionActions, []);
});

test('import prechecks distinguish bounded failure codes while edits remain unverified', () => {
  for (const [detail, alter] of [
    ['track_iterator_exhaustion_unknown', h => {
      h.state.onTrackTask = op => {
        if (op.op === 'importMidiProgramFixture') h.song.keys.createIterator = () => ({ next: () => undefined, done: () => true });
      };
    }],
    ['track_has_events', h => {
      h.state.onTrackTask = op => {
        if (op.op === 'importMidiProgramFixture') h.song.keys.events.push({ name: 'Private peer' });
      };
    }],
    ['fixture_url_unavailable', h => {
      const originalUrl = h.Host.Url;
      h.Host.Url = value => value.endsWith(fixture) ? null : originalUrl(value);
    }],
    ['fixture_file_unavailable', h => { h.files.delete(mailbox + fixture); }],
    ['import_time_unavailable', h => { h.song.context.functions.newMediaTime = () => null; }],
    ['import_native_lease_changed', h => {
      h.song.context.editor.selection.isEmpty = () => {
        h.state.nextDocument = { ...h.state.document, fileUrl: 'file:///scratch/private-alternate.song' };
        return true;
      };
    }],
  ]) {
    const h = harness();
    alter(h);
    const response = h.request();
    assert.deepEqual(response.result, { native_outcome: 'unknown', stage: 'import', detail_code: detail });
    assert.deepEqual(h.edits, []);
    assert.deepEqual(h.selectionActions, []);
  }
});

test('document switch during select or route check preserves the new document selection', () => {
  for (const [phase, update] of [
    ['onSelect', { token: 'ndc1:0000000000000001:2', title: 'Other', fileUrl: 'file:///scratch/other.song' }],
    ['onInspect', { token: 'ndc1:0000000000000001:2', title: 'Other', fileUrl: 'file:///scratch/other.song' }],
    ['onInspect', { fileUrl: 'file:///scratch/renamed.song' }],
  ]) {
    const h = harness();
    const sentinel = { name: 'Sentinel selection', notes: [] };
    h.state[phase] = () => {
      if (!h.state.selected.some(event => event.name === partName)) return;
      Object.assign(h.state.document, update);
      h.state.selected = [sentinel];
    };
    const response = h.request();
    assert.equal(response.result.native_outcome, 'unknown', phase);
    assert.deepEqual(h.edits, ['import'], phase);
    assert.deepEqual(h.selectionActions, ['select'], phase);
    assert.deepEqual(h.state.selected, [sentinel], phase);
    assert.equal(h.song.keys.events[0].notes.length, 1, phase);
  }
});

test('replacement of the issued selection within the same document prevents a release or delete', () => {
  const h = harness();
  const sentinel = { name: 'Peer selection', notes: [] };
  h.state.onInspect = () => {
    if (h.state.selected.some(event => event.name === partName)) h.state.selected = [sentinel];
  };
  const response = h.request();
  assert.equal(response.result.native_outcome, 'unknown');
  assert.deepEqual(h.edits, ['import']);
  assert.deepEqual(h.selectionActions, ['select']);
  assert.deepEqual(h.state.selected, [sentinel]);
  assert.equal(h.song.keys.events[0].notes.length, 1);
});

test('post-write mismatches report unknown and leave unverified notes untouched', () => {
  for (const [alter, stage] of [
    [h => { h.state.numeric = undefined; }, 'import'],
    [h => { h.state.onImport = () => { h.state.classUid = 'changed route'; }; }, 'seed_verification'],
    [h => { h.state.onImport = part => { part.notes.push({ pitch: 64 }); }; }, 'seed_verification'],
    [h => { h.state.onImport = part => { part.createSequenceIterator = () => ({ next: () => part.notes[0], done: () => '' }); }; }, 'seed_verification'],
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
