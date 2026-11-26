import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';
const instance={object_uid:'12345678-1234-1234-1234-123456789ABC',class_uid:'B9D7536C-A010-457D-A182-36E5531164C9',native_class:'AudioEffect',object_id:'Rumble Native'};
const target={channel:'Scratch',slot:0,plugin:'Rumble Native',expected_instance:instance.object_uid,action:'read_rumble_state'};
function observation() {
  return {ok:true,outcome:'observed',operation:'rumble_model_state_read',schema:'rumble_processor_state_v1',serialization:'component_alias_IStorable',instance:{...instance},source:'component_alias',capture_sha256:'a'.repeat(64),capture_byte_count:100,scope:'direct_serialized_processor_ids_and_numeric_settings',complete:true,
    processors:Array.from({length:26},(_,i)=>({slot:i===25?'amp':`effect${i+1}`,class_uid:'00000000-0000-0000-0000-000000000000',zero_guid:true,settings:[]})),processor_count:26,numeric_setting_count:0,models_sha256:'b'.repeat(64),settings_sha256:'c'.repeat(64),numeric_encoding:'json_parsed_u64_i64_f64_bits_v1',value_semantics:'sequential_snapshot',native_access_metadata_verified:false,mutation_capability:false,audio_verified:false,persistence_verified:false};
}
function harness(reply=()=>observation(), plugin=target.plugin) {
  const calls=[],context={open:true,document:'rumble:1',session:'rumble'};
  const service={capabilities(){return JSON.stringify({ok:true,api:'fender-mcp-native-v1'});},request(device,body){const r=JSON.parse(body);calls.push(r);const v=reply(r,context);return typeof v==='string'?v:JSON.stringify(v);}};
  const scope=vm.createContext({include_file(){},memberTypes(){return{};},PreSonus:{ControlSurfaceComponent:class{}},Host:{Objects:{getObjectByName(){return service;}}}});
  vm.runInContext(asset('bridge/NativePluginControls.js'),scope);vm.runInContext(asset('bridge/BridgeComponent.js'),scope);
  const component=vm.runInContext('createBridgeComponent()',scope);component.bridge={requestContext:{...context},context(){return{...context};}};component.pluginOf=()=>({name:plugin,device:{}});
  return{component,calls,context,controls:vm.runInContext('new NativePluginControls({})',scope)};
}
function erased(v) {assert.equal(v.ok,false);for(const k of ['processors','instance','source','schema','models_sha256','settings_sha256','capture_sha256','token'])assert.equal(k in v,false,k);for(const k of ['native_access_metadata_verified','mutation_capability','audio_verified','persistence_verified'])assert.equal(v[k],false);}
test('fixed processor-state reader keeps all26 identities and no capsule',()=>{
  const h=harness(),v=h.component.pluginState(target);assert.equal(v.ok,true);assert.equal(v.processors.length,26);assert.equal(v.native_access_metadata_verified,false);assert.equal(v.token,undefined);assert.deepEqual(h.calls,[{operation:'rumble_state_read',expected_instance:instance.object_uid}]);
});
test('a numbered Rumble insert keeps its native label and requires the exact identity', () => {
  const duplicate = { ...instance, object_id: 'Rumble Native 2' };
  const h = harness(() => ({ ...observation(), instance: duplicate }));
  const result = h.component.pluginState({ ...target, slot: 1 });
  assert.equal(result.ok, true);
  assert.equal(result.instance.object_id, duplicate.object_id);
  assert.equal(result.instance.object_uid, duplicate.object_uid);
  assert.equal(result.processors.length, 26);

  for (const identity of [
    { ...duplicate, object_id: '' },
    { ...duplicate, class_uid: '00000000-0000-0000-0000-000000000000' },
    { ...duplicate, object_uid: '00000000-0000-0000-0000-000000000000' },
  ]) {
    const wrong = harness(() => ({ ...observation(), instance: identity }));
    erased(wrong.component.pluginState({ ...target, slot: 1 }));
  }
});
test('numeric wire checks retain exact integer bounds and finite float bits',()=>{
  const h=harness();for(const value of [{kind:'unsigned_integer',decimal:'18446744073709551615'},{kind:'signed_integer',decimal:'-9223372036854775808'},{kind:'float64',bits_hex:'8000000000000000'}])assert.equal(h.controls.processorStateNumber(value),true);
  for(const value of [{kind:'unsigned_integer',decimal:'18446744073709551616'},{kind:'unsigned_integer',decimal:'01'},{kind:'signed_integer',decimal:'0'},{kind:'signed_integer',decimal:'-9223372036854775809'},{kind:'float64',bits_hex:'7ff0000000000000'},{kind:'float64',bits_hex:'fff8000000000000'}])assert.equal(h.controls.processorStateNumber(value),false);
});
test('incomplete duplicate unsorted and private processor schemas fail closed',()=>{
  const mutations=[v=>v.processors.pop(),v=>{v.processors[0].slot='amp';},v=>{v.processors[0].zero_guid=false;},v=>{v.processors[0].class_uid='{00000000-0000-0000-0000-000000000000}';},v=>{v.processors[0].settings=[{name:'z',value:{kind:'float64',bits_hex:'0000000000000000'}},{name:'a',value:{kind:'float64',bits_hex:'0000000000000000'}}];v.numeric_setting_count=2;},v=>{v.processors[0].settings=[{name:'private-path',value:{kind:'float64',bits_hex:'0000000000000000'}}];v.numeric_setting_count=1;},v=>{v.numeric_setting_count=1;},v=>{v.native_access_metadata_verified=true;},v=>{v.token='PRIVATE';},v=>{v.settings_sha256='PRIVATE';},v=>{v.instance.object_uid='OTHER';}];
  for(const mutate of mutations){const h=harness(()=>{const v=observation();mutate(v);return v;});erased(h.component.pluginState(target));}
});
test('unchecked state requests and missing exact target reject before native access',()=>{
  for(const extra of [{token:'private'},{source:'private'},{path:'private'},{bytes:'private'},{rumble_slot:'amp'},{plugin:'Other'},{expected_instance:''}]){const h=harness();erased(h.component.pluginState({...target,...extra}));assert.equal(h.calls.length,0);}
});
test('native drift exceptions and malformed replies erase all state evidence',()=>{
  for(const mutate of [c=>{c.document='other:1';},c=>{c.session='other';},c=>{c.open=false;}]){const h=harness((r,c)=>{mutate(c);return observation();});erased(h.component.pluginState(target));}
  for(const reply of [()=>{throw Error('PRIVATE');},()=>'{PRIVATE',()=>'[]']){const h=harness(reply),v=h.component.pluginState(target);erased(v);assert.equal(JSON.stringify(v).includes('PRIVATE'),false);}
});
test('outer bridge requires fresh state context and removes late observations',()=>{
  const h=bridgeHarness();let calls=0;h.bridge.component={pluginState(){calls++;h.manager.activeDocument={title:'Other'};return observation();}};
  assert.equal(h.request({op:'pluginState',args:target}).ok,false);assert.equal(calls,0);
  const v=h.writeRequest({op:'pluginState',args:target});assert.equal(v.ok,false);assert.equal(v.code,'native_context_lost');assert.equal(v.result,undefined);
});

test('Mustang processor reads require exact class, action, effect target and document', () => {
  const mustang = { ...target, plugin: 'Mustang Native', action: 'read_mustang_state', kind: 'effect' };
  const receipt = {
    ...observation(),
    operation: 'mustang_model_state_read',
    schema: 'mustang_processor_state_v1',
    instance: { ...instance, class_uid: '11B8AA91-9BED-4A7F-90F4-02C6A6C7AF34', object_id: 'Mustang Native 2' },
  };
  const h = harness(() => receipt, mustang.plugin);
  const result = h.component.pluginState(mustang);
  assert.equal(result.ok, true);
  assert.equal(result.plugin, mustang.plugin);
  assert.equal(result.instance.object_id, 'Mustang Native 2');
  assert.deepEqual(h.calls, [{ operation: 'mustang_state_read', expected_instance: instance.object_uid }]);

  const crossed = harness(() => observation(), mustang.plugin);
  const failure = crossed.component.pluginState(mustang);
  erased(failure);
  assert.equal(failure.error.code, 'mustang_processor_state_unavailable');

  const wrongTarget = harness(() => receipt);
  const wrongKind = harness(() => receipt, mustang.plugin);
  erased(wrongTarget.component.pluginState(mustang));
  erased(wrongKind.component.pluginState({ ...mustang, kind: 'instrument' }));
  assert.equal(wrongTarget.calls.length + wrongKind.calls.length, 0);

  const outer = bridgeHarness();
  let dispatched = 0;
  outer.bridge.component = {
    pluginState() {
      dispatched++;
      outer.manager.activeDocument = { title: 'Other' };
      return result;
    },
  };
  assert.equal(outer.request({ op: 'pluginState', args: mustang }).ok, false);
  assert.equal(dispatched, 0);
  const late = outer.writeRequest({ op: 'pluginState', args: mustang });
  assert.equal(late.code, 'native_context_lost');
  assert.equal(late.result, undefined);
});
