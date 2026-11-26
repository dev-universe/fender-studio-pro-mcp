import test from 'node:test';
import assert from 'node:assert/strict';
import { bridgeHarness, editTaskHarness } from './host.mjs';

test('Music track output reads the retained selector label instead of its scalar or record port', () => {
  const task = editTaskHarness();
  const reads = [];
  let port = { label: 'Event Input 2', type: 0 };
  const track = {
    name: 'Lead',
    mediaType: 'Music',
    channel: {
      findParameter(name) {
        reads.push(name);
        return { value: -1, getCurrentPort: () => port };
      },
    },
  };
  const tracks = [track];
  const context = {
    mainTrackList: {
      get numTracks() { return tracks.length; },
      getTrack: index => tracks[index],
    },
  };
  const read = () => task.run([{ op: 'eventInput', track: 'Lead' }], context).results[0];

  assert.deepEqual(read(), { op: 'eventInput', track: 'Lead', event_input: 2, label: 'Event Input 2' });
  port = { label: 'Event Input 16', type: 0 };
  assert.equal(read().event_input, 16);
  assert.deepEqual(reads, ['outputPort', 'outputPort']);

  for (const invalid of [{ label: 'Event Input 17', type: 0 }, { label: 'Event Input 2', type: 1 }, null]) {
    port = invalid;
    assert.match(read().error, /does not expose a numbered Event Input/);
  }
  tracks.push({ name: 'Lead', mediaType: 'Audio' });
  assert.match(read().error, /resolve to one Music track/);
  assert.equal(reads.length, 5);
});

test('the bridge guards Event Input reads with a current native document', () => {
  const host = bridgeHarness();
  const current = host.current();
  const payload = { op: 'trackEventInput', args: { track: 'Lead' } };
  assert.equal(host.request(payload).code, 'context_required');
  assert.equal(host.request({ ...payload, expectedSession: current.session, expectedDocument: 'stale' }).code, 'document_changed');
  const valid = { ...payload, expectedSession: current.session, expectedDocument: current.document };
  assert.match(host.request(valid).error, /Native document context/);

  host.bridge.context = () => current;
  host.bridge.documentMode = 'native';
  let result = { op: 'eventInput', track: 'Lead', event_input: 2, label: 'Event Input 2' };
  host.bridge.trackTask = request => {
    assert.deepEqual(JSON.parse(JSON.stringify(request)), { ops: [{ op: 'eventInput', track: 'Lead' }] });
    return { results: [result] };
  };
  assert.deepEqual(host.request(valid).result, { track: 'Lead', event_input: 2, label: 'Event Input 2' });
  result = { ...result, track: 'Other' };
  assert.match(host.request(valid).error, /did not confirm an Event Input/);
});
