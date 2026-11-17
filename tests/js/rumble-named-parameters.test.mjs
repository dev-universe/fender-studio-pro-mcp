import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';
const instance={object_uid:'12345678-1234-1234-1234-123456789ABC',class_uid:'B9D7536C-A010-457D-A182-36E5531164C9',native_class:'AudioEffect',object_id:'Rumble Native'};
const target={channel:'Scratch',slot:0,plugin:'Rumble Native',expected_instance:instance.object_uid,action:'read_rumble_named_parameters'};
const failure={ok:false,outcome:'not_applied',native_outcome:'not_applied',mutation_capability:false,can_set_via_this_action:false,selection_authority:false,atomic_snapshot:false,audio_verified:false,persistence_verified:false,error:{code:'rumble_named_parameters_unavailable',message:'The bounded Rumble named-parameter observation was not established'}};
function observation() {
 return {ok:true,outcome:'observed',operation:'rumble_named_parameters_read',schema:'rumble_named_parameters_v1',instance:{...instance},source:'component_alias_fixed_controllers',scope:'fixed_immediate_serialized_sibling_names',controllers:Array.from({length:26},(_,i)=>({slot:i===25?'amp':`effect${i+1}`,serialized_class_uid:'00000000-0000-0000-0000-000000000000',status:'present',parameter_count:0,expected_names:[],parameters:[]})),models_sha256:'a7995d1e57ccc8f7983c4c45a2c6e8a0931cdda5617d1a7453d34d716302269b',before_after_model_ids_equal:true,before_after_name_sets_equal:true,name_set_complete:true,controller_counts_complete:true,named_metadata_complete:true,access_metadata_complete:true,current_values_complete:true,expected_name_count:0,matched_name_count:0,value_semantics:'sequential_snapshot',mutation_capability:false,can_set_via_this_action:false,selection_authority:false,atomic_snapshot:false,audio_verified:false,persistence_verified:false};
}
function description() {
 const number={status:'observed',variant_tag:1,kind:'integer',decimal:'9223372036854775807'};
 return {name:'gain',tag:7,kind:1,enabled:true,read_only:false,current:{...number},min:{...number},max:{...number},default:{...number},normalized:{status:'observed',bits_hex:'40000000'},support:{raw_number:true,normalized:true}};
}
function positive() {const v=observation();v.controllers[0].expected_names=['gain'];v.controllers[0].parameters=[{name:'gain',status:'observed',description:description()}];v.expected_name_count=1;v.matched_name_count=1;return v;}
function incomplete(v) {for(const k of ['named_metadata_complete','access_metadata_complete','current_values_complete'])v[k]=false;return v;}
function setNames(v,index,names) {Object.assign(v.controllers[index],{expected_names:names,parameters:names.map(name=>({name,status:'parameter_absent',description:null}))});v.expected_name_count=v.controllers.reduce((n,row)=>n+row.expected_names.length,0);incomplete(v);}
function harness(reply=()=>observation()) {
 const calls=[],context={open:true,document:'rumble:1',session:'rumble'};
 const service={capabilities(){return JSON.stringify({ok:true,api:'fender-mcp-native-v1'});},request(device,body){const request=JSON.parse(body);calls.push(request);const v=reply(request,context);return typeof v==='string'?v:JSON.stringify(v);}};
 const scope=vm.createContext({include_file(){},memberTypes(){return{};},PreSonus:{ControlSurfaceComponent:class{}},Host:{Objects:{getObjectByName(){return service;}}}});
 vm.runInContext(asset('bridge/NativePluginControls.js'),scope);vm.runInContext(asset('bridge/BridgeComponent.js'),scope);
 const component=vm.runInContext('createBridgeComponent()',scope);component.bridge={requestContext:{...context},context(){return{...context};}};component.pluginOf=()=>({name:'Rumble Native',device:{}});
 return{component,calls,context,controls:vm.runInContext('new NativePluginControls({})',scope)};
}
function erased(value) {assert.deepEqual(JSON.parse(JSON.stringify(value)),failure);assert.equal(JSON.stringify(value).includes('PRIVATE'),false);}
test('named positive observations survive count zero and independently large indexed aggregates',()=>{
 for(const factory of [observation,positive]) {const h=harness(factory),v=h.component.pluginState(target);assert.equal(v.ok,true);assert.equal(Object.keys(v).length,27);assert.equal(v.controllers.length,26);assert.deepEqual(h.calls,[{operation:'rumble_named_parameters_read',expected_instance:instance.object_uid}]);assert.equal(v.can_set_via_this_action,false);}
 const value=positive();value.controllers.forEach(row=>row.parameter_count=128);assert.equal(harness(()=>value).component.pluginState(target).ok,true);
});
test('every unavailable controller keeps expected names while count and parameters are null',()=>{
 for(const status of ['node_absent','controller_absent','qi_failed','malformed_success','malformed_absence','identity_mismatch','negative_count','count_limit']) {
  const v=positive();Object.assign(v.controllers[0],{status,parameter_count:null,parameters:null});incomplete(v);v.controller_counts_complete=false;v.matched_name_count=0;assert.equal(harness(()=>v).component.pluginState(target).ok,true,status);v.controllers[0].parameter_count=0;erased(harness(()=>v).component.pluginState(target));
 }
 const v=observation();Object.assign(v.controllers[0],{status:'node_absent',parameter_count:null,parameters:null});incomplete(v);v.controller_counts_complete=false;assert.equal(harness(()=>v).component.pluginState(target).ok,true);
});
test('all named lookup holes preserve independent controller count completeness',()=>{
 for(const status of ['parameter_absent','description_unavailable','unsupported_metadata']) {const v=positive();Object.assign(v.controllers[0].parameters[0],{status,description:null});incomplete(v);v.matched_name_count=0;assert.equal(harness(()=>v).component.pluginState(target).ok,true);v.named_metadata_complete=true;erased(harness(()=>v).component.pluginState(target));}
});
test('names are strict sorted unique ASCII bounded by slot and total',()=>{
 for(const names of [[''],['1gain'],['gain.x'],['gain/x'],['gain\n'],['é'],['a'.repeat(65)],['gain','gain'],['b','a'],Array.from({length:65},(_,i)=>`n${String(i).padStart(2,'0')}`)]) {const v=observation();setNames(v,0,names);erased(harness(()=>v).component.pluginState(target));}
 const v=observation();setNames(v,0,['a'.repeat(64)]);assert.equal(harness(()=>v).component.pluginState(target).ok,true);
 for(let i=0;i<4;i++)setNames(v,i,Array.from({length:64},(_,j)=>`n${String(j).padStart(2,'0')}`));assert.equal(harness(()=>v).component.pluginState(target).ok,true);setNames(v,4,['extra']);erased(harness(()=>v).component.pluginState(target));
});
test('names and parameter rows match exactly including descriptor names',()=>{
 for(const mutate of [v=>v.controllers[0].parameters=[],v=>v.controllers[0].parameters[0].name='other',v=>v.controllers[0].parameters[0].description.name='other',v=>v.controllers[0].parameters.push(v.controllers[0].parameters[0]),v=>v.controllers[0].parameters[0].index=0]) {const v=positive();mutate(v);erased(harness(()=>v).component.pluginState(target));}
});
test('all derived counts and completeness flags are checked independently',()=>{
 for(const key of ['controller_counts_complete','named_metadata_complete','access_metadata_complete','current_values_complete','name_set_complete','before_after_model_ids_equal','before_after_name_sets_equal']) {const v=positive();v[key]=false;erased(harness(()=>v).component.pluginState(target));}
 for(const key of ['expected_name_count','matched_name_count']) {const v=positive();v[key]=2;erased(harness(()=>v).component.pluginState(target));}
});
test('precise i64 f64 and f32 carriers survive named adaptation',()=>{
 for(const current of [{status:'observed',variant_tag:1,kind:'integer',decimal:'9007199254740993'},{status:'observed',variant_tag:1,kind:'integer',decimal:'-9223372036854775808'},{status:'observed',variant_tag:2,kind:'float64',bits_hex:'8000000000000000'}]) {const v=positive();v.controllers[0].parameters[0].description.current=current;const result=harness(()=>v).component.pluginState(target);assert.equal(result.ok,true);assert.deepEqual(JSON.parse(JSON.stringify(result.controllers[0].parameters[0].description.current)),current);}
 const v=positive(),d=v.controllers[0].parameters[0].description;d.kind=4;d.current.decimal='-1';d.default={status:'unavailable',variant_tag:null,reason:'missing'};assert.equal(harness(()=>v).component.pluginState(target).ok,true);d.normalized.bits_hex='7f800000';erased(harness(()=>v).component.pluginState(target));
});
test('unknown kinds disabled readonly and unavailable current preserve support policy',()=>{
 for(const change of [{kind:999},{enabled:false},{read_only:true}]) {const v=positive(),d=v.controllers[0].parameters[0].description;Object.assign(d,change);erased(harness(()=>v).component.pluginState(target));d.support={raw_number:false,normalized:false};assert.equal(harness(()=>v).component.pluginState(target).ok,true);}
 const v=positive(),d=v.controllers[0].parameters[0].description;d.current={status:'unavailable',variant_tag:2,reason:'non_finite'};d.support={raw_number:false,normalized:false};v.current_values_complete=false;assert.equal(harness(()=>v).component.pluginState(target).ok,true);
});
test('closed named schema rejects private fields forged discriminators and ordered controller drift',()=>{
 const mutations=[v=>v.operation='state_capture',v=>v.schema='other',v=>v.atomic_snapshot=true,v=>v.token='PRIVATE',v=>v.instance.object_uid='OTHER',v=>v.controllers.reverse(),v=>v.controllers[0].serialized_class_uid+='\n',v=>v.controllers[0].parameters[0].description.formatter='PRIVATE',v=>v.controllers[0].parameters[0].description.value='PRIVATE',v=>v.controllers[0].parameters[0].description.current.decimal='9223372036854775808',v=>v.controllers[0].parameters[0].description.current.decimal='1\n',v=>v.controllers[0].parameters[0].description.current={status:'observed',variant_tag:2,kind:'float64',bits_hex:'8000000000000000\n'},v=>v.controllers[0].parameters[0].description.normalized.bits_hex='40000000\n',v=>v.models_sha256+='\n'];
 for(const mutate of mutations){const v=positive();mutate(v);erased(harness(()=>v).component.pluginState(target));}
});
test('named requests reject extra fields invalid targets and noncanonical instances before native dispatch',()=>{
 for(const extra of [{token:'PRIVATE'},{token:''},{token:null},{source:'PRIVATE'},{name:'PRIVATE'},{path:'PRIVATE'},{bytes:'PRIVATE'},{value:1},{script:'PRIVATE'},{plugin:'Other'},{expected_instance:''},{expected_instance:instance.object_uid+'\n'},{expected_instance:instance.object_uid.toLowerCase()},{expected_instance:'00000000-0000-0000-0000-000000000000'},{kind:'instrument'},{kind:'panner'},{slot:16},{slot:null},{channel:''}]) {const h=harness();erased(h.component.pluginState({...target,...extra}));assert.equal(h.calls.length,0);}
});
test('native malformed replies throws and late document session or attachment loss erase all evidence',()=>{
 for(const mutate of [c=>c.document='other:1',c=>c.session='other',c=>c.open=false]) {const h=harness((r,c)=>{mutate(c);return {...positive(),operation:'state_capture',token:'PRIVATE'};});erased(h.component.pluginState(target));}
 for(const reply of [()=>{throw Error('PRIVATE');},()=>'{PRIVATE',()=>'[]',()=>({ok:false,operation:'state_capture',controllers:[{private:'PRIVATE'}]})]) erased(harness(reply).component.pluginState(target));
});
test('outer bridge uses trusted action to erase preflight and late-context failures',()=>{
 const h=bridgeHarness();let calls=0;h.bridge.component={pluginState(){calls++;h.manager.activeDocument={title:'Other'};return {...positive(),operation:'state_capture',token:'PRIVATE'};}};
 erased(h.request({op:'pluginState',args:target}).result);assert.equal(calls,0);erased(h.writeRequest({op:'pluginState',args:target}).result);assert.equal(calls,1);
 const mutated=bridgeHarness();mutated.bridge.component={pluginState(args){args.action='capture';mutated.manager.activeDocument={title:'Other'};return {...positive(),operation:'state_capture',token:'PRIVATE'};}};
 erased(mutated.writeRequest({op:'pluginState',args:{...target}}).result);
});
test('oversized native replies fail whole observation without truncation',()=>{
 const v=positive();v.controllers[0].parameters[0].description.name='PRIVATE'.repeat(50000);assert.ok(JSON.stringify(v).length>262144);erased(harness(()=>v).component.pluginState(target));
});
