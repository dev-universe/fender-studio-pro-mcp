import vm from 'node:vm';
import { readFileSync } from 'node:fs';

export const mailbox = 'file:///contract-mailbox/';
export const asset = (path) => readFileSync(new URL(`../../assets/${path}`, import.meta.url), 'utf8');

export function memoryHost() {
  const files = new Map();
  const document = { title: 'Scratch', path: { url: 'file:///scratch/song.song' } };
  const manager = { activeDocument: document };
  const effects = [];
  const Host = {
    Url: (url) => ({ url }),
    IO: {
      File: ({ url }) => ({ exists: () => files.has(url) }),
      openTextFile: ({ url }) => {
        let read = false;
        return { readLine: () => read ? undefined : (read = true, files.get(url)), close() {} };
      },
      createTextFile: ({ url }) => {
        let text = '';
        return { writeString: (value) => { text += value; }, close: () => files.set(url, '\uFEFF' + text) };
      },
    },
    Objects: { getObjectByUrl: (url) => url === '://studioapp/DocumentManager' ? manager : null },
    Attributes: () => ({ setAttribute(key, value) { this[key] = value; } }),
    GUI: { Commands: {
      findCommand: (category, name) => category === 'Contract' && name === 'Increment' ? {} : null,
      interpretCommand: (category, name, checkOnly) => {
        if (category !== 'Contract' || name !== 'Increment') return false;
        if (!checkOnly) effects.push({ category, name });
        return true;
      },
    } },
    Results: { kResultOk: 0 },
    Interfaces: { IEditTask: 'IEditTask' },
  };
  const write = (name, value) => files.set(mailbox + name, JSON.stringify(value) + '\n');
  const read = (name) => {
    const text = files.get(mailbox + name);
    return text === undefined ? null : JSON.parse(text.replace(/^\uFEFF/, ''));
  };
  return { Host, files, manager, document, effects, write, read };
}

export function bridgeHarness() {
  const host = memoryHost();
  const context = vm.createContext({ Host: host.Host, PreSonus: {} });
  vm.runInContext(asset('edit-tasks/package/McpDocumentContext.js'), context, { filename: 'McpDocumentContext.js' });
  vm.runInContext(asset('bridge/BridgeCore.js'), context, { filename: 'BridgeCore.js' });
  const Bridge = vm.runInContext('Bridge', context);
  const bridge = new Bridge({ mailbox, allowEval: true }, null);
  let serial = 0;
  const request = (fields) => {
    const payload = { protocol: 2, id: `request-${++serial}`, args: {}, ...fields };
    host.write('request.json', payload);
    bridge.tick('contract');
    return host.read('response.json');
  };
  const current = () => request({ op: 'context' }).result;
  const writeRequest = (fields = {}) => {
    const scope = current();
    return request({
      expectedSession: scope.session, expectedDocument: scope.document,
      op: 'command', args: { category: 'Contract', name: 'Increment' }, ...fields,
    });
  };
  return { ...host, bridge, request, current, writeRequest };
}

export function editTaskHarness(kind = 'track') {
  const host = memoryHost();
  const context = vm.createContext({ Host: host.Host, McpEditConfig: { mailbox, allowEval: true } });
  context.include_file = (name) => {
    if (name !== 'McpEditConfig.js') {
      vm.runInContext(asset(`edit-tasks/package/${name}`), context, { filename: name });
    }
  };
  const filename = kind === 'track' ? 'McpTrackEdit.js' : 'McpEdit.js';
  const requestFilename = kind === 'track' ? 'track-edit-request.json' : 'edit-request.json';
  const resultFilename = kind === 'track' ? 'track-edit-result.json' : 'edit-result.json';
  vm.runInContext(asset(`edit-tasks/package/${filename}`), context, { filename });
  const task = vm.runInContext(kind === 'track' ? 'createMcpTrackEdit()' : 'createMcpEdit()', context);
  let serial = 0;
  const arm = (ops, overrides = {}) => {
    const id = `edit-${++serial}`;
    const session = 'edit-session';
    const document = `${session}:1`;
    const active = host.manager.activeDocument;
    host.write('status.json', { protocol: 2, session, ...overrides.status });
    host.write('task-lease.json', {
      id, kind, session, document,
      title: typeof active?.title === 'string' ? active.title : null,
      fileUrl: typeof active?.path?.url === 'string' ? active.path.url : null,
      expiresAt: Date.now() + 10_000,
      ...overrides.lease,
    });
    host.write(requestFilename, { id, ops, expectedSession: session, expectedDocument: document, ...overrides.request });
    return id;
  };
  const invoke = (editContext) => {
    task.performEdit(editContext);
    return host.read(resultFilename);
  };
  const run = (ops, editContext) => {
    const id = arm(ops);
    const result = invoke(editContext);
    if (result.id !== id) throw new Error('EditTask returned an unrelated request ID');
    return result;
  };
  return { ...host, task, run, arm, invoke, requestFilename, resultFilename };
}

export function musicContext() {
  const iterator = (list) => {
    let index = 0;
    return { done: () => index >= list.length, next: () => list[index++] || null };
  };
  const makeTrack = (name, mediaType) => {
    const track = { name, mediaType, events: [] };
    track.createIterator = () => iterator(track.events);
    return track;
  };
  const keys = makeTrack('Keys', 'Music');
  const drums = makeTrack('Drums', 'Music');
  const audio = makeTrack('Audio', 'Audio');
  const tracks = [keys, drums, audio];
  const parts = {
    addMusicPart: (start, end, track) => {
      const part = { name: track.name, startTime: { seconds: start.seconds }, endTime: { seconds: end.seconds }, notes: [] };
      track.events.push(part);
      return part;
    },
  };
  const music = {
    createEvent: () => ({}),
    insertEvent: (part, note) => part.notes.push(note),
    modifyPitch: (note, pitch) => { note.pitch = pitch; },
    modifyVelocity: (note, velocity) => { note.velocity = Math.round(velocity * 127); },
    resizeEvent: (note, length) => { note.length = length; },
    moveEvent: (note, beat) => { note.beat = beat; },
  };
  const functions = {
    newMediaTime: () => ({ seconds: 0 }),
    renameEvent: (event, name) => { event.name = name; },
    root: {
      createIterator: () => iterator(tracks),
      createFunctions: (name) => name === 'MusicPartFunctions' ? parts : name === 'MusicFunctions' ? music : null,
    },
  };
  return { keys, drums, audio, parts, music, context: { functions } };
}
