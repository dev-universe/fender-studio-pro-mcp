import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
import {asset} from './host.mjs';
const positive=JSON.parse(readFileSync(new URL('../fixtures/cv-port-structure-native-positive.json',import.meta.url),'utf8'));
const target={kind:'instrument',track:'Scratch',plugin:'CV Instrument',expected_instance:positive.instance.object_uid,action:'inspect_cv_port_structure'};
function harness(reply=()=>structuredClone(positive)) {
    const calls=[],context={open:true,document:'cv:1',session:'cv'};
    const service={capabilities(){return JSON.stringify({ok:true,api:'fender-mcp-native-v1'});},request(device,body){const request=JSON.parse(body);calls.push(request);const result=reply(request,context);return typeof result==='string'?result:JSON.stringify(result);}};
    const scope=vm.createContext({include_file(){},memberTypes(){return{};},PreSonus:{ControlSurfaceComponent:class{},ParamID:{kLabel:'label',kInsertName:'insertName'}},Host:{Objects:{getObjectByName(){return service;}}}});
    vm.runInContext(asset('bridge/NativePluginControls.js'),scope);vm.runInContext(asset('bridge/BridgeComponent.js'),scope);
    const component=vm.runInContext('createBridgeComponent()',scope);
    component.bridge={requestContext:{...context},context(){return{...context};}};
    component.pluginOf=args=>({name:args.plugin,device:{}});
    return {component,calls,context};
}
function noObservation(result){for(const key of ['children','instance','source','parent_before','parent_after','parent_details','limits','token']) assert.equal(key in result,false,key);}
test('CV structure uses one dedicated DSP lifecycle request and no capsule',()=>{
    const h=harness(),result=h.component.pluginState(target);assert.equal(result.ok,true);assert.equal(result.native_outcome,'observed');assert.equal(result.mutation_capability,false);
    assert.deepEqual(h.calls,[{operation:'cv_port_structure_inspect',expected_instance:target.expected_instance}]);
});
test('CV structure rejects irrelevant fields including explicit null before native dispatch',()=>{
    for(const key of ['token','channel','slot','source','port','param','name','tag','index','iid','method','path','depth','offset','limit','script']) for(const value of [null,0,'unchecked']) {
        const h=harness();h.component.pluginOf=()=>{throw Error('must not resolve');};const result=h.component.pluginState({...target,[key]:value});assert.equal(result.outcome,'not_applied',key);assert.equal(h.calls.length,0);noObservation(result);
    }
    for(const change of [{kind:'effect'},{kind:null},{track:' '},{track:'bad\0name'},{plugin:'CV'},{expected_instance:target.expected_instance.toLowerCase()},{expected_instance:'00000000-0000-0000-0000-000000000000'}]) {
        const h=harness();const result=h.component.pluginState({...target,...change});assert.equal(result.outcome,'not_applied');assert.equal(h.calls.length,0);
    }
});
test('CV structure zero and bounded overlimit observations preserve false authorities',()=>{
    const h=harness(()=>({...structuredClone(positive),child_count:33,status:'child_limit',observation_complete:false}));const result=h.component.pluginState(target);assert.equal(result.ok,true);assert.equal(result.children.length,0);assert.equal(result.observation_complete,false);
});
test('CV structure malformed transport and unfinished lifecycle are unknown without retry',()=>{
    for(const reply of [()=>'{bad',()=>null,()=>{throw Error('transport lost');},()=>({...structuredClone(positive),phase:'prepared'}),()=>({...structuredClone(positive),cleanup_calls:2}),()=>({...structuredClone(positive),selection_authority:true}),()=>({...structuredClone(positive),token:'handle'}),()=>({...structuredClone(positive),child_count:-1})]) {
        const h=harness(reply);const result=h.component.pluginState(target);assert.equal(result.outcome,'unknown');assert.equal(h.calls.length,1);noObservation(result);
    }
});
test('CV structure only known before-prepare refusal preserves not_applied',()=>{
    for(const phase of ['before_prepare','prepare_dispatched','prepared','cleanup_dispatched','closed_checked','unknown']) {
        const h=harness(()=>({ok:false,outcome:'not_applied',phase,error:{code:'unavailable',message:'No structure'}}));const result=h.component.pluginState(target);assert.equal(result.outcome,phase==='before_prepare'?'not_applied':'unknown');assert.equal(h.calls.length,1);noObservation(result);
    }
});
test('CV structure late document and session invalidation erase all child evidence',()=>{
    for(const change of [c=>{c.document='other:1';},c=>{c.session='other';},c=>{c.open=false;}]) {
        const h=harness((request,context)=>{change(context);return structuredClone(positive);});const result=h.component.pluginState(target);assert.equal(result.outcome,'unknown');assert.equal(h.calls.length,1);noObservation(result);
    }
});
test('CV nested children retain nonnumeric typed metadata and reject forged handles or malformed carriers',()=>{
    for(const [variant_tag,data] of [[0,{kind:'empty'}],[3,{kind:'text',value:'nested label'}],[4,{kind:'opaque_object',present:true}],[7,{kind:'unsupported',tag:7}]]) {
        const child={...structuredClone(positive.parent_before),name:'nested',kind:'Unknown(11)',value:{variant_tag,data},normalized:{kind:'not_applicable'}};
        const bytes=Buffer.byteLength(child.name)+Buffer.byteLength(child.display)+(data.kind==='text'?Buffer.byteLength(data.value):0);
        const value={...structuredClone(positive),child_count:1,children:[child],retained_text_bytes:bytes};
        assert.equal(harness(()=>value).component.pluginState(target).ok,true);
        for(const mutate of [v=>{v.children[0].pointer='handle';},v=>{v.children[0].value.data.pointer='handle';},v=>{v.children[0].support={raw_number:true};},v=>{v.retained_text_bytes++;},v=>{v.children[0].normalized.bits_hex='bad';}]) {
            const forged=structuredClone(value);mutate(forged);const result=harness(()=>forged).component.pluginState(target);assert.equal(result.outcome,'unknown');noObservation(result);
        }
    }
});
