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

function serumEventInputTask() {
  const host = editTaskHarness();
  const token = 'ndc1:0000000000000001:1';
  const first = { label: 'Event Input 1', type: 0 };
  const peer = { label: 'Event Input 16', type: 0 };
  const destination = { label: 'Event Input 16', type: 0 };
  let selected = first;
  const ports = [first, peer, destination];
  const calls = [];
  const unit = { recordArmed: false, monitorActive: false };
  const channel = {
    recordUnit: unit,
    findParameter: name => name === 'outputPort' ? { getCurrentPort: () => selected } : null,
    connectTo(port) {
      calls.push(port);
      selected = port;
      return this.receipt;
    },
    receipt: 1,
  };
  const track = { name: 'Lead', mediaType: 'Music', channel };
  const service = {
    documentContext: () => JSON.stringify({
      ok: true, api: 'fender-mcp-native-v1', context_api: 'native_document_context_v1',
      token, open: true, title: 'Scratch', fileUrl: 'file:///scratch/song.song',
    }),
    eventPortSameOwner: (_source, port) => JSON.stringify({
      ok: true, outcome: 'observed', same_owner: port === destination,
    }),
  };
  host.Host.Objects.getObjectByName = name => name === 'FenderMcpNative' ? service : null;
  host.Host.Classes = { createInstance: () => ({ setPortList() {}, getNextPort: index => ports[index] }) };
  const context = {
    mainTrackList: { numTracks: 1, getTrack: () => track },
    editor: { model: { root: { environment: { getPortList: () => ports } } } },
  };
  const run = (before = 1) => {
    host.arm([{ op: 'serumEventInputSet', track: 'Lead', before, after: 16 }], {
      lease: { nativeDocument: { context_api: 'native_document_context_v1', token } },
    });
    return host.invoke(context).results[0];
  };
  return { run, channel, calls, destination, unit, service };
}

test('only the exact same-owner destination is dispatched among duplicate Event Input labels', () => {
  const h = serumEventInputTask();
  assert.equal(h.run().native_outcome, 'applied_unverified');
  assert.deepEqual(h.calls, [h.destination]);
});

test('stale Event Input, monitoring, and unknown owner refuse before dispatch', () => {
  const h = serumEventInputTask();
  assert.equal(h.run(2).native_outcome, 'not_applied');
  h.unit.monitorActive = true;
  assert.equal(h.run().native_outcome, 'not_applied');
  h.unit.monitorActive = false;
  h.service.eventPortSameOwner = () => '{"ok":false,"outcome":"not_applied"}';
  assert.equal(h.run().native_outcome, 'not_applied');
  assert.deepEqual(h.calls, []);
});

test('a missing route switch receipt after dispatch reports unknown with one call', () => {
  const h = serumEventInputTask();
  h.channel.receipt = null;
  assert.equal(h.run().native_outcome, 'unknown');
  assert.deepEqual(h.calls, [h.destination]);
});
