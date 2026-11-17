import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { asset, bridgeHarness } from './host.mjs';
const instance={object_uid:'12345678-1234-1234-1234-123456789ABC',class_uid:'B9D7536C-A010-457D-A182-36E5531164C9',native_class:'AudioEffect',object_id:'Rumble Native'};
const target={channel:'Scratch',slot:0,plugin:'Rumble Native',expected_instance:instance.object_uid,action:'read_rumble_parameters'};
function observation() {
 return {ok:true,outcome:'observed',operation:'rumble_parameters_read',schema:'rumble_controller_parameters_v1',instance:{...instance},source:'component_alias_fixed_controllers',scope:'fixed_immediate_controller_parameter_lists',controllers:Array.from({length:26},(_,i)=>({slot:i===25?'amp':`effect${i+1}`,serialized_class_uid:'00000000-0000-0000-0000-000000000000',status:'present',parameter_count:0,enumeration_status:'complete',parameters:[],serialized_name_count:0,matched_serialized_name_count:0})),models_sha256:'a7995d1e57ccc8f7983c4c45a2c6e8a0931cdda5617d1a7453d34d716302269b',before_after_model_ids_equal:true,census_complete:true,parameter_lists_complete:true,access_metadata_complete:true,current_values_complete:true,serialized_name_coverage_complete:true,reported_parameter_count:0,enumerated_parameter_count:0,descriptor_budget_exceeded:false,value_semantics:'sequential_snapshot',mutation_capability:false,can_set_via_this_action:false,selection_authority:false,atomic_snapshot:false,audio_verified:false,persistence_verified:false};
}
function description() {
 const number={status:'observed',variant_tag:1,kind:'integer',decimal:'9223372036854775807'};
 return {name:'gain',tag:7,kind:1,enabled:true,read_only:false,current:{...number},min:{...number},max:{...number},default:{...number},normalized:{status:'observed',bits_hex:'80000000'},support:{raw_number:true,normalized:true}};
}
function positive() {const v=observation(),row=v.controllers[0];row.parameter_count=1;row.parameters=[{index:0,status:'observed',description:description()}];row.serialized_name_count=1;row.matched_serialized_name_count=1;v.reported_parameter_count=1;v.enumerated_parameter_count=1;return v;}
function incomplete(v,census=false) {for(const k of ['census_complete','parameter_lists_complete','access_metadata_complete','current_values_complete','serialized_name_coverage_complete'])v[k]=false;v.census_complete=census;return v;}
function harness(reply=()=>observation()) {
  const calls=[],context={open:true,document:'rumble:1',session:'rumble'};
  const service={capabilities(){return JSON.stringify({ok:true,api:'fender-mcp-native-v1'});},request(device,body){const r=JSON.parse(body);calls.push(r);const v=reply(r,context);return typeof v==='string'?v:JSON.stringify(v);}};
  const scope=vm.createContext({include_file(){},memberTypes(){return{};},PreSonus:{ControlSurfaceComponent:class{}},Host:{Objects:{getObjectByName(){return service;}}}});
  vm.runInContext(asset('bridge/NativePluginControls.js'),scope);vm.runInContext(asset('bridge/BridgeComponent.js'),scope);
  const component=vm.runInContext('createBridgeComponent()',scope);component.bridge={requestContext:{...context},context(){return{...context};}};component.pluginOf=()=>({name:'Rumble Native',device:{}});
  return{component,calls,context,controls:vm.runInContext('new NativePluginControls({})',scope)};
}

function erased(v) {assert.equal(v.ok,false);assert.equal(Object.keys(v).length,10);for(const k of ['controllers','instance','source','schema','models_sha256','token'])assert.equal(k in v,false,k);for(const k of ['mutation_capability','can_set_via_this_action','selection_authority','atomic_snapshot','audio_verified','persistence_verified'])assert.equal(v[k],false);}
test('native controller metadata keeps zero lists and integer carriers without write authority',()=>{
 for(const factory of [observation,positive]) {const h=harness(factory),v=h.component.pluginState(target);assert.equal(v.ok,true);assert.equal(v.controllers.length,26);assert.equal(v.can_set_via_this_action,false);assert.deepEqual(h.calls,[{operation:'rumble_parameters_read',expected_instance:instance.object_uid}]);}
 const h=harness(positive);assert.equal(h.component.pluginState(target).controllers[0].parameters[0].description.current.decimal,'9223372036854775807');
});
test('every unavailable controller status preserves null rather than zero',()=>{
 for(const status of ['node_absent','controller_absent','qi_failed','malformed_success','malformed_absence','identity_mismatch','negative_count','count_limit']) {const v=observation();Object.assign(v.controllers[0],{status,parameter_count:null,parameters:null,enumeration_status:'unavailable'});incomplete(v);assert.equal(harness(()=>v).component.pluginState(target).ok,true,status);v.controllers[0].parameter_count=0;erased(harness(()=>v).component.pluginState(target));}
});
test('indexed holes preserve count and separately incomplete metadata',()=>{
 for(const status of ['parameter_absent','description_unavailable','unsupported_metadata']) {const v=positive();Object.assign(v.controllers[0],{enumeration_status:'incomplete',matched_serialized_name_count:0,parameters:[{index:0,status,description:null}]});incomplete(v,true);v.enumerated_parameter_count=0;assert.equal(harness(()=>v).component.pluginState(target).ok,true);v.parameter_lists_complete=true;erased(harness(()=>v).component.pluginState(target));}
});
test('aggregate descriptor limit returns all 26 counts and no partial lists',()=>{
 const v=observation();v.controllers.forEach((row,i)=>Object.assign(row,{parameter_count:i<3?128:0,parameters:null,enumeration_status:'descriptor_total_limit'}));incomplete(v,true);v.descriptor_budget_exceeded=true;v.reported_parameter_count=384;assert.equal(harness(()=>v).component.pluginState(target).ok,true);v.controllers[25].parameters=[];erased(harness(()=>v).component.pluginState(target));
});
test('numeric carriers enforce exact int64 finite bits and unavailable algebra',()=>{
 const c=harness().controls;
 for(const decimal of ['0','9007199254740993','9223372036854775807','-9223372036854775808'])assert.equal(c.rumbleParametersNumber({status:'observed',variant_tag:1,kind:'integer',decimal}),true);
 for(const decimal of ['-0','01','9223372036854775808','-9223372036854775809'])assert.equal(c.rumbleParametersNumber({status:'observed',variant_tag:1,kind:'integer',decimal}),null);
 for(const bits_hex of ['8000000000000000','0000000000000001'])assert.equal(c.rumbleParametersNumber({status:'observed',variant_tag:2,kind:'float64',bits_hex}),true);
 for(const bits_hex of ['7ff0000000000000','fff8000000000000'])assert.equal(c.rumbleParametersNumber({status:'observed',variant_tag:2,kind:'float64',bits_hex}),null);
 for(const bits_hex of ['80000000','40000000'])assert.equal(c.rumbleParametersNormalized({status:'observed',bits_hex}),true);
 for(const bits_hex of ['7f800000','ffc00000'])assert.equal(c.rumbleParametersNormalized({status:'observed',bits_hex}),null);
 const v=positive(),d=v.controllers[0].parameters[0].description;d.current={status:'unavailable',variant_tag:2,reason:'non_finite'};d.support={raw_number:false,normalized:false};v.current_values_complete=false;assert.equal(harness(()=>v).component.pluginState(target).ok,true);d.support.raw_number=true;erased(harness(()=>v).component.pluginState(target));
});
test('closed shapes reject forged discriminators metadata count and private fields',()=>{
 const mutations=[v=>v.operation='state_capture',v=>v.schema='other',v=>v.atomic_snapshot=true,v=>v.token='PRIVATE',v=>v.instance.object_uid='OTHER',v=>v.controllers.reverse(),v=>v.reported_parameter_count=3,v=>v.controllers[0].parameters[0].index=1,v=>v.controllers[0].parameters[0].description.name='x'.repeat(129),v=>v.controllers[0].parameters[0].description.name='\ud800',v=>v.controllers[0].parameters[0].description.formatter='PRIVATE',v=>v.controllers[0].parameters[0].description.kind=999,v=>v.controllers[0].parameters[0].description.read_only=true,v=>v.serialized_name_coverage_complete=false];
 for(const mutate of mutations){const v=positive();mutate(v);erased(harness(()=>v).component.pluginState(target));}
 const v=positive(),d=v.controllers[0].parameters[0].description;d.kind=999;d.support={raw_number:false,normalized:false};assert.equal(harness(()=>v).component.pluginState(target).ok,true);
});
test('fixed requests reject tokens and non-effect kinds before native access',()=>{
 for(const extra of [{token:'private'},{source:'private'},{path:'private'},{bytes:'private'},{plugin:'Other'},{expected_instance:''},{kind:'instrument'}]) {const h=harness();erased(h.component.pluginState({...target,...extra}));assert.equal(h.calls.length,0);}
});
test('native exceptions malformed replies and late context erase complete and partial evidence',()=>{
 for(const mutate of [c=>c.document='other:1',c=>c.session='other',c=>c.open=false]) {const h=harness((r,c)=>{mutate(c);return positive();});erased(h.component.pluginState(target));}
 for(const reply of [()=>{throw Error('PRIVATE');},()=>'{PRIVATE',()=>'[]',()=>({ok:false,operation:'state_capture',controllers:[{private:'PRIVATE'}]})]) {const v=harness(reply).component.pluginState(target);erased(v);assert.equal(JSON.stringify(v).includes('PRIVATE'),false);}
});
test('outer bridge requires fresh context and removes late metadata even with forged operation',()=>{
 const h=bridgeHarness();let calls=0;h.bridge.component={pluginState(){calls++;h.manager.activeDocument={title:'Other'};return {...positive(),operation:'state_capture'};}};
 assert.equal(h.request({op:'pluginState',args:target}).ok,false);assert.equal(calls,0);const v=h.writeRequest({op:'pluginState',args:target});assert.equal(v.ok,false);assert.equal(v.code,'native_context_lost');assert.equal(v.result,undefined);
});

test('output size is bounded after bridge adaptation without truncation',()=>{
 const v=observation(),d=description();d.name='\x01'.repeat(128);
 for(let i=0;i<2;i++)Object.assign(v.controllers[i],{parameter_count:128,parameters:Array.from({length:128},(_,index)=>({index,status:'observed',description:{...d}}))});
 v.reported_parameter_count=256;v.enumerated_parameter_count=256;
 assert.ok(JSON.stringify(v).length>262144);erased(harness(()=>v).component.pluginState(target));
});
test('unknown kinds unavailable defaults and nonfinite normalized do not imply a writer',()=>{
 const v=positive(),d=v.controllers[0].parameters[0].description;d.kind=999;d.support={raw_number:false,normalized:false};assert.equal(harness(()=>v).component.pluginState(target).ok,true);
 d.kind=4;d.current.decimal='-1';d.default={status:'unavailable',variant_tag:null,reason:'missing'};d.normalized={status:'unavailable',reason:'non_finite'};d.support={raw_number:true,normalized:false};assert.equal(harness(()=>v).component.pluginState(target).ok,true);
});
