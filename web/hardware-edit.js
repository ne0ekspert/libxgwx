// Existing field writers only. Unknown module encodings remain read-only.
const make=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n;};
export function createHardwareEditor(api){
 let selectedBase,selectedSlot=0,action,key,index=0,pending=false,value,revision,view;
 const dirty=()=>pending;
 function clear(){pending=false;value=undefined;api.onDraft();}
 function leave(){return !pending||window.confirm('Discard unapplied hardware change?');}
 function render(base,summary,readonly){
  if(selectedBase!==base.base){if(!leave())return view;clear();selectedBase=base.base;selectedSlot=(summary.hardware.modules.find(m=>m.base===base.base)?.slot)??0;action=undefined;}
  const modules=summary.hardware.modules.filter(m=>m.base===base.base),catalog=api.catalog();
  const entry=m=>{const matches=catalog.filter(c=>c.id===m.id&&c.subType===m.subType);return matches.length===1?matches[0]:undefined;};
  const count=base.slotCount,knownCount=Number.isInteger(count)&&count>=0;
  if(knownCount&&selectedSlot>=count)selectedSlot=0;
  view=make('section',undefined,'project-setting-view hardware-editor');view.id='project-setting-view';view.setAttribute('aria-label',`Base ${base.base} hardware editor`);
  view.append(make('h2',`Base ${base.base} · hardware`),make('p',knownCount?`${count} slots · ${modules.length} configured modules`:'Slot count is not decoded. Only declared module positions are shown.','edit-help'));
  const baseControls=make('div',undefined,'hardware-base-controls');baseControls.append(make('h3','Base size · slot count'));
  const baseChoice=make('select');baseChoice.id='hardware-base-size';baseChoice.setAttribute('aria-label','Base size · slot count');
  if(![4,6,8,10,12].includes(count)){const n=make('option',knownCount?`${count} (current)`:'Not decoded');n.value=String(count);baseChoice.append(n);}
  for(const n of [4,6,8,10,12]){const option=make('option',`${n} slots`);option.value=String(n);baseChoice.append(option);}baseChoice.value=action==='slotCount'&&pending?value:String(count);
  let baseError=readonly?'This workspace is read-only.':!knownCount?'Base slot count is absent; adding this field is not supported.':'';
  if(!baseError){try{api.probe({operation:'slotCount',base:base.base,slot:null,expectedValue:String(count),replacement:String([4,6,8,10,12].includes(count)?count:4),key:null,index:null});}catch(e){baseError=String(e);}}
  baseChoice.disabled=!!baseError;baseChoice.addEventListener('change',()=>{if(!leave()){baseChoice.value=String(count);return;}clear();action='slotCount';value=baseChoice.value;pending=value!==String(count);revision=api.revision();api.onDraft();replace();view.querySelector('#hardware-base-size')?.focus();});
  if(action!=='slotCount'){const label=make('label','Configured slots');label.append(baseChoice);baseControls.append(label,make('p',baseError||'Choose a size to review and apply. Occupied modules must fit.','edit-help'));}
  view.append(baseControls);
  const slots=make('div',undefined,'hardware-rack');slots.setAttribute('role','listbox');slots.setAttribute('aria-label','Base slots');
  const numbers=knownCount?Array.from({length:Math.min(count,64)},(_,i)=>i):[...new Set(modules.map(m=>m.slot).filter(Number.isInteger))];
  const unknownSpan=modules.some(m=>!entry(m));
  let selectedModule;
  numbers.forEach((slot,i)=>{
   const module=modules.find(m=>m.slot===slot)||modules.find(m=>entry(m)&&m.slot<slot&&slot<m.slot+entry(m).slotSpan);
   if(slot===selectedSlot)selectedModule=module;
   const reserved=module&&module.slot!==slot;
   const label=module?(reserved?`Reserved by slot ${module.slot}`:entry(module)?.model||module.name||'Unidentified module'):(unknownSpan?'No starting module · occupancy unverified':'Empty');
   const button=make('button',undefined,`hardware-slot${slot===selectedSlot?' selected':''}${module?' occupied':''}`);button.type='button';button.dataset.hardwareSlot=slot;button.setAttribute('role','option');button.setAttribute('aria-selected',String(slot===selectedSlot));button.setAttribute('aria-label',`Slot ${slot}: ${label}`);button.tabIndex=slot===selectedSlot?0:-1;button.append(make('small',`SLOT ${slot}`),make('strong',label));
   const select=next=>{if(!leave())return false;clear();selectedSlot=next;action=undefined;replace();return true;};
   button.addEventListener('click',()=>select(slot));
   button.addEventListener('keydown',e=>{if(['ArrowDown','ArrowUp','ArrowLeft','ArrowRight','Home','End'].includes(e.key)){e.preventDefault();const next=e.key==='Home'?0:e.key==='End'?numbers.length-1:Math.max(0,Math.min(numbers.length-1,i+(['ArrowDown','ArrowRight'].includes(e.key)?1:-1)));if(select(numbers[next]))view.querySelector(`[data-hardware-slot="${numbers[next]}"]`)?.focus();}else if(e.key==='Enter'){e.preventDefault();view.querySelector('#hardware-field')?.focus();}});slots.append(button);
  });view.append(slots);
  if(knownCount&&count>64)view.append(make('p','Only the first 64 declared slots are displayed.','edit-help'));
  const module=selectedModule,model=module&&entry(module),startSlot=module?.slot??selectedSlot;
  const info=make('div',undefined,'hardware-selected');info.append(make('h3',`Slot ${selectedSlot}${module?` · ${model?.model||module.name||'Module'}`:' · Empty'}`));
  if(!module)info.append(make('p','No module starts here. Module insertion is not available in this editor.','edit-help'));
  else if(module.slot!==selectedSlot)info.append(make('p',`This position belongs to the module starting at slot ${module.slot}. Changes apply to that module.`,'edit-help'));
  let selections=[],optionError='';
  if(module&&model?.options.length){try{selections=api.options(base.base,startSlot);}catch(e){optionError=String(e);}}
  const choices=[];
  if(knownCount)choices.push(['slotCount','Base slot count']);
  if(module?.comment!=null)choices.push(['comment','Module comment']);
  if(module?.inputFilterRaw!=null)choices.push(['inputFilter','Input filter']);
  if(selections.length)choices.push(['option','Verified module option']);
  const unsupported=model?.visibleOptions.filter(o=>!model.options.some(p=>p.key===o.key))??[];
  if(unsupported.length)info.append(make('p',`Read-only options: ${unsupported.map(o=>o.label).join(', ')}. Their write encoding is not verified.`,'edit-help'));
  if(optionError)info.append(make('p',`Module options are read-only. ${optionError}`,'edit-help'));
  info.append(make('p','Model, module identity and base/slot position are read-only. Model replacement, insertion and deletion require a separate validated editing flow.','edit-help'));
  if(!choices.length){info.append(make('p','No supported editable field was decoded for this selection.','edit-help'));view.append(info);return view;}
  if(!choices.some(([k])=>k===action))action=module?.inputFilterRaw!=null?'inputFilter':module?.comment!=null?'comment':'slotCount';
  const mode=make('select');mode.id='hardware-field';mode.setAttribute('aria-label','Hardware field');for(const[k,label]of choices){const option=make('option',label);option.value=k;mode.append(option);}mode.value=action;
  mode.addEventListener('change',()=>{if(!leave()){mode.value=action;return;}clear();action=mode.value;replace();});info.append(mode);
  let old,allowed=[];
  if(action==='slotCount'){old=String(count);allowed=[4,6,8,10,12].map(n=>({value:String(n),label:String(n)}));}
  else if(action==='comment')old=module.comment;
  else if(action==='inputFilter'){old=String(module.inputFilterRaw);allowed=[0,1,3,5,10,20,70,100].map(n=>({value:String(n),label:n?`${n} ms`:'Default'}));}
  else {
   const opts=model.options;
   if(!opts.some(o=>o.key===key))key=opts[0].key;
   const optionSelect=make('select');optionSelect.setAttribute('aria-label','Module option');for(const o of opts){const n=make('option',o.label);n.value=o.key;optionSelect.append(n);}optionSelect.value=key;
   optionSelect.addEventListener('change',()=>{if(!leave()){optionSelect.value=key;return;}clear();key=optionSelect.value;index=0;replace();});info.append(optionSelect);
   const option=opts.find(o=>o.key===key);index=Math.min(index,option.count-1);
   if(option.count>1){const channel=make('select');channel.setAttribute('aria-label','Option channel');for(let i=0;i<option.count;i++){const n=make('option',`${option.scope} ${i+1}`);n.value=i;channel.append(n);}channel.value=index;channel.addEventListener('change',()=>{if(!leave()){channel.value=index;return;}clear();index=Number(channel.value);replace();});info.append(channel);}
   old=String(selections.find(s=>s.key===key&&s.index===index)?.value);allowed=option.values.map(v=>({value:String(v.value),label:v.label}));
  }
  if(!pending){value=old;revision=api.revision();}
  const patch=()=>({operation:action,base:base.base,slot:action==='slotCount'?null:startSlot,expectedValue:action==='slotCount'||action==='comment'?old:module.details,replacement:value,key:action==='option'?key:null,index:action==='option'?index:null});
  const form=make('form');form.id='hardware-change-form';const label=make('label',action==='comment'?'Module comment':'New value');const field=make(action==='comment'?'input':'select');field.id=action==='slotCount'?'hardware-base-size':'hardware-replacement';field.setAttribute('aria-label',label.textContent);if(action==='comment'){field.type='text';field.maxLength=4096;}else{if(!allowed.some(c=>c.value===old)){const n=make('option',`${old} (current)`);n.value=old;field.append(n);}for(const c of allowed){const n=make('option',c.label);n.value=c.value;field.append(n);}}field.value=value;label.append(field);form.append(label);
  const message=make('p',undefined,'edit-help');message.setAttribute('role','status');const actions=make('div',undefined,'edit-actions');const apply=make('button','Apply hardware change');apply.id='apply-hardware-change';apply.type='submit';const cancel=make('button','Cancel hardware change');cancel.type='button';cancel.addEventListener('click',()=>{clear();replace();});actions.append(apply,cancel);form.append(actions,message);
  function validate(){let error=readonly?'This workspace is read-only.':'';if(!error){try{api.probe(patch());}catch(e){error=String(e);}}message.textContent=error||'Changes apply to the downloaded workspace copy.';apply.disabled=!!error||!pending;field.disabled=readonly||/CPU type|catalog.*mismatch|ambiguous|field is absent|missing.*attribute/i.test(error);}
  field.addEventListener(action==='comment'?'input':'change',()=>{value=field.value;pending=value!==old;api.onDraft();validate();});
  form.addEventListener('submit',e=>{e.preventDefault();if(!pending||apply.disabled)return;api.apply(patch(),revision);});if(action==='slotCount'){baseControls.replaceChildren(make('h3','Base size · slot count'));label.firstChild.textContent='Configured slots';field.setAttribute('aria-label','Base size · slot count');apply.textContent='Apply base size';baseControls.append(form);}else info.append(form);view.append(info);validate();
  view.addEventListener('keydown',e=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();clear();replace();view.querySelector(`[data-hardware-slot="${selectedSlot}"]`)?.focus();}});
  function replace(){const previous=view;const next=render(base,summary,readonly);previous.replaceWith(next);next.querySelector(action==='slotCount'?'#hardware-base-size':'#hardware-field')?.focus({preventScroll:true});}
  return view;
 }
 return {render,canSelectBase(base){if(base===selectedBase)return true;if(!leave())return false;clear();return true;},hasDraft:dirty,canLeave:leave,clearDraft:clear,reset(){selectedBase=undefined;selectedSlot=0;action=undefined;clear();}};
}
