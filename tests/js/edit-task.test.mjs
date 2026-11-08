import test from 'node:test';
import assert from 'node:assert/strict';
import { editTaskHarness, musicContext } from './host.mjs';

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
