import { settingDetails } from './project-settings.js';
const make=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n;};
export function createNetworkEditor(api){
 let selected,field,value,pending=false,revision,view;
 const clear=()=>{pending=false;value=undefined;api.onDraft();};
 const canSelect=node=>{if(selected===node.key)return true;if(pending&&!window.confirm('Discard unapplied network metadata change?'))return false;clear();selected=node.key;field=undefined;return true;};
 function render(node,summary,readonly){
  const target=node.networkTarget;
  const network=target?summary.networks[target.networkIndex]:undefined;
  const data=target?.moduleIndex!==undefined?network.modules[target.moduleIndex]:network;
  view=settingDetails({...node,label:`Network / Communication · ${node.label}`,reason:'File settings only. No device or server connection is made.'});view.classList.add('network-editor');view.prepend(view.querySelector('h2'),view.querySelector('.setting-support'));
  const limitation=make('p','IP, port, station/address, protocol, mode and service parameters are read-only: no validated writer is exposed for these fields.','setting-support');view.append(limitation);
  if(target){
   const module=target.moduleIndex!==undefined;
   const keys=(module?['configName','alias','description']:['name']).filter(k=>data?.[k]!=null);
   const linked=module?Object.fromEntries(['cnet','fenet','xgpd'].map(k=>[k,(summary[k]??[]).filter(x=>data.id!=null&&x.typeCode===data.id)])):{};
   if(Object.values(linked).some(a=>a.length)){const details=settingDetails({label:'Linked communication parameters / services',data:linked,reason:'Decoded values from this file; protocol writes are not implemented.'});details.removeAttribute('id');view.append(details);}
   if(!keys.length){view.append(make('p','No supported existing metadata attribute was decoded.','edit-help'));return view;}
   if(!keys.includes(field))field=keys[0];
   const edit=make('section',undefined,'network-metadata');edit.append(make('h3','Edit existing metadata'));
   const mode=make('select');mode.id='network-field';mode.setAttribute('aria-label','Network metadata field');for(const k of keys){const o=make('option',({name:'Network name',configName:'Configuration name',alias:'Alias',description:'Description'})[k]);o.value=k;mode.append(o);}mode.value=field;
   mode.addEventListener('change',()=>{if(pending&&!window.confirm('Discard unapplied network metadata change?')){mode.value=field;return;}clear();field=mode.value;replace();});edit.append(mode);
   const old=data[field];if(!pending){value=old;revision=api.revision();}
   const patch=()=>({networkIndex:target.networkIndex,module,base:module?data.base:null,slot:module?data.slot:null,field,expectedValue:old,replacement:value});
   const form=make('form');const label=make('label','New value');const input=make('input');input.id='network-replacement';input.type='text';input.maxLength=4096;input.value=value;label.append(input);form.append(label);
   const actions=make('div',undefined,'edit-actions');const apply=make('button','Apply network change');apply.id='apply-network-change';apply.type='submit';const cancel=make('button','Cancel network change');cancel.type='button';cancel.addEventListener('click',()=>{clear();replace();});actions.append(apply,cancel);form.append(actions);
   const message=make('p',undefined,'edit-help');message.setAttribute('role','status');form.append(message);
   function validate(){let error=readonly?'This workspace is read-only.':'';if(!error){try{api.probe(patch());}catch(e){error=String(e);}}message.textContent=error||'Only this existing metadata attribute changes in the downloaded file.';input.disabled=!!error&&(/absent|ambiguous|read-only/.test(error));apply.disabled=!!error||!pending;}
   input.addEventListener('input',()=>{value=input.value;pending=value!==old;api.onDraft();validate();});form.addEventListener('submit',e=>{e.preventDefault();if(pending&&!apply.disabled)api.apply(patch(),revision);});edit.append(form);view.insertBefore(edit,limitation);validate();
  }
  view.addEventListener('keydown',e=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();clear();replace();}});
  function replace(){const previous=view;previous.replaceWith(render(node,summary,readonly));view.querySelector('#network-field')?.focus({preventScroll:true});}
  return view;
 }
 return {render,canSelect,hasDraft:()=>pending,clearDraft:clear,reset(){selected=undefined;field=undefined;clear();}};
}
