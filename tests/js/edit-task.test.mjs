import test from 'node:test';
import assert from 'node:assert/strict';
import { editTaskHarness, mailbox, musicContext } from './host.mjs';

test('graph probe reflects only its fixed function object using the observed Device identity', () => {
  const task = editTaskHarness('track');
  const device = {}, graphFunctions = {}, calls = [];
  const uid = '12345678-1234-1234-1234-123456789ABC';
  const channel = { label: 'Fixture', mediaType: 'Audio', find(name) {
    return name === 'Inserts' ? { find(id) { return id === 'FX01' ? { find(child) { return child === 'Device' ? device : null; } } : null; } } : null;
  } };
  task.Host.Objects.getObjectByName = (name) => name === 'FenderMcpNative' ? {
    capabilities: () => JSON.stringify({ ok: true, api: 'fender-mcp-native-v1' }),
    request(...args) {
      calls.push(args);
      const request = JSON.parse(args[1]);
      return JSON.stringify({ ok: true, instance: { object_uid: uid }, ...(request.operation === 'inspect_aux'
        ? { scope: 'auxiliary_type_observation', mutation_capability: false, auxiliary_report: { nodes: [] } } : {}) });
    },
  } : null;
  const context = { functions: { root: {
    environment: { find: () => ({ getChannelList: () => ({ numChannels: 1, getChannel: () => channel }) }) },
    createFunctions: name => name === 'AudioFXGraphEditFunctions' ? graphFunctions : null,
  } } };
  const result = task.run([{ op: 'probe', channel: 'Fixture' }], context);
  assert.equal(result.results[0].graphReflection.mutation_capability, false);
  assert.equal(calls.length, 2);
  assert.equal(calls[0].length, 2);
  assert.equal(calls[1].length, 3);
  assert.equal(calls[1][0], device);
  assert.equal(calls[1][2], graphFunctions);
  assert.deepEqual(JSON.parse(calls[1][1]), { operation: 'inspect_aux', expected_instance: uid });
});

function mutation(kind) {
  if (kind === 'track') {
    const song = musicContext();
    return {
      context: song.context,
      ops: [{ op: 'createPart', track: 'Keys', start: 0, end: 8 }],
      count: () => song.keys.events.length,
    };
  }
  const note = { pitch: 60 };
  let index = 0;
  return {
    context: {
      iterator: { done: () => index > 0, next: () => { index++; return note; } },
      functions: { modifyPitch: (target, pitch) => { target.pitch = pitch; } },
    },
    ops: [{ op: 'transpose', semitones: 1 }],
    count: () => note.pitch - 60,
  };
}

for (const kind of ['track', 'notes']) {
  test(`${kind} EditTask consumes request and lease before applying it and cannot replay`, () => {
    const task = editTaskHarness(kind);
    const edit = mutation(kind);
    const id = task.arm(edit.ops);
    const original = task.read(task.requestFilename);
    const originalCreate = task.Host.IO.createTextFile;
    const writes = [];
    task.Host.IO.createTextFile = (url) => {
      writes.push(url.url);
      assert.equal(edit.count(), url.url.endsWith(task.resultFilename) ? 1 : 0);
      return originalCreate(url);
    };
    assert.equal(task.invoke(edit.context).id, id);
    assert.equal(edit.count(), 1);
    assert.deepEqual(writes.slice(0, 2), [mailbox + task.requestFilename, mailbox + 'task-lease.json']);
    assert.deepEqual(task.read(task.requestFilename), { id: null, ops: [] });
    assert.deepEqual(task.read('task-lease.json'), { id: null });
    task.Host.IO.createTextFile = originalCreate;
    assert.match(task.invoke(edit.context).error, /no edit request/);
    task.write(task.requestFilename, original);
    assert.match(task.invoke(edit.context).error, /no matching EditTask lease/);
    assert.equal(edit.count(), 1);
  });

  const rejected = [
    ['missing lease', {}, (task) => task.files.delete(mailbox + 'task-lease.json'), /matching EditTask lease/],
    ['stale request ID', { lease: { id: 'previous-request' } }, null, /matching EditTask lease/],
    ['wrong task kind', { lease: { kind: kind === 'track' ? 'notes' : 'track' } }, null, /matching EditTask lease/],
    ['expired lease', { lease: { expiresAt: Date.now() - 1 } }, null, /lease expired/],
    ['non-numeric expiry', { lease: { expiresAt: 'later' } }, null, /lease expired/],
    ['older protocol', { status: { protocol: 1 } }, null, /session is not current/],
    ['closed bridge', { status: { closed: true } }, null, /session is not current/],
    ['different bridge session', { status: { session: 'new-session' } }, null, /session is not current/],
    ['wrong requested session', { request: { expectedSession: 'old-session' } }, null, /context does not match/],
    ['wrong requested document', { request: { expectedDocument: 'another-document' } }, null, /context does not match/],
    ['different song path', {}, (task) => { task.manager.activeDocument.path.url = 'file:///other/song.song'; }, /active document does not match/],
    ['different song title', {}, (task) => { task.manager.activeDocument.title = 'Other'; }, /active document does not match/],
    ['closed document', {}, (task) => { task.manager.activeDocument = null; }, /active document does not match/],
    ['invalid document object', { lease: { title: null, fileUrl: null } }, (task) => { task.manager.activeDocument = 'unavailable'; }, /active document does not match/],
    ['unavailable document API', {}, (task) => { task.Host.Objects.getObjectByUrl = null; }, /active document is not available/],
    ['omitted title', { lease: { title: undefined } }, null, /no document identity/],
    ['omitted file URL', { lease: { fileUrl: undefined } }, null, /no document identity/],
    ['malformed operations', { request: { ops: { length: 1 } } }, null, /no edit request/],
  ];
  for (const [reason, overrides, change, expected] of rejected) {
    test(`${kind} EditTask rejects ${reason} without applying operations`, () => {
      const task = editTaskHarness(kind);
      const edit = mutation(kind);
      const id = task.arm(edit.ops, overrides);
      if (change) change(task);
      const result = task.invoke(edit.context);
      assert.equal(result.id, id);
      assert.match(result.error, expected);
      assert.equal(edit.count(), 0);
      assert.deepEqual(task.read(task.requestFilename), { id: null, ops: [] });
      assert.deepEqual(task.read('task-lease.json'), { id: null });
      assert.match(task.invoke(edit.context).error, /no edit request/);
      assert.equal(edit.count(), 0);
    });
  }

  test(`${kind} EditTask preserves explicit null identities for unsaved documents`, () => {
    const task = editTaskHarness(kind);
    const edit = mutation(kind);
    task.manager.activeDocument = {};
    const result = task.run(edit.ops, edit.context);
    assert.equal(result.error, undefined);
    assert.equal(edit.count(), 1);
  });

  test(`${kind} EditTask refuses to apply when lease consumption cannot be written`, () => {
    const task = editTaskHarness(kind);
    const edit = mutation(kind);
    task.arm(edit.ops);
    const originalCreate = task.Host.IO.createTextFile;
    task.Host.IO.createTextFile = (url) => url.url.endsWith('task-lease.json') ? null : originalCreate(url);
    assert.match(task.invoke(edit.context).error, /could not consume/);
    assert.equal(edit.count(), 0);
  });
}

test('createPart creates an empty instrument part and addNotes writes the requested part', () => {
  const task = editTaskHarness();
  const song = musicContext();
  const result = task.run([
    { op: 'createPart', track: 'Keys', start: 0, end: 8 },
    { op: 'createPart', track: 'Keys', start: 16, end: 24 },
    { op: 'addNotes', track: 'Keys', part: 2, notes: [{ pitch: 60, beat: 1, length: 0.5, velocity: 110 }] },
  ], song.context);
  assert.equal(result.results[0].added, 0);
  assert.equal(result.results[1].part.start, 16);
  assert.equal(result.results[2].added, 1);
  assert.equal(result.results[2].part.number, 2);
  assert.deepEqual(song.keys.events[0].notes, []);
  assert.deepEqual(song.keys.events[1].notes, [{ pitch: 60, velocity: 110, length: 0.5, beat: 1 }]);
  assert.deepEqual(song.drums.events, []);
});

test('createPart applies the requested name to the created native part', () => {
  const task = editTaskHarness();
  const song = musicContext();
  const result = task.run([{ op: 'createPart', track: 'Keys', start: 0, end: 4, name: 'Named part' }], song.context).results[0];
  assert.equal(result.part.name, 'Named part');
  assert.equal(song.keys.events[0].name, 'Named part');
  assert.equal(song.context.functions.executeImmediately, false);
});

test('unavailable part naming rejects before creating an unnamed part', () => {
  const task = editTaskHarness();
  const song = musicContext();
  delete song.context.functions.renameEvent;
  const result = task.run([{ op: 'createPart', track: 'Keys', start: 0, end: 4, name: 'Named part' }], song.context).results[0];
  assert.match(result.error, /part naming is not available/);
  assert.equal(song.keys.events.length, 0);
});

test('createPart accepts initial notes with beat relative to the new part', () => {
  const task = editTaskHarness();
  const song = musicContext();
  const result = task.run([
    { op: 'createPart', track: 'Keys', start: 16, end: 32, notes: [{ pitch: 36, beat: 0, length: 0.25, velocity: 100 }] },
  ], song.context).results[0];
  assert.equal(result.added, 1);
  assert.equal(result.part.start, 16);
  assert.deepEqual(song.keys.events[0].notes, [{ pitch: 36, velocity: 100, length: 0.25, beat: 0 }]);
});

test('invalid track and part ranges reject without creating a part', () => {
  const task = editTaskHarness();
  const song = musicContext();
  const result = task.run([
    { op: 'createPart', track: 'Audio', start: 0, end: 8 },
    { op: 'createPart', track: 'Keys', start: 8, end: 8 },
    { op: 'createPart', track: 'Missing', start: 0, end: 8 },
  ], song.context).results;
  assert.match(result[0].error, /not an instrument track/);
  assert.match(result[1].error, /end after start/);
  assert.match(result[2].error, /no track named/);
  assert.deepEqual(song.keys.events, []);
  assert.deepEqual(song.audio.events, []);
});

test('addNotes reports invalid notes separately from the notes it applied', () => {
  const task = editTaskHarness();
  const song = musicContext();
  const result = task.run([
    { op: 'createPart', track: 'Keys', start: 0, end: 8 },
    { op: 'addNotes', track: 'Keys', part: 1, notes: [
      { pitch: 60, beat: 0, length: 1 },
      { pitch: 62, beat: -1, length: 1 },
    ] },
    { op: 'addNotes', track: 'Keys', part: 2, notes: [{ pitch: 64, beat: 0, length: 1 }] },
  ], song.context).results;
  assert.equal(result[1].added, 1);
  assert.equal(result[1].errors.length, 1);
  assert.match(result[1].errors[0], /note 2 needs/);
  assert.match(result[2].error, /part 2 does not exist/);
  assert.equal(song.keys.events[0].notes.length, 1);
});

test('EditTask never runs arbitrary script even when a legacy config enables eval', () => {
  const task = editTaskHarness();
  const song = musicContext();
  const result = task.run([{ op: 'eval', code: 'context.functions.root.compromised = true; return 1;' }], song.context);
  assert.match(result.results[0].error, /arbitrary script execution is not supported/);
  assert.equal(song.context.functions.root.compromised, undefined);
});

test('missing music-edit functions reject before adding a part', () => {
  const task = editTaskHarness();
  const song = musicContext();
  song.context.functions.root.createFunctions = (name) => name === 'MusicPartFunctions' ? song.parts : null;
  const result = task.run([
    { op: 'createPart', track: 'Keys', start: 0, end: 8, notes: [{ pitch: 60, beat: 0, length: 1 }] },
  ], song.context).results[0];
  assert.match(result.error, /notes cannot be added here/);
  assert.deepEqual(song.keys.events, []);
});

test('duplicate event names and fractional part indices reject without changing notes', () => {
  const task = editTaskHarness();
  const song = musicContext();
  task.run([
    { op: 'createPart', track: 'Keys', start: 0, end: 8 },
    { op: 'createPart', track: 'Keys', start: 16, end: 24 },
  ], song.context);
  const notes = [{ pitch: 60, beat: 0, length: 1 }];
  const result = task.run([
    { op: 'addNotes', track: 'Keys', part: 'Keys', notes },
    { op: 'addNotes', track: 'Keys', part: 1.5, notes },
    { op: 'editEvent', track: 'Keys', event: 'Keys', to: 4 },
  ], song.context).results;
  assert.match(result[0].error, /part name is ambiguous/);
  assert.match(result[1].error, /part 1\.5 does not exist/);
  assert.match(result[2].error, /event name is ambiguous/);
  assert.deepEqual(song.keys.events.map((event) => event.notes), [[], []]);
  assert.deepEqual(song.keys.events.map((event) => event.startTime.seconds), [0, 16]);
});

test('ambiguous folder and track names reject before creating or moving tracks', () => {
  for (const tracks of [
    [{ name: 'Group', isFolder: true }, { name: 'Group', isFolder: true }, { name: 'Keys' }],
    [{ name: 'Keys' }, { name: 'Keys' }],
  ]) {
    const task = editTaskHarness();
    const effects = [];
    const context = {
      mainTrackList: { numTracks: tracks.length, getTrack: (index) => tracks[index] },
      functions: {
        addTrack: () => { effects.push('created'); return { name: 'Group', isFolder: true }; },
        moveToFolder: () => effects.push('moved'),
      },
    };
    const result = task.run([{ op: 'folder', tracks: ['Keys'], folder: 'Group', create: true }], context).results[0];
    assert.match(result.error, /name is ambiguous/);
    assert.deepEqual(effects, []);
  }
});
