import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { asset } from './host.mjs';

const expected = '12345678-1234-1234-1234-123456789ABC';
const request = { operation: 'scope_sidechain_assignment_get', expected_instance: expected };
const sendId = '34567812-3456-2345-1234-123456789ABD';
const remove = { ...request, operation: 'scope_sidechain_assignment_remove', expected_send: sendId };
const instance = { object_uid: expected, class_uid: '36322412-AD90-4EE0-B63C-37929566B081', native_class: 'AudioEffect' };

function harness() {
  const calls = [];
  const active = { open: true, document: 'scratch:1', session: 'scratch' };
  let playerStopped = true;
  let capability = 'authored_scope_assignment_v1';
  let removalCapability = 'recorded_scope_send_remove_v1';
  let removalReply;
  let onRequest = () => {};
  let replyInstance = instance;
  const service = {
    capabilities: () => JSON.stringify({ ok: true, api: 'fender-mcp-native-v1',
      scope_sidechain_assignment_api: capability, scope_sidechain_assignment_remove_api: removalCapability }),
    request(...args) {
      onRequest();
      calls.push(args);
      if (JSON.parse(args[1]).operation === remove.operation) {
        return JSON.stringify(removalReply ?? { ok: true, operation: remove.operation,
          outcome: 'applied_unverified', recorded: true, removed_send: sendId, instance: replyInstance });
      }
      return JSON.stringify({ ok: true, operation: request.operation, outcome: 'observed',
        scope: 'authored_native_assignment', state: 'assigned_to_exact_scope', instance: replyInstance });
    },
  };
  const context = vm.createContext({ Host: { Objects: { getObjectByName: () => service } } });
  vm.runInContext(asset('bridge/NativePluginControls.js'), context);
  const Controls = vm.runInContext('NativePluginControls', context);
  const controls = new Controls({ bridge: {
    requestContext: active, context: () => ({ ...active }), insertGraphTransportStopped: () => playerStopped,
  } });
  return { controls, calls,
    setRemovalCapability: next => { removalCapability = next; }, setRemovalReply: next => { removalReply = next; },
    setCapability: next => { capability = next; },
    setInstance: next => { replyInstance = next; }, startPlayback: () => { playerStopped = false; },
    changeDocumentOnRequest: () => { onRequest = () => { controls.component.bridge.context = () => ({
      ...active, document: 'scratch:2',
    }); }; } };
}

test('Scope assignment sends the exact source as the only auxiliary native object', () => {
  const h = harness();
  const device = {};
  const source = {};
  assert.equal(h.controls.request(device, request, source).state, 'assigned_to_exact_scope');
  assert.deepEqual(h.calls, [[device, JSON.stringify(request), source]]);
});

test('Scope assignment refuses a missing source or unavailable operation without dispatch', () => {
  const h = harness();
  assert.equal(h.controls.request({}, request).outcome, 'not_applied');
  h.setCapability(undefined);
  assert.equal(h.controls.request({}, request, {}).outcome, 'not_applied');
  h.setCapability('authored_scope_assignment_v1');
  h.startPlayback();
  assert.equal(h.controls.request({}, request, {}).outcome, 'not_applied');
  assert.equal(h.calls.length, 0);
});

test('Scope assignment suppresses results from a different instance or document', () => {
  const h = harness();
  h.setInstance({ ...instance, object_uid: '12345678-1234-1234-1234-123456789ACC' });
  assert.equal(h.controls.request({}, request, {}).outcome, 'not_applied');
  h.setInstance(instance);
  h.changeDocumentOnRequest();
  assert.equal(h.controls.request({}, request, {}).outcome, 'not_applied');
  assert.equal(h.calls.length, 2);
});

test('Scope removal needs its own capability and a canonical Send identity', () => {
  const h = harness();
  const target = {};
  const source = {};
  h.setRemovalCapability(undefined);
  assert.equal(h.controls.request(target, remove, source).outcome, 'not_applied');
  h.setRemovalCapability('recorded_scope_send_remove_v1');
  assert.equal(h.controls.request(target, { ...remove, expected_send: sendId.toLowerCase() }, source).outcome, 'not_applied');
  assert.equal(h.calls.length, 0);
  const result = h.controls.request(target, remove, source);
  assert.equal(result.outcome, 'applied_unverified');
  assert.equal(result.removed_send, sendId);
  assert.deepEqual(h.calls, [[target, JSON.stringify(remove), source]]);
});

test('Scope removal preserves firm refusal and treats a lost lease as unknown', () => {
  const h = harness();
  h.setRemovalReply({ ok: false, outcome: 'not_applied', error: { code: 'wrong_send', message: 'wrong Send' } });
  assert.equal(h.controls.request({}, remove, {}).outcome, 'not_applied');
  h.setRemovalReply(undefined);
  h.changeDocumentOnRequest();
  assert.equal(h.controls.request({}, remove, {}).outcome, 'unknown');
  assert.equal(h.calls.length, 2);
});
