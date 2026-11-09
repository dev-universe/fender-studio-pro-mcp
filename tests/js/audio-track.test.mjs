import test from 'node:test';
import assert from 'node:assert/strict';
import { editTaskHarness } from './host.mjs';

function audioTrackHarness() {
  const task = editTaskHarness();
  const tracks = [{ name: 'Existing', mediaType: 'Audio' }];
  const format = { name: '5.1', mediaType: 'Audio' };
  const formatLookups = [];
  const calls = [];
  task.Host.Engine = { TrackFormats: {
    findEqual(name) { formatLookups.push(name); return name === '5.1' ? format : null; },
  } };
  const functions = {
    executeImmediately: false,
    root: {
      createIterator() {
        let index = 0;
        return { done: () => index >= tracks.length, next: () => tracks[index++] };
      },
    },
    addMediaTrack(index, name, chosenFormat) {
      calls.push({ index, name, format: chosenFormat, immediate: this.executeImmediately });
      const track = { name, mediaType: 'Audio' };
      Object.defineProperty(track, 'channel', { get() { throw new Error('channel has not settled'); } });
      tracks.push(track);
      return track;
    },
  };
  const context = { functions, mainTrackList: { getInsertPosition: () => tracks.length } };
  const op = { op: 'addAudioTrack', name: 'Surround scratch', format: '5.1' };
  return { task, tracks, format, formatLookups, calls, functions, context, op };
}

test('audio track creation passes the exact named format and reports the created track before its channel settles', () => {
  const h = audioTrackHarness();
  const result = h.task.run([h.op], h.context).results[0];
  assert.deepEqual(h.formatLookups, ['5.1']);
  assert.equal(h.calls.length, 1);
  assert.equal(h.calls[0].format, h.format);
  assert.deepEqual(h.calls[0], { index: 1, name: 'Surround scratch', format: h.format, immediate: true });
  assert.deepEqual(result, { op: 'addAudioTrack', created: true, track: 'Surround scratch', format: '5.1' });
  assert.equal(h.functions.executeImmediately, false);
});

test('unknown audio format refuses creation without falling back to the master or stereo format', () => {
  const h = audioTrackHarness();
  const result = h.task.run([{ ...h.op, format: 'Unknown format' }], h.context).results[0];
  assert.match(result.error, /no track format named/);
  assert.deepEqual(h.formatLookups, ['Unknown format']);
  assert.equal(h.calls.length, 0);
  assert.equal(h.tracks.length, 1);
});

test('a known non-audio format refuses creation', () => {
  const h = audioTrackHarness();
  h.format.mediaType = 'Music';
  const result = h.task.run([h.op], h.context).results[0];
  assert.match(result.error, /not audio/);
  assert.equal(h.calls.length, 0);
});

test('an existing track name refuses creation before any mutation', () => {
  const h = audioTrackHarness();
  const result = h.task.run([{ ...h.op, name: 'Existing' }], h.context).results[0];
  assert.match(result.error, /already has the requested name/);
  assert.equal(h.calls.length, 0);
  assert.equal(h.functions.executeImmediately, false);
});

test('duplicate names created earlier in the same batch refuse a second creation', () => {
  const h = audioTrackHarness();
  const results = h.task.run([h.op, h.op], h.context).results;
  assert.equal(results[0].created, true);
  assert.match(results[1].error, /already has the requested name/);
  assert.equal(h.calls.length, 1);
});

test('an unreadable track list refuses creation because duplicate names cannot be checked', () => {
  const h = audioTrackHarness();
  h.functions.root.createIterator = () => null;
  const result = h.task.run([h.op], h.context).results[0];
  assert.match(result.error, /check duplicate names/);
  assert.equal(h.calls.length, 0);
});

test('invalid names and formats refuse before native format lookup or creation', () => {
  for (const field of ['name', 'format']) {
    for (const value of ['', '   ', 'bad\0value', 'x'.repeat(257), null, 42]) {
      const h = audioTrackHarness();
      const result = h.task.run([{ ...h.op, [field]: value }], h.context).results[0];
      assert.match(result.error, /required/);
      assert.deepEqual(h.formatLookups, []);
      assert.equal(h.calls.length, 0);
    }
  }
});

test('an invalid insertion position refuses creation', () => {
  for (const position of [-1, 0.5, Infinity, '1']) {
    const h = audioTrackHarness();
    h.context.mainTrackList.getInsertPosition = () => position;
    const result = h.task.run([h.op], h.context).results[0];
    assert.match(result.error, /insertion position/);
    assert.equal(h.calls.length, 0);
  }
});

test('audio track creation restores the execution mode and preserves an uncertain host outcome', () => {
  const h = audioTrackHarness();
  h.functions.executeImmediately = true;
  h.functions.addMediaTrack = () => { throw new Error('native error'); };
  const result = h.task.run([h.op], h.context).results[0];
  assert.equal(result.outcome, 'unknown');
  assert.match(result.error, /inspect the song before retrying/);
  assert.equal(h.functions.executeImmediately, true);
});

test('audio track creation uses the existing document and session guard', () => {
  const h = audioTrackHarness();
  h.task.arm([h.op], { request: { expectedDocument: 'another-document' } });
  const result = h.task.invoke(h.context);
  assert.match(result.error, /context does not match/);
  assert.deepEqual(h.formatLookups, []);
  assert.equal(h.calls.length, 0);
});
