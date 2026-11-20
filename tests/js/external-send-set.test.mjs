import test from 'node:test';
import assert from 'node:assert/strict';
import { bridgeHarness } from './host.mjs';

const plugin = 'External Effect Stereo';
const instance = {
    object_uid: '12345678-1234-1234-1234-123456789ABC',
    class_uid: '889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4',
    native_class: 'AudioEffect',
    object_id: plugin + " 2",
};
const target = { channel: 'Audio B', slot: 1, plugin };
const args = {
    channel: target.channel, slot: target.slot, expected_instance: instance.object_uid,
    expected_current: 'none', destination: 'main',
};

function harness() {
    const host = bridgeHarness();
    const calls = [];
    const strip = { label: target.channel, mute: 1, monitor: 0, recordArmed: 0, solo: 0, input: null, output: '메인' };
    const main = { label: '메인', volume: 0, output: null };
    const tracks = [{ name: target.channel, channel: target.channel, mediaType: 'Audio' }];
    const channels = [{ label: 'Audio A' }, strip, main];
    const state = { current: 'none', transport: {}, stable: true, editorOpen: true };
    const device = {};
    const targetPlugin = { name: plugin, device, component: {}, element: {} };
    const port = (name, tag, text) => ({
        name, tag, text, native_kind: 'List', writable: false, value: -1, normalized: 1,
        native_value: { variant_tag: 1, data: { kind: 'integer', value: -1 } },
        native_normalized: { kind: 'float32', bits_hex: '3F800000' },
    });
    const matches = query => query.channel === target.channel && query.slot === target.slot && query.plugin === plugin;
    host.bridge.song = () => ({ trackCount: 2, transport: { precount: false, preroll: false } });
    host.bridge.transportPanel = () => ({ findParameter: key => ({ value: state.transport[key] ?? 0 }) });
    host.bridge.metronome = () => ({ click: false, preCount: false, precount: false, preroll: false });
    host.bridge.tracks = () => tracks;
    host.bridge.component = {
        channels: () => channels,
        pluginOf: query => matches(query) ? targetPlugin : null,
        pluginParams: query => ({
            instance: matches(query) && state.editorOpen ? instance : null,
            parameter_discovery: { source: 'controller' },
            page_complete: true, access_metadata_complete: true, value_read_complete: true,
            complete: true, reported_count: 24,
            params: [
                port('sendPort', 102, state.current === 'main' ? '메인' : '없음'),
                port('receivePort', 103, '없음'),
                ...Array.from({ length: 22 }, (_, index) => ({ name: `param${index}` })),
            ].map((row, index) => ({ ...row, index })),
        }),
        nativePluginRequest: (selected, request) => {
            calls.push(request);
            assert.equal(selected, device);
            state.current = request.destination;
            return {
                ok: true, outcome: 'applied_unverified', operation: 'external_stereo_send_set',
                phase: 'closed_checked', raw_select_result: 1, selection_calls: 1,
                checkmark_and_display_confirmed: true, instance, destination: request.destination,
                diagnostic: {
                    version: 'stereo_send_v1', failure_phase: null, dispatch_attempted: true,
                    menu_callbacks_began: true, select_entered: true, select_returned: true, cleanup_proven: true,
                },
            };
        },
        externalPortMenu: () => ({
            ok: true, instance, held_send_route: { assigned: state.current === 'main', stable: state.stable },
            rows: [
                { title: '없음', checked: state.current === 'none' },
                { title: '메인 - 1 + 2', checked: state.current === 'main' },
                {}, {},
            ],
        }),
    };
    const set = (request = args, fields = {}) =>
        host.writeRequest({ op: 'externalStereoSendSet', args: request, ...fields });
    return { calls, strip, main, tracks, state, set, bridge: host.bridge };
}

test('Stereo Send binds the selected channel, slot and current route with one callback per transition', () => {
    const host = harness();
    const selected = host.set().result;
    assert.equal(selected.ok, true);
    assert.equal(selected.assigned, true);
    assert.equal(selected.controller_rows, 24);
    assert.equal(selected.slot, 1);
    assert.equal(selected.channel, target.channel);
    assert.equal(selected.audio_verified, false);
    assert.equal(selected.persistence_verified, false);
    const restored = host.set({ ...args, expected_current: 'main', destination: 'none' }).result;
    assert.equal(restored.ok, true);
    assert.equal(restored.assigned, false);
    assert.deepEqual(restored.after, selected.before);
    assert.deepEqual(host.calls.map(request => request.destination), ['main', 'none']);
});

test('Stereo Send refuses stale, ambiguous and unsafe targets before native dispatch', () => {
    for (const change of [
        host => { host.state.transport.start = true; },
        host => { host.state.editorOpen = false; },
        host => { host.strip.mute = 0; },
        host => { host.main.volume = 1; },
        host => { host.main.output = 'Out 1 + 2'; },
        host => { host.tracks.push({ ...host.tracks[0] }); },
        host => { host.state.current = 'main'; },
    ]) {
        const host = harness();
        change(host);
        assert.equal(host.set().result.native_outcome, 'not_applied');
        assert.equal(host.calls.length, 0);
    }
    const host = harness();
    assert.equal(host.set({ ...args, slot: 0 }).result.native_outcome, 'not_applied');
    assert.equal(host.set(args, { expectedDocument: 'expired' }).result.native_outcome, 'not_applied');
    assert.equal(host.calls.length, 0);
});

test('Studio Pro refuses an unavailable editor through a return value without throwing', () => {
    const host = harness();
    host.state.editorOpen = false;
    const result = host.set().result;
    assert.equal(result.native_outcome, 'not_applied');
    assert.equal(result.diagnostic.failure_phase, 'controller_baseline');
    assert.equal(result.diagnostic.dispatch_attempted, false);
    assert.equal(host.calls.length, 0);
    assert.doesNotMatch(host.bridge.externalStereoSendSet.toString(), /\bthrow\b/);
});

test('Stereo Send with missing post-selection route proof returns unknown without retry', () => {
    const host = harness();
    host.state.stable = false;
    const result = host.set().result;
    assert.equal(result.native_outcome, 'unknown');
    assert.equal(result.diagnostic.failure_phase, 'independent_menu');
    assert.equal(result.diagnostic.select_entered, true);
    assert.equal(host.calls.length, 1);
});
