import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const nativePositive=JSON.parse(readFileSync(new URL('../fixtures/external-menu-native-positive.json',import.meta.url),'utf8'));
import {asset,bridgeHarness} from './host.mjs';
const instance={object_uid:'12345678-1234-1234-1234-123456789ABC',class_uid:'889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4',native_class:'AudioEffect',object_id:'External Effect Stereo'};
const target={channel:'Scratch',slot:0,plugin:'External Effect Stereo',expected_instance:instance.object_uid,port:'send'};
const row=(path=[0],submenu=false)=>({path,name:null,title:'Main',category:null,checked:false,enabled:true,has_submenu:submenu,handler_present:true,kind:submenu?'submenu':'opaque'});
function menu(request={port:'send'}) {const value=structuredClone(nativePositive);value.port=request.port;value.parameter={name:request.port==='send'?'sendPort':'receivePort',tag:request.port==='send'?102:103};return value;}
function harness(reply=menu){const calls=[],targets=[],context={open:true,document:'menu:1',session:'menu'};const service={capabilities(){return JSON.stringify({ok:true,api:'fender-mcp-native-v1'});},request(device,body){const request=JSON.parse(body);calls.push(request);const r=reply(request,context);return typeof r==='string'?r:JSON.stringify(r);}};const scope=vm.createContext({include_file(){},memberTypes(){return{};},PreSonus:{ControlSurfaceComponent:class{},ParamID:{kLabel:'label',kInsertName:'insertName'}},Host:{Objects:{getObjectByName(){return service;}}}});vm.runInContext(asset('bridge/NativePluginControls.js'),scope);vm.runInContext(asset('bridge/BridgeComponent.js'),scope);const component=vm.runInContext('createBridgeComponent()',scope);component.bridge={requestContext:{...context},context(){return{...context};}};component.pluginOf=args=>{targets.push(args);return{name:args.plugin,device:{}};};return{component,calls,targets,context,controls:vm.runInContext('new NativePluginControls({})',scope)};}
function noMenu(value){for(const key of ['rows','instance','source','scope','port','parameter','coverage','lifecycle','menu_class','menu_phase','selection_authority','selection_calls','persistence_verified','audio_verified'])assert.equal(key in value,false,key);}
test('External menu routes one fixed native reader without setter or selector action',()=>{for(const port of ['send','return']){const h=harness();const result=h.component.externalPortMenu({...target,port});assert.equal(result.ok,true);assert.equal(result.native_outcome,'observed');assert.equal(result.selection_authority,false);assert.equal(result.selection_calls,0);assert.deepEqual(h.calls,[{operation:'external_port_menu',expected_instance:instance.object_uid,port}]);assert.deepEqual(structuredClone(h.targets),[{channel:'Scratch',slot:0,plugin:target.plugin}]);}});
test('External menu target is closed before plugin resolution or dispatch',()=>{for(const extra of [{channel:''},{channel:'x'.repeat(257)},{channel:'bad\0name'},{slot:16},{slot:true},{slot:0.5},{plugin:'Chorder'},{expected_instance:instance.object_uid.toLowerCase()},{expected_instance:'00000000-0000-0000-0000-000000000000'},{port:'select'},...['kind','track','source','path','param','action','command','max_rows','token','value'].map(k=>({[k]:'unchecked'}))]){const h=harness();h.component.pluginOf=()=>{throw Error('must not resolve');};const r=h.component.externalPortMenu({...target,...extra});assert.equal(r.ok,false);assert.equal(r.outcome,'not_applied');noMenu(r);assert.equal(h.calls.length,0);}});
test('External menu validates complete hierarchy and Unicode byte budgets',()=>{const h=harness(request=>{const r=menu(request);r.rows=[row([0],true),row([0,0]),row([0,1]),row([1])];r.coverage.row_count=4;return r;});assert.equal(h.component.externalPortMenu(target).ok,true);for(const mutate of [r=>{r.rows[0].path=[0,0];},r=>{r.rows[0].path=[true];},r=>{r.rows[0].path=[256];},r=>{r.rows[0].path=[];},r=>{r.rows[0].title='한'.repeat(342);},r=>{r.rows[0].title='\ud800';},r=>{r.rows[0].kind='parameter_value';},r=>{r.rows[0].checked=0;},r=>{r.rows[0].extra=true;},r=>{r.rows=Array.from({length:33},(_,i)=>({...row([i]),title:'x'.repeat(1024)}));r.coverage.row_count=33;}]){const h=harness(request=>{const r=menu(request);mutate(r);return r;});const r=h.component.externalPortMenu(target);assert.equal(r.outcome,'unknown');noMenu(r);assert.equal(h.calls.length,1);}});
test('External menu refuses forged authority and incomplete cleanup',()=>{for(const mutate of [r=>{r.selection_authority=true;},r=>{r.selection_calls=1;},r=>{r.audio_verified=true;},r=>{r.persistence_verified=true;},r=>{r.menu_phase='build_dispatched';},r=>{r.lifecycle.builder_final_release_count=1;},r=>{r.lifecycle.root_menu_final_release_count=1;},r=>{r.lifecycle.post_cleanup_binding_checked=false;},r=>{r.instance.class_uid='00000000-0000-0000-0000-000000000000';},r=>{r.parameter.tag=103;},r=>{r.port='return';},r=>{r.coverage.complete=false;},r=>{r.token='fake';},r=>{r.value_semantics='atomic';},r=>{r.source='dsp';}]){const h=harness(request=>{const r=menu(request);mutate(r);return r;});const r=h.component.externalPortMenu(target);assert.equal(r.outcome,'unknown');noMenu(r);}});
test('External menu distinguishes known preconstruct refusals from uncertain dispatched work',()=>{for(const phase of [undefined,'prepared']){const h=harness(()=>({ok:false,outcome:'not_applied',error:{code:'unavailable',message:'No editor'},...(phase?{menu_phase:phase}:{})}));const r=h.component.externalPortMenu(target);assert.equal(r.outcome,'not_applied');noMenu(r);}for(const reply of [()=>({ok:false,outcome:'unknown',menu_phase:'construct_dispatched',error:{code:'uncertain',message:'Lease changed'}}),()=>({ok:false,outcome:'not_applied',menu_phase:'build_dispatched',error:{code:'uncertain',message:'Lease changed'}}),()=>({ok:false,outcome:'not_applied',rows:[],error:{code:'forged',message:'bad'}}),()=>'{bad',()=>{throw Error('late');}]){const h=harness(reply);const r=h.component.externalPortMenu(target);assert.equal(r.outcome,'unknown');noMenu(r);assert.equal(h.calls.length,1);}});
test('External menu late document and session changes erase all menu evidence',()=>{for(const mutate of [c=>{c.document='other:1';},c=>{c.session='other';},c=>{c.open=false;}]){const h=harness((request,context)=>{mutate(context);return menu(request);});const r=h.component.externalPortMenu(target);assert.equal(r.outcome,'unknown');noMenu(r);}});
test('Direct menu requests cannot add source, parameter names or selection paths',()=>{const h=harness();for(const extra of [{source:'controller'},{param:'sendPort'},{action:'select'},{path:[0]},{expected_instance:''},{port:true}]){const r=h.controls.request({},{operation:'external_port_menu',expected_instance:instance.object_uid,port:'send',...extra});assert.equal(r.outcome,'not_applied');noMenu(r);}assert.equal(h.calls.length,0);});
test('BridgeCore requires both leases and removes a late successful menu',()=>{const h=bridgeHarness();let calls=0;h.bridge.component={externalPortMenu(){calls++;h.manager.activeDocument={title:'Changed'};return menu();}};const missing=h.request({op:'externalPortMenu',args:target});assert.equal(missing.ok,false);assert.equal(calls,0);const r=h.writeRequest({op:'externalPortMenu',args:target});assert.equal(calls,1);assert.equal(r.ok,false);assert.equal(r.code,'native_context_lost');assert.equal(r.result,undefined);});

test('External menu requires the formal per-text byte coverage bound',()=>{
    assert.equal(nativePositive.coverage.max_text_bytes_each,1024);
    assert.equal(harness().component.externalPortMenu(target).ok,true);
    for(const mutate of [r=>{delete r.coverage.max_text_bytes_each;},...[null,true,'1024',1023,1025].map(v=>r=>{r.coverage.max_text_bytes_each=v;}),r=>{r.coverage.unchecked=1024;}]){
        const h=harness(request=>{const r=menu(request);mutate(r);return r;});
        const result=h.component.externalPortMenu(target);
        assert.equal(result.outcome,'unknown');noMenu(result);assert.equal(h.calls.length,1);
    }
});

function provenanceMenu(request) {
    const value = menu(request);
    value.rows = [
        {path:[0],name:'0',title:'없음',category:'Param',checked:true,enabled:true,has_submenu:false,handler_present:true,kind:'opaque'},
        {path:[1],name:'1',title:'메인 - 1 + 2',category:'Param',checked:false,enabled:true,has_submenu:false,handler_present:true,kind:'opaque'},
        {path:[2],name:'',title:'-',category:'',checked:false,enabled:true,has_submenu:false,handler_present:false,kind:'opaque'},
        {path:[3],name:'Setup',title:'오디오 I/O 설정',category:'Console',checked:false,enabled:true,has_submenu:false,handler_present:true,kind:'opaque'}
    ];
    value.coverage.row_count = 4;
    value.provenance = {leaves:[0,1].map(i=>({path:[i],handler_matches_builder:true,data_handler_matches_builder:false,data_builder_matches_builder:false,data_parameter_is_selected:false,original_is_selected:[false,true],original_end:{kind:'self_reference'}})),menu_data_typed_values_observed:false,selection_authority:false,selection_calls:0,proof_scope:'held_target_sequential_read_only',read_only_intent:true,atomic_snapshot:false,continuous_active_mixer_membership_verified:false};
    return value;
}
function provenanceHarness(reply=provenanceMenu) {
    const h=harness(reply), held={name:target.plugin,device:{},component:{},element:{}};
    h.component.pluginOf=()=>held;
    return {...h,held};
}
test('Provenance default false keeps the exact old request and closed true mode uses one native call',()=>{
    for (const option of [{},{provenance:false}]) {
        const h=harness();assert.equal(h.component.externalPortMenu({...target,...option}).ok,true);
        assert.deepEqual(h.calls,[{operation:'external_port_menu',expected_instance:instance.object_uid,port:'send'}]);
    }
    const h=provenanceHarness();const result=h.component.externalPortMenu({...target,provenance:true});
    assert.equal(result.ok,true);assert.equal(result.provenance.menu_data_typed_values_observed,false);
    assert.deepEqual(h.calls,[{operation:'external_port_menu',expected_instance:instance.object_uid,port:'send',provenance:true}]);
});
test('Provenance wrong class, return and nonboolean intent refuse before resolution',()=>{
    for(const extra of [{plugin:'External Effect Mono',provenance:true},{port:'return',provenance:true},...[1,0,null,'true',{}].map(provenance=>({provenance}))]) {
        const h=provenanceHarness();h.component.pluginOf=()=>{throw Error('must not resolve');};
        assert.equal(h.component.externalPortMenu({...target,...extra}).outcome,'not_applied');assert.equal(h.calls.length,0);
    }
});
test('Provenance rejects typed aliases, new authority, malformed chains and any changed fixed row',()=>{
    const mutations=[r=>{r.provenance.leaves[0].handler_matches_builder=1;},r=>{r.provenance.leaves[0].original_is_selected=[0];},r=>{r.provenance.leaves[0].original_end={kind:'cycle',index:false};},r=>{r.provenance.leaves[0].original_end={kind:'cycle',index:1};},r=>{r.provenance.leaves[0].original_end={kind:'limit'};},r=>{r.provenance.leaves[0].pointer=1;},r=>{r.provenance.menu_data_typed_values_observed=true;},r=>{r.provenance.selection_calls=1;},r=>{r.provenance.leaves.reverse();},r=>{r.rows[3].title='Changed';},r=>{r.rows[2].handler_present=true;},r=>{r.rows[0].checked=false;},r=>{delete r.provenance;},r=>{r.instance.object_id='Other';}];
    for(const mutate of mutations){const h=provenanceHarness(req=>{const r=provenanceMenu(req);mutate(r);return r;});const result=h.component.externalPortMenu({...target,provenance:true});assert.equal(result.outcome,'unknown');assert.equal('provenance' in result,false);noMenu(result);assert.equal(h.calls.length,1);}
});
test('Provenance fresh route replacement rejects retained old target metadata',()=>{
    for(const field of ['device','component','element']){
        const h=provenanceHarness();let reads=0;
        h.component.pluginOf=()=> ++reads===1 ? h.held : {...h.held,[field]:{}};
        const result=h.component.externalPortMenu({...target,provenance:true});assert.equal(result.outcome,'unknown');noMenu(result);assert.equal('provenance' in result,false);
    }
});
test('Provenance late context loss preserves unknown and exposes no provenance',()=>{
    const h=provenanceHarness((request,context)=>{context.document='replaced';return provenanceMenu(request);});
    const r=h.component.externalPortMenu({...target,provenance:true});assert.equal(r.outcome,'unknown');assert.equal('provenance' in r,false);noMenu(r);
});

function realMappedProvenanceHarness(callback=()=>{}) {
    const h=harness((request, context)=>{callback(state, request, context);return provenanceMenu(request);});
    h.component.pluginOf=Object.getPrototypeOf(h.component).pluginOf;
    const device={findParameter(){}}, slot={isConnected(){return state.slotConnected;},getParamValue(){return target.plugin;},getPlugInClassName(){return target.plugin;},component:{find(name){return name==='Device'?device:null;}}};
    const channel={isConnected(){return state.connected;},getParamValue(){return 'Scratch';},find(name){return name==='inserts'?{getElement(index){return index===0?state.slot:null;}}:null;}};
    const state={present:true,connected:true,slotConnected:true,slot,channel,getElementCalls:0};
    const bank={getElement(index){state.getElementCalls++;return index===0&&state.present?state.channel:null;}};
    h.component.hostComponent={model:{root:{find(name){return name==='mixer'?{find(child){return child==='channels'?bank:null;}}:null;}}}};
    return {...h,state,originalSlot:slot,run:()=>h.component.externalPortMenu({...target,provenance:true})};
}
test('Actual mapped resolver stable target publishes only a bracketed sequential read-only scope',()=>{
    const h=realMappedProvenanceHarness(),r=h.run();assert.equal(r.ok,true);assert.equal(h.calls.length,1);assert.equal(h.state.getElementCalls,512);
    assert.equal(r.provenance.proof_scope,'bracketed_current_mapped_bank_sequential_read_only');assert.equal(r.provenance.read_only_intent,true);
    assert.equal(r.provenance.atomic_snapshot,false);assert.equal(r.provenance.continuous_active_mixer_membership_verified,false);assert.equal(r.selection_authority,false);
});
test('Actual mapped resolver removal before or after native refuses publication',()=>{
    const pre=realMappedProvenanceHarness();pre.state.present=false;assert.equal(pre.run().outcome,'not_applied');assert.equal(pre.calls.length,0);
    const post=realMappedProvenanceHarness(state=>{state.present=false;});const r=post.run();assert.equal(r.outcome,'unknown');assert.equal(post.calls.length,1);assert.equal('provenance' in r,false);noMenu(r);
});
test('Actual mapped resolver disconnected channel or slot before and after native refuses publication',()=>{
    for (const key of ['connected','slotConnected']) {
        const pre=realMappedProvenanceHarness();pre.state[key]=false;assert.equal(pre.run().outcome,'not_applied');assert.equal(pre.calls.length,0);
        const post=realMappedProvenanceHarness(state=>{state[key]=false;});const r=post.run();assert.equal(r.outcome,'unknown');assert.equal(post.calls.length,1);assert.equal('provenance' in r,false);noMenu(r);
    }
});
test('Actual mapped resolver same-name class replacement after native rejects old held metadata',()=>{
    const h=realMappedProvenanceHarness(state=>{state.slot={...state.slot,component:{find(){return {findParameter(){}};}}};});
    const r=h.run();assert.equal(r.outcome,'unknown');assert.equal(h.calls.length,1);assert.equal('provenance' in r,false);noMenu(r);
});
test('Actual mapped resolver transient detach and reattach remains explicitly unobserved by bracketing',()=>{
    let transient=false;const h=realMappedProvenanceHarness(state=>{state.present=false;transient=true;state.present=true;});
    const r=h.run();assert.equal(transient,true);assert.equal(r.ok,true);assert.equal(r.provenance.atomic_snapshot,false);assert.equal(r.provenance.continuous_active_mixer_membership_verified,false);
});
test('Native scope cannot forge bridge bracketing or continuous membership and atomicity',()=>{
    for(const [key,value] of [['proof_scope','bracketed_current_mapped_bank_sequential_read_only'],['proof_scope','atomic'],['read_only_intent',false],['read_only_intent',1],['atomic_snapshot',true],['atomic_snapshot',0],['continuous_active_mixer_membership_verified',true],['continuous_active_mixer_membership_verified',0]]) {
        const h=provenanceHarness(req=>{const r=provenanceMenu(req);r.provenance[key]=value;return r;});const r=h.component.externalPortMenu({...target,provenance:true});assert.equal(r.outcome,'unknown');assert.equal('provenance' in r,false);noMenu(r);
    }
});
