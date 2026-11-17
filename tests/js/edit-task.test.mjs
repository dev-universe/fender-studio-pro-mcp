import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, editTaskHarness, mailbox, memoryHost, musicContext } from './host.mjs';

function nativeReadHelper(helper, stage, patch, omitOutcome = false) {
  const { Host } = memoryHost();
  const device = {}, auxiliary = {}, calls = [];
  const uid = '12345678-1234-1234-1234-123456789ABC';
  const inserts = { find: name => name === 'FX01' ? { find: child => child === 'Device' ? device : null } : null };
  const channel = { find: name => name === 'Inserts' ? inserts : null };
  Host.Objects.getObjectByName = name => name === 'FenderMcpNative' ? {
    // Capabilities has its own contract and intentionally has no read outcome,
    // node scope, instance or ancestry predicate.
    capabilities: () => JSON.stringify({ ok: true, api: 'fender-mcp-native-v1' }),
    request(...args) {
      calls.push(args);
      const request = JSON.parse(args[1]);
      const identity = request.operation === 'inspect';
      const response = { ok: true, instance: { object_uid: uid }, ...(identity ? {} : helper === 'legacy' ? {
        outcome: 'observed', scope: 'auxiliary_type_observation', mutation_capability: false, auxiliary_report: { nodes: [] },
      } : {
        outcome: 'observed', scope: 'insert_folder_ancestry_observation', mutation_capability: false, atomic_snapshot: false,
        ancestor_matches_active_model: true, insert_folder_ancestry_report: {
          traversal: 'parent_chain_only', ancestor_matches_active_model: true,
        },
      }) };
      if ((stage === 'identity') === identity) {
        Object.assign(response, patch);
        if (omitOutcome) delete response.outcome;
      }
      return JSON.stringify(response);
    },
  } : null;
  const context = vm.createContext({ Host, McpEditConfig: { mailbox }, channel, auxiliary });
  context.include_file = name => {
    if (name !== 'McpEditConfig.js') vm.runInContext(asset(`edit-tasks/package/${name}`), context, { filename: name });
  };
  vm.runInContext(asset('edit-tasks/package/McpTrackEdit.js'), context, { filename: 'McpTrackEdit.js' });
  const result = vm.runInContext(helper === 'legacy'
    ? 'mteGraphReflection(channel, auxiliary)' : 'mteInsertFolderAncestry(channel)', context);
  return { result, calls, device, auxiliary: helper === 'legacy' ? auxiliary : inserts };
}

function presetPreflightHelper({ identity = {}, result = {}, capability = {}, absentOutcome = false } = {}) {
  const { Host } = memoryHost();
  const calls = [], device = {}, instance = {
    object_uid: '12345678-1234-1234-1234-123456789ABC',
    class_uid: '12345678-1234-1234-1234-123456789ABD', native_class: 'AudioEffectDevice', object_id: 'Device',
  };
  const inserts = { find: name => name === 'FX01' ? { find: name => name === 'Device' ? device : null } : null };
  const channel = { find: name => name === 'Inserts' ? inserts : null };
  const environment = { status: 'observed', ancestor_matches: true, scope: 'fixed_active_document_controller_environment', mutation_capability: false, atomic_snapshot: false };
  const observed = { ok: true, outcome: 'observed', instance,
    scope: 'insert_folder_preset_preflight_observation', status: 'observed', mutation_capability: false, atomic_snapshot: false,
    anchor_slot_bound: true, folder: { id: 'Inserts', native_class: 'AudioInsertFolder', object_uid: '12345678-1234-1234-1234-123456789ABE', class_uid: null },
    document_environment_observation: environment, target_flags: 3,
    metadata: { attribute_count: 3, class_name: 'FX Chains', class_vendor: 'Fender', class_category: null } };
  Host.Objects.getObjectByName = () => ({
    capabilities: () => JSON.stringify({ ok: true, api: 'fender-mcp-native-v1', insert_folder_preset_preflight_observation: true, ...capability }),
    request(...args) {
      calls.push(args);
      const first = JSON.parse(args[1]).operation === 'inspect';
      const value = first ? { ok: true, instance, ...identity } : { ...observed, ...result };
      if (absentOutcome && !first) delete value.outcome;
      return JSON.stringify(value);
    },
  });
  const context = vm.createContext({ Host, McpEditConfig: { mailbox }, channel });
  context.include_file = name => { if (name !== 'McpEditConfig.js') vm.runInContext(asset(`edit-tasks/package/${name}`), context); };
  vm.runInContext(asset('edit-tasks/package/McpTrackEdit.js'), context);
  return { result: vm.runInContext('mteInsertFolderPresetPreflight(channel)', context), calls, device, inserts };
}

test('folder preset preflight observes only the fixed bound read-only route', () => {
  const { result, calls, device, inserts } = presetPreflightHelper();
  assert.equal(result.status, 'observed');
  assert.equal(result.target_flags, 3);
  assert.equal(result.folder.class_uid, null);
  assert.equal(result.document_environment_observation.ancestor_matches, true);
  assert.equal(result.mutation_capability, false);
  assert.equal(calls.length, 2);
  assert.equal(calls[1][0], device);
  assert.equal(calls[1][2], inserts);
  assert.equal(JSON.parse(calls[1][1]).operation, 'inspect_insert_folder_preset_preflight');
});

test('folder preset preflight distinguishes absent capability and unavailable backend without fallback', () => {
  for (const flag of [false, null, undefined]) {
    const { result, calls } = presetPreflightHelper({ capability: { insert_folder_preset_preflight_observation: flag } });
    assert.equal(result.status, 'unavailable');
    assert.equal(result.target_flags, undefined);
    assert.equal(calls.length, 0);
  }
  const { result, calls } = presetPreflightHelper({ result: {
    status: 'unavailable', reason: 'metadata unavailable', anchor_slot_bound: undefined, folder: undefined,
    document_environment_observation: undefined, target_flags: undefined, metadata: undefined,
  } });
  assert.equal(result.status, 'unavailable');
  assert.equal(result.folder, undefined);
  assert.equal(calls.length, 2);
});

test('folder preset preflight refuses unknown read evidence and never makes a second call after uncertain identity', () => {
  for (const field of ['outcome', 'native_outcome']) {
    for (const value of ['unknown', 'applied_verified', 'not_applied', null, {}, true]) {
      for (const stage of ['identity', 'result']) {
        const { result, calls } = presetPreflightHelper({ [stage]: { [field]: value } });
        assert.equal(typeof result.error, 'string');
        assert.equal(result.folder, undefined);
        assert.equal(calls.length, stage === 'identity' ? 1 : 2);
      }
    }
  }
  assert.equal(typeof presetPreflightHelper({ absentOutcome: true }).result.error, 'string');
  for (const field of ['error', 'native_error', 'errors']) {
    const { result, calls } = presetPreflightHelper({ identity: { [field]: ['bad'] } });
    assert.equal(typeof result.error, 'string');
    assert.equal(calls.length, 1);
  }
});

test('folder preset preflight refuses foreign, unbound or malformed success data', () => {
  const known = presetPreflightHelper().result;
  const patches = [
    { ok: false }, { status: 'unknown' }, { status: null }, { mutation_capability: true }, { atomic_snapshot: true },
    { scope: 'auxiliary_type_observation' }, { instance: { ...known.instance, object_uid: 'foreign' } },
    { instance: { ...known.instance, class_uid: 'foreign' } }, { instance: { ...known.instance, native_class: 'foreign' } },
    { anchor_slot_bound: false }, { folder: null }, { folder: { ...known.folder, id: 'Other' } },
    { folder: { ...known.folder, object_uid: 'bad' } }, { folder: { ...known.folder, class_uid: 'bad' } },
    { folder: { ...known.folder, native_class: 'DeviceFolder' } }, { folder: { ...known.folder, path: 'untrusted' } },
    { document_environment_observation: { ...known.document_environment_observation, ancestor_matches: false } },
    { document_environment_observation: null }, { target_flags: 1 }, { target_flags: 3.5 }, { target_flags: 2147483648 },
    { metadata: { ...known.metadata, attribute_count: -1 } }, { metadata: { ...known.metadata, attribute_count: 65 } },
    { metadata: { ...known.metadata, class_name: {} } }, { metadata: { ...known.metadata, class_vendor: 'x'.repeat(257) } },
    { metadata: { ...known.metadata, class_category: 'bad\0text' } }, { path: 'untrusted' },
    { native_error: {} }, { errors: ['failure'] },
  ];
  for (const patch of patches) {
    const { result } = presetPreflightHelper({ result: patch });
    assert.equal(typeof result.error, 'string', JSON.stringify(patch));
    assert.equal(result.folder, undefined);
    assert.equal(result.target_flags, undefined);
  }
});

test('document Environment observations preserve separate known match and unavailable states', () => {
  for (const environment of [
    { status: 'observed', ancestor_matches: true },
    { status: 'observed', ancestor_matches: false },
    { status: 'unavailable', reason: 'fixed child unavailable' },
  ]) {
    const value = { ...environment, scope: 'fixed_active_document_controller_environment', mutation_capability: false, atomic_snapshot: false };
    const { result } = nativeReadHelper('ancestry', 'observation', {
      document_environment_observation: value,
      insert_folder_ancestry_report: { traversal: 'parent_chain_only', ancestor_matches_active_model: true, document_environment_observation: value },
    });
    assert.equal(result.error, undefined);
    assert.equal(result.document_environment_observation.status, value.status);
    assert.equal(result.document_environment_observation.ancestor_matches, value.ancestor_matches);
    assert.equal(result.ancestor_matches_active_model, true);
  }
});

test('document Environment malformed and contradictory evidence never retains a usable predicate', () => {
  const known = { status: 'observed', ancestor_matches: true, scope: 'fixed_active_document_controller_environment', mutation_capability: false, atomic_snapshot: false };
  const unavailable = { status: 'unavailable', reason: 'missing', scope: known.scope, mutation_capability: false, atomic_snapshot: false };
  const invalid = [null, [], {}, { ...known, status: 'unknown' }, { ...known, status: null },
    { ...known, ancestor_matches: 1 }, { ...known, ancestor_matches: null }, { ...known, mutation_capability: true },
    { ...known, atomic_snapshot: true }, { ...known, scope: 'active_model' }, { ...known, path: 'untrusted' },
    { ...unavailable, reason: '' }, { ...unavailable, reason: 'x'.repeat(513) }, { ...unavailable, reason: {} },
    { ...unavailable, ancestor_matches: false }];
  for (const value of invalid) {
    const { result } = nativeReadHelper('ancestry', 'observation', {
      document_environment_observation: value,
      insert_folder_ancestry_report: { traversal: 'parent_chain_only', ancestor_matches_active_model: true, document_environment_observation: value },
    });
    assert.equal(typeof result.error, 'string');
    assert.equal(result.document_environment_observation, undefined);
  }
  for (const [outer, inner] of [[known, { ...known, ancestor_matches: false }], [known, undefined], [undefined, known], [unavailable, known]]) {
    const { result } = nativeReadHelper('ancestry', 'observation', {
      document_environment_observation: outer,
      insert_folder_ancestry_report: { traversal: 'parent_chain_only', ancestor_matches_active_model: true, document_environment_observation: inner },
    });
    assert.equal(typeof result.error, 'string');
    assert.equal(result.insert_folder_ancestry_report, undefined);
  }
});

const nativeReadStatusCases = [
  ['legacy absent statuses', {}, 'identity', true],
  ['both observed', { outcome: 'observed', native_outcome: 'observed' }, true],
  ['outcome observed', { outcome: 'observed' }, true],
  ['native_outcome observed without outcome', { native_outcome: 'observed' }, 'identity', true],
  ['explicit empty errors', { error: null, native_error: null, errors: [] }, true],
  ...['outcome', 'native_outcome'].flatMap(key =>
    ['unknown', 'partially_applied', 'applied', 'applied_unverified', 'applied_verified', 'not_applied', null, '', true, 0, {}, []]
      .map(value => [`${key} ${JSON.stringify(value)}`, { [key]: value }, false])),
  ...['error', 'native_error'].flatMap(key =>
    ['', false, 0, {}, [], { message: 'contradictory read' }]
      .map(value => [`${key} ${JSON.stringify(value)}`, { [key]: value }, false])),
  ...[null, '', false, 0, {}, ['failed'], [null], ['']]
    .map(value => [`errors ${JSON.stringify(value)}`, { errors: value }, false]),
  ['contradictory observed and unknown', { outcome: 'observed', native_outcome: 'unknown' }, false],
  ...[false, null, 1, 'true'].map(value => [`ok ${JSON.stringify(value)}`, { ok: value }, false]),
];

for (const helper of ['legacy', 'ancestry']) {
  for (const stage of ['identity', 'observation']) {
    test(`native read boundary ${helper} ${stage} status matrix`, () => {
      const failures = [];
      for (const [label, patch, acceptance, omitOutcome] of nativeReadStatusCases) {
        const accepted = acceptance === true || acceptance === stage;
        const { result, calls, device, auxiliary } = nativeReadHelper(helper, stage, patch, omitOutcome);
        try {
          if (accepted) {
            assert.ok(result.error === undefined || result.error === null);
            assert.equal(result.ok, true);
            assert.equal(result.mutation_capability, false);
            assert.equal(calls.length, 2);
            assert.equal(calls[1][0], device);
            assert.equal(calls[1][2], auxiliary);
            if (helper === 'ancestry') assert.equal(result.ancestor_matches_active_model, true);
          } else {
            assert.equal(typeof result.error, 'string');
            assert.equal(result.ancestor_matches_active_model, undefined);
            assert.equal(result.insert_folder_ancestry_report, undefined);
            assert.equal(result.auxiliary_report, undefined);
            assert.equal(calls.length, stage === 'identity' ? 1 : 2);
          }
        } catch (error) {
          failures.push({ label, native_calls: calls.length, error: error.message });
        }
      }
      assert.deepEqual(failures, []);
    });
  }
}

test('graph probe reflects the fixed factory, task editor, Inserts folder and bounded ancestry', () => {
  const task = editTaskHarness('track');
  const device = {}, graphFunctions = {}, contextEditor = {}, calls = [], factoryCalls = [];
  const uid = '12345678-1234-1234-1234-123456789ABC';
  const insertFolder = { find(id) { return id === 'FX01' ? { find(child) { return child === 'Device' ? device : null; } } : null; } };
  const channel = { label: 'Fixture', mediaType: 'Audio', find: name => name === 'Inserts' ? insertFolder : null };
  task.Host.Objects.getObjectByName = (name) => name === 'FenderMcpNative' ? {
    capabilities: () => JSON.stringify({ ok: true, api: 'fender-mcp-native-v1' }),
    request(...args) {
      calls.push(args);
      const request = JSON.parse(args[1]);
      return JSON.stringify({ ok: true, instance: { object_uid: uid }, ...(request.operation === 'inspect_aux'
        ? { outcome: 'observed', scope: 'auxiliary_type_observation', mutation_capability: false, auxiliary_report: {
          nodes: [{ identity: null, reflection: { available: true, types: [{ name: args[2] === contextEditor ? 'FixtureEditView' : args[2] === insertFolder ? 'FixtureInsertFolder' : 'EditFunctions' }] } }],
        } } : request.operation === 'inspect_insert_folder_ancestry' ? {
          outcome: 'observed', scope: 'insert_folder_ancestry_observation', mutation_capability: false, atomic_snapshot: false,
          ancestor_matches_active_model: false, insert_folder_ancestry_report: {
            traversal: 'parent_chain_only', ancestor_matches_active_model: false, complete: false,
          },
        } : {}) });
    },
  } : null;
  const context = { functions: { root: {
    environment: { find: () => ({ getChannelList: () => ({ numChannels: 1, getChannel: () => channel }) }) },
    createFunctions(name) {
      factoryCalls.push({ receiver: this, name, argumentCount: arguments.length });
      return name === 'AudioFXGraphEditFunctions' ? graphFunctions : null;
    },
  } }, editor: contextEditor };
  const result = task.run([{ op: 'probe', channel: 'Fixture' }], context);
  assert.equal(result.results[0].graphReflection.mutation_capability, false);
  assert.equal(result.results[0].contextEditorReflection.mutation_capability, false);
  assert.equal(result.results[0].contextEditorReflection.auxiliary_report.nodes[0].reflection.types[0].name, 'FixtureEditView');
  assert.equal(result.results[0].contextEditorReflection.auxiliary_report.nodes[0].identity, null);
  assert.equal(result.results[0].graphReflection.auxiliary_report.nodes[0].reflection.types[0].name, 'EditFunctions');
  assert.equal(result.results[0].insertFolderReflection.mutation_capability, false);
  assert.equal(result.results[0].insertFolderReflection.auxiliary_report.nodes[0].reflection.types[0].name, 'FixtureInsertFolder');
  assert.equal(result.results[0].insertFolderAncestry.ancestor_matches_active_model, false);
  assert.equal(result.results[0].insertFolderAncestry.insert_folder_ancestry_report.complete, false);
  assert.equal(calls.length, 8);
  assert.equal(calls[0].length, 2);
  assert.equal(calls[1].length, 3);
  assert.equal(calls[1][0], device);
  assert.equal(calls[1][2], graphFunctions);
  assert.deepEqual(JSON.parse(calls[1][1]), { operation: 'inspect_aux', expected_instance: uid });
  assert.equal(calls[2].length, 2);
  assert.equal(calls[2][0], device);
  assert.deepEqual(JSON.parse(calls[2][1]), { operation: 'inspect' });
  assert.equal(calls[3].length, 3);
  assert.equal(calls[3][0], device);
  assert.equal(calls[3][2], contextEditor);
  assert.deepEqual(JSON.parse(calls[3][1]), { operation: 'inspect_aux', expected_instance: uid });
  assert.equal(calls[4].length, 2);
  assert.equal(calls[4][0], device);
  assert.deepEqual(JSON.parse(calls[4][1]), { operation: 'inspect' });
  assert.equal(calls[5].length, 3);
  assert.equal(calls[5][0], device);
  assert.equal(calls[5][2], insertFolder);
  assert.deepEqual(JSON.parse(calls[5][1]), { operation: 'inspect_aux', expected_instance: uid });
  assert.equal(calls[6].length, 2);
  assert.equal(calls[6][0], device);
  assert.deepEqual(JSON.parse(calls[6][1]), { operation: 'inspect' });
  assert.equal(calls[7].length, 3);
  assert.equal(calls[7][0], device);
  assert.equal(calls[7][2], insertFolder);
  assert.deepEqual(JSON.parse(calls[7][1]), { operation: 'inspect_insert_folder_ancestry', expected_instance: uid });
  assert.deepEqual(factoryCalls.map(call => call.name), ['DeviceEditFunctions', 'AudioFXGraphEditFunctions']);
  assert.ok(factoryCalls.every(call => call.receiver === context.functions.root && call.argumentCount === 1));
});

for (const [label, editorResponse] of [
  ['malformed native JSON', '{not json'],
  ['foreign Device identity', JSON.stringify({ ok: true,
    instance: { object_uid: 'AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA' },
    scope: 'auxiliary_type_observation', mutation_capability: false, auxiliary_report: { nodes: [] } })],
]) {
  test(`context editor reflection fails closed on ${label}`, () => {
    const task = editTaskHarness('track');
    const device = {}, graphFunctions = {}, editor = {}, calls = [];
    const uid = '12345678-1234-1234-1234-123456789ABC';
    const channel = { label: 'Fixture', mediaType: 'Audio', find(name) {
      return name === 'Inserts' ? { find(id) { return id === 'FX01' ? { find(child) { return child === 'Device' ? device : null; } } : null; } } : null;
    } };
    task.Host.Objects.getObjectByName = name => name === 'FenderMcpNative' ? {
      capabilities: () => JSON.stringify({ ok: true, api: 'fender-mcp-native-v1' }),
      request(...args) {
        calls.push(args);
        if (args[2] === editor) return editorResponse;
        const request = JSON.parse(args[1]);
        return JSON.stringify({ ok: true, instance: { object_uid: uid }, ...(request.operation === 'inspect_aux'
          ? { outcome: 'observed', scope: 'auxiliary_type_observation', mutation_capability: false, auxiliary_report: { nodes: [] } } : {}) });
      },
    } : null;
    const context = { editor, functions: { root: {
      environment: { find: () => ({ getChannelList: () => ({ numChannels: 1, getChannel: () => channel }) }) },
      createFunctions: name => name === 'AudioFXGraphEditFunctions' ? graphFunctions : null,
    } } };
    const row = task.run([{ op: 'probe', channel: 'Fixture', auxiliary: { untrusted: true } }], context).results[0];
    assert.equal(row.graphReflection.mutation_capability, false);
    assert.equal(typeof row.contextEditorReflection.error, 'string');
    assert.equal(row.contextEditorReflection.auxiliary_report, undefined);
    assert.equal(calls.length, 8);
    assert.equal(calls[3][2], editor);
  });
}

for (const malformed of ['json', 'foreign_identity', 'wrong_scope', 'mutation', 'atomic', 'missing_predicate', 'delegate_traversal', 'predicate_mismatch']) {
  test(`fixed Inserts ancestry fails closed on ${malformed}`, () => {
    const task = editTaskHarness('track');
    const device = {}, editor = {}, calls = [];
    const uid = '12345678-1234-1234-1234-123456789ABC';
    const inserts = { find: name => name === 'FX01' ? { find: child => child === 'Device' ? device : null } : null };
    const channel = { label: 'Fixture', mediaType: 'Audio', find: name => name === 'Inserts' ? inserts : null };
    const response = { ok: true, outcome: 'observed', instance: { object_uid: uid }, scope: 'insert_folder_ancestry_observation',
      mutation_capability: false, atomic_snapshot: false, ancestor_matches_active_model: true,
      insert_folder_ancestry_report: { traversal: 'parent_chain_only', ancestor_matches_active_model: true } };
    if (malformed === 'foreign_identity') response.instance.object_uid = 'AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA';
    if (malformed === 'wrong_scope') response.scope = 'auxiliary_type_observation';
    if (malformed === 'mutation') response.mutation_capability = true;
    if (malformed === 'atomic') response.atomic_snapshot = true;
    if (malformed === 'missing_predicate') delete response.ancestor_matches_active_model;
    if (malformed === 'delegate_traversal') response.insert_folder_ancestry_report.traversal = 'delegate';
    if (malformed === 'predicate_mismatch') response.insert_folder_ancestry_report.ancestor_matches_active_model = false;
    task.Host.Objects.getObjectByName = name => name === 'FenderMcpNative' ? {
      capabilities: () => JSON.stringify({ ok: true, api: 'fender-mcp-native-v1' }),
      request(...args) {
        calls.push(args);
        const request = JSON.parse(args[1]);
        if (request.operation === 'inspect_insert_folder_ancestry') return malformed === 'json' ? '{invalid' : JSON.stringify(response);
        return JSON.stringify({ ok: true, instance: { object_uid: uid }, ...(request.operation === 'inspect_aux'
          ? { outcome: 'observed', scope: 'auxiliary_type_observation', mutation_capability: false, auxiliary_report: { nodes: [] } } : {}) });
      },
    } : null;
    const context = { editor, functions: { root: {
      environment: { find: () => ({ getChannelList: () => ({ numChannels: 1, getChannel: () => channel }) }) },
      createFunctions: () => ({}),
    } } };
    const row = task.run([{ op: 'probe', channel: 'Fixture', auxiliary: { untrusted: true }, ancestors: 128,
      path: 'foreign/path', operation: 'set_parameter' }], context).results[0];
    assert.equal(typeof row.insertFolderAncestry.error, 'string');
    assert.equal(row.insertFolderAncestry.insert_folder_ancestry_report, undefined);
    const ancestryCalls = calls.filter(args => JSON.parse(args[1]).operation === 'inspect_insert_folder_ancestry');
    assert.equal(ancestryCalls.length, 1);
    assert.equal(ancestryCalls[0][0], device);
    assert.equal(ancestryCalls[0][2], inserts);
    assert.deepEqual(JSON.parse(ancestryCalls[0][1]), { operation: 'inspect_insert_folder_ancestry', expected_instance: uid });
  });
}

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
