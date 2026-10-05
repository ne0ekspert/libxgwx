import { globalVariablesView } from './global-variables.js';
import { projectSettings, settingsTree, settingDetails } from "./project-settings.js";
// Independent browser UI for existing IEC records. No placement or new logic.
const contacts = ['NO','NC','RISING','FALLING','NEGATED_RISING','NEGATED_FALLING'];
const coils = ['OUTPUT','INVERSE','SET','RESET','RISING','FALLING'];
const kinds = new Map([
 ['Normally open contact variable','NO'],['Normally closed contact variable','NC'],
 ['Rising-edge contact variable','RISING'],['Falling-edge contact variable','FALLING'],
 ['Negated rising-edge contact variable','NEGATED_RISING'],['Negated falling-edge contact variable','NEGATED_FALLING'],
 ['Output coil variable','OUTPUT'],['Inverse output coil variable','INVERSE'],['Set coil variable','SET'],
 ['Reset coil variable','RESET'],['Rising-edge coil variable','RISING'],['Falling-edge coil variable','FALLING']
]);
export function ladderTargets(summary, programIndex) {
 const ladder = (summary.ladder ?? []).find(p => p.programIndex === programIndex);
 if (!ladder || ladder.projectType !== 2 || ladder.version !== 'LD VER 1.1') return [];
 const symbols = (summary.localVariables?.[programIndex] ?? []).filter(v => !v.isInstance && v.dataTypeCode >= 1 && v.dataTypeCode <= 19);
 return (ladder.sourceStrings ?? []).flatMap((source, ordinal) => {
  const kind = kinds.get(source.iecElementKind);
  let output = false, mask = 1, block, pin;
  if (kind) output = source.iecElementKind.includes('coil');
  else if (source.isIecFunctionOperand) {
   const link = (ladder.iecFunctionOperandLinks ?? []).find(l => l.recordOffset === source.iecRecordOffset);
   block = (ladder.iecFunctions ?? []).find(b => b.recordOffset === link?.targetRecordOffset);
   pin = block?.pins.find(p => !p.isControl && p.referenceOrdinal === link.ordinal && (p.direction === 'output') === link.isOutput);
   if (!link || !block || !pin || link.isArray) return [];
   mask = link.dataTypeMask; output = link.isOutput;
  } else return [];
  const choices = symbols.filter(v => (!output || v.storageClass !== 'I') && (mask & 2 ** (v.dataTypeCode - 1))).map(v => ({name:v.name,type:v.dataType,storage:v.storageClass}));
  return [{key:`${programIndex}:${ordinal}`, programIndex, ordinal, offset:source.offset,
   expectedKind:kind ?? block.name, value:source.value, row:source.iecRowIndex ?? 0, group:source.iecGroupIndex,
   position:source.iecPosition, cellOffset:kind ? source.iecRecordOffset : block.recordOffset, category:kind ? (output ? 'coil' : 'contact') : 'function', output,
   label:kind ? `${output ? 'Coil' : 'Contact'} ${kind}` : `${block.name}.${pin.name} · ${output ? 'output' : 'input'} · ${pin.dataType ?? pin.typeExpression ?? 'typed'}`,
   choices, kindChoices:kind ? (output ? coils : contacts) : []}];
 });
}
export function createLadderEditor(api) {
 let selectedProgram = 0, selectedKey, targets = [], target, action = 'operand', draft = false, capturedRevision;
 let section, inspector, selectedSettingKey, settingsSelect;
 const el = (tag, text, cls) => {const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(cls)node.className=cls;return node;};
 function discard() {draft=false;api.onDraft();}
 function canLeave() {return !draft || window.confirm('Discard unapplied ladder change?');}
 function render(summary, readonly) {
  section=el('section',undefined,'edit-card ladder-edit');section.id='ladder-edit';
  const title=el('div',undefined,'workspace-title');title.append(el('h3','PLC program editor'),el('span','Local copy'));section.append(title);
  const layout=el('div',undefined,'ladder-edit-layout');
  const project=el('details',undefined,'workspace-project');project.open=!window.matchMedia('(max-width: 900px)').matches;
  const projectTitle=el('summary','Project');project.append(projectTitle);
  const projectBody=el('div',undefined,'workspace-project-body');project.append(projectBody);
  const nav=el('nav',undefined,'workspace-project-nav');nav.setAttribute('aria-label','Workspace project');
  nav.append(el('strong','Scan programs'));
  (summary.programs ?? []).forEach((p,i)=>{const button=el('button',p.name ?? `Program ${i+1}`,'project-program');button.type='button';button.setAttribute('aria-label',`Open ladder ${p.name ?? i+1}`);button.setAttribute('aria-current',i===selectedProgram?'true':'false');button.addEventListener('click',()=>{if(selectedSettingKey&&selectedProgram===i){settingsSelect(null);return;}if(!canLeave())return;discard();selectedSettingKey=undefined;selectedProgram=i;selectedKey=undefined;replaceWorkspace(summary,readonly);api.programSelected(i);});nav.append(button);});
  projectBody.append(nav,el('h4','Existing records'));
  const canvas=el('div',undefined,'workspace-canvas');
  const canvasTitle=el('div',undefined,'workspace-canvas-title');canvasTitle.append(el('strong',summary.programs?.[selectedProgram]?.name ?? 'Ladder'),el('span','Select a record · Enter to edit'));canvas.append(canvasTitle);
  targets=ladderTargets(summary,selectedProgram);
  const ladder=(summary.ladder ?? []).find(p=>p.programIndex===selectedProgram);
  if(ladder){if(ladder.projectType===2&&targets.some(t=>t.category!=='function'&&t.position))canvas.append(iecDiagram(ladder,targets));else if(ladder.projectType!==2){const view=el('div',undefined,'workspace-diagram');view.innerHTML=api.ladderMarkup(ladder);api.bindLadder(view,ladder);canvas.append(view);}}
  else canvas.append(el('p','No decoded ladder for this program.','edit-help'));
  const properties=el('aside',undefined,'workspace-properties');properties.setAttribute('aria-label','Selection properties');properties.append(el('h3','Properties'));
  inspector=el('div',undefined,'ladder-record-inspector');properties.append(inspector);
  layout.append(project,canvas,properties);section.append(layout);
  const select=el('select');select.id='ladder-program';select.setAttribute('aria-label','Ladder program');
  (summary.programs ?? []).forEach((p,i)=>{const option=el('option',p.name ?? `Program ${i+1}`);option.value=i;select.append(option);});
  select.value=String(selectedProgram);select.addEventListener('change',()=>{if(!canLeave()){select.value=String(selectedProgram);return;}discard();selectedSettingKey=undefined;selectedProgram=Number(select.value);selectedKey=undefined;replaceWorkspace(summary,readonly);api.programSelected(selectedProgram);});projectBody.append(select);
  const tree=settingsTree(projectSettings(summary),node=>settingsSelect(node));nav.append(tree.root);
  const note=el('div',undefined,'setting-selection-note');note.hidden=true;properties.append(note);
  settingsSelect=node=>{
   if(node?.networkView&&!api.networkCanSelect(node))return;
   if(node?.hardwareBase!==undefined&&!api.hardwareCanSelect(node.hardwareBase))return;
   selectedSettingKey=node?.key;
   canvas.querySelector('.project-setting-view')?.remove();
   for(const child of canvas.children)if(child!==canvasTitle)child.hidden=!!node;
   inspector.hidden=!!node;note.hidden=!node;
   const metadata=properties.querySelector('.sidebar-metadata');if(metadata)metadata.hidden=!!node;
   canvasTitle.firstChild.textContent=node?.label??summary.programs?.[selectedProgram]?.name??'Ladder';canvasTitle.lastChild.textContent=node?'Project settings':'Select a record · Enter to edit';
   section.querySelectorAll('[data-setting-key]').forEach(button=>button.setAttribute('aria-current',String(button.dataset.settingKey===node?.key)));
   section.querySelectorAll('.project-program').forEach(button=>button.setAttribute('aria-current',String(!node&&button.getAttribute('aria-label')===`Open ladder ${summary.programs?.[selectedProgram]?.name??selectedProgram+1}`)));
   if(node){const detail=node.globalVariables?globalVariablesView(summary.variables??[],api.selectVariableTable,readonly):node.networkView?api.networkView(node,summary,readonly):node.hardwareBase!==undefined?api.hardwareView(node.data,summary,readonly):settingDetails(node,index=>{if(readonly){api.message('This workspace is read-only.',true);return;}api.selectVariable(index);});if(readonly){const edit=detail.querySelector('.edit-setting-description');if(edit)edit.disabled=true;}canvas.append(detail);note.replaceChildren(el('strong',node.label),el('p',node.reason,'edit-help'));}
  };
  section.onkeydown=e=>{if(e.key==='Escape'&&selectedSettingKey){e.preventDefault();settingsSelect(null);section.querySelector('.project-program[aria-current="true"]')?.focus();}};
  nav.addEventListener('keydown',e=>{
   const keys=['ArrowDown','ArrowUp','Home','End','ArrowRight','ArrowLeft'];if(!keys.includes(e.key))return;
   const visible=[...nav.querySelectorAll('button,summary')].filter(n=>n.getClientRects().length);const index=visible.indexOf(e.target);if(index<0)return;
   if(e.key==='ArrowRight'||e.key==='ArrowLeft'){const disclosure=e.target.tagName==='SUMMARY'?e.target.parentElement:e.target.closest('details');if(disclosure){e.preventDefault();disclosure.open=e.key==='ArrowRight';if(!disclosure.open)disclosure.querySelector('summary').focus();}return;}
   e.preventDefault();const next=e.key==='Home'?0:e.key==='End'?visible.length-1:Math.max(0,Math.min(visible.length-1,index+(e.key==='ArrowDown'?1:-1)));visible[next]?.focus();
  });
  if(!targets.length){inspector.append(el('p','This program has no supported editable IEC records.','edit-help'));if(selectedSettingKey&&tree.nodes.has(selectedSettingKey))settingsSelect(tree.nodes.get(selectedSettingKey));return section;}
  const list=el('div',undefined,'ladder-record-list');list.setAttribute('role','listbox');list.setAttribute('aria-label','Existing ladder records');projectBody.append(list);
  targets.forEach((item,i)=>{
   const button=el('button',undefined,'ladder-record');button.type='button';button.dataset.ladderKey=item.key;button.setAttribute('role','option');
   const glyph=item.category==='coil'?'—( )—':item.category==='contact'?'—| |—':'[ pin ]';
   button.append(el('small',`L${item.row} · ${glyph} · ${item.label}`),el('strong',item.value));
   button.addEventListener('click',()=>choose(item,readonly));
   button.addEventListener('dblclick',()=>{if(choose(item,readonly))inspector.querySelector('select')?.focus();});
   button.addEventListener('keydown',e=>{if(['ArrowDown','ArrowUp','ArrowRight','ArrowLeft','Home','End'].includes(e.key)){e.preventDefault();const step=['ArrowDown','ArrowRight'].includes(e.key)?1:-1;const next=e.key==='Home'?0:e.key==='End'?targets.length-1:Math.max(0,Math.min(targets.length-1,i+step));if(choose(targets[next],readonly))list.children[next].focus();}else if(e.key==='Enter'){e.preventDefault();if(choose(item,readonly))inspector.querySelector('select')?.focus();}});list.append(button);
  });
  if(ladder?.iecFunctions?.length){
   const functions=el('div',undefined,'decoded-functions');
   for(const block of ladder.iecFunctions){const pins=targets.filter(t=>t.category==='function'&&t.cellOffset===block.recordOffset);if(!pins.length)continue;const diagram=el('div',undefined,'function-diagram');diagram.append(el('h4',block.name));const pinList=el('div',undefined,'function-pin-list');for(const pin of pins){const button=el('button',undefined,`function-pin ${pin.output?'output':'input'}`);button.type='button';button.dataset.pinKey=pin.key;button.append(el('small',pin.label),el('strong',pin.value));button.addEventListener('click',()=>{if(choose(pin,readonly))inspector.querySelector('select')?.focus();});pinList.append(button);}diagram.append(pinList);functions.append(diagram);}
   if(functions.children.length){if(!ladder.cells.length){canvas.querySelector('[data-ladder-table-host]')?.remove();canvas.querySelector('.empty')?.remove();}functions.prepend(el('p','Existing function operands','edit-help'));canvas.append(functions);}
  }
  function bindCells(){section.querySelectorAll('[data-cell-offset]').forEach(cell=>{const matches=targets.filter(t=>t.cellOffset===Number(cell.dataset.cellOffset));if(!matches.length)return;cell.classList.add('editable-ladder-cell');cell.setAttribute('role','button');cell.tabIndex=0;cell.setAttribute('aria-label',`Select ${matches[0].label}: ${matches[0].value}`);const open=()=>{if(choose(matches[0],readonly))inspector.querySelector('select')?.focus();};cell.addEventListener('click',open);cell.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();open();}});});}
  bindCells();section.querySelector('[data-ladder-unknown-toggle]')?.addEventListener('change',()=>{bindCells();markCells();});
  const restoreSetting=selectedSettingKey;
  const initial=targets.find(t=>t.key===selectedKey) ?? targets[0];draft=false;choose(initial,readonly);if(restoreSetting&&tree.nodes.has(restoreSetting))settingsSelect(tree.nodes.get(restoreSetting));return section;
 }
 function iecDiagram(ladder,items){
  const scroll=el('div',undefined,'iec-canvas-scroll');scroll.setAttribute('aria-label','Decoded IEC ladder layout');
  const stage=el('div',undefined,'iec-canvas-stage');scroll.append(stage);
  const edges=ladder.iecCircuitGraph?.edges ?? [];
  const rowKeys=[...new Set([...edges.flatMap(e=>[`${e.start.groupIndex}:${e.start.rowIndex}`,`${e.end.groupIndex}:${e.end.rowIndex}`]),...items.map(t=>`${t.group}:${t.row}`)])].sort((a,b)=>{const[x,y]=a.split(':').map(Number),[u,v]=b.split(':').map(Number);return x-u||y-v;});
  const maxX=Math.max(10,...edges.flatMap(e=>[e.start.x,e.end.x]),...items.map(t=>t.position?.[0]??0));
  const x=value=>55+value/maxX*690;const y=(group,row)=>65+rowKeys.indexOf(`${group}:${row}`)*100;
  const height=Math.max(400,rowKeys.length*100+80);stage.style.height=`${height}px`;
  const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox',`0 0 800 ${height}`);svg.setAttribute('preserveAspectRatio','none');svg.setAttribute('aria-hidden','true');stage.append(svg);
  const line=(x1,y1,x2,y2)=>{const n=document.createElementNS(svg.namespaceURI,'line');for(const[k,v]of Object.entries({x1,y1,x2,y2,stroke:'#778b7d','stroke-width':2}))n.setAttribute(k,v);svg.append(n);};
  for(const edge of edges){if(!edge.kind.toLowerCase().includes('wire'))continue;line(x(edge.start.x),y(edge.start.groupIndex,edge.start.rowIndex),x(edge.end.x),y(edge.end.groupIndex,edge.end.rowIndex));}
  for(const key of rowKeys){const[group,row]=key.split(':').map(Number);const marker=el('span',`G${group+1} · L${row}`,'iec-row-label');marker.style.top=`${y(group,row)/height*100}%`;stage.append(marker);}
  const symbols={NO:'—| |—',NC:'—|/|—',RISING:'—|P|—',FALLING:'—|N|—',NEGATED_RISING:'—|P/|—',NEGATED_FALLING:'—|N/|—',OUTPUT:'—( )—',INVERSE:'—(/)—',SET:'—(S)—',RESET:'—(R)—'};
  for(const item of items.filter(t=>t.category!=='function'&&t.position)){const button=el('div',undefined,'iec-record-node');button.dataset.cellOffset=item.cellOffset;button.style.left=`${x(item.position[0])/800*100}%`;button.style.top=`${y(item.group,item.row)/height*100}%`;button.append(el('span',item.value,'iec-operand-label'),el('strong',symbols[item.expectedKind]??(item.category==='coil'?`—(${item.expectedKind==='RISING'?'P':'N'})—`:'—| |—'),'iec-record-symbol'));stage.append(button);}
  return scroll;
 }
 function replaceWorkspace(summary,readonly){const previous=section;const metadata=previous.querySelector('.sidebar-metadata');const next=render(summary,readonly);if(metadata){metadata.hidden=!!selectedSettingKey;next.querySelector('.workspace-properties').append(metadata);}previous.replaceWith(next);(next.querySelector('.workspace-project').open?next.querySelector('.project-program[aria-current="true"]'):next.querySelector('#ladder-field'))?.focus({preventScroll:true});}
 function choose(item,readonly) {
  if(!canLeave())return false;
  settingsSelect?.(null);discard();target=item;selectedKey=item.key;capturedRevision=api.getRevision();
  section.querySelectorAll('[data-ladder-key]').forEach(b=>{const selected=b.dataset.ladderKey===item.key;b.classList.toggle('selected',selected);b.setAttribute('aria-selected',String(selected));b.tabIndex=selected?0:-1;});
  markCells();showInspector(readonly);return true;
 }
 function markCells(){section.querySelectorAll('[data-pin-key]').forEach(pin=>{const selected=pin.dataset.pinKey===selectedKey;pin.classList.toggle('selected',selected);pin.setAttribute('aria-pressed',String(selected));});section.querySelectorAll('[data-cell-offset]').forEach(cell=>{const selected=Number(cell.dataset.cellOffset)===target?.cellOffset;cell.classList.toggle('selected-ladder-cell',selected);if(cell.classList.contains('editable-ladder-cell'))cell.setAttribute('aria-pressed',String(selected));});}
 function showInspector(readonly) {
  inspector.replaceChildren();inspector.append(el('h4',target.label),el('p',`Current: ${target.value}`,'edit-help'));
  const mode=el('select');mode.id='ladder-field';mode.setAttribute('aria-label','Change ladder field');
  for(const [value,label] of [['operand','Declared operand'],...(target.kindChoices.length?[['kind','Contact / coil kind']]:[])]){const option=el('option',label);option.value=value;mode.append(option);}
  if(!target.kindChoices.length)action='operand';mode.value=action;
  mode.addEventListener('change',()=>{if(!canLeave()){mode.value=action;return;}discard();action=mode.value;showInspector(readonly);});inspector.append(mode);
  const form=el('form');form.id='ladder-change-form';const label=el('label',action==='kind'?'Replacement kind':'Declared symbol');
  const value=el('select');value.id='ladder-replacement';value.setAttribute('aria-label',label.textContent);
  const current=action==='kind'?target.expectedKind:target.value;
  const choices=action==='kind'?target.kindChoices.map(name=>({name,type:''})):target.choices;
  if(!choices.some(c=>c.name===current)){const option=el('option',`${current} (current, retained)`);option.value=current;value.append(option);}
  choices.forEach(c=>{const option=el('option',`${c.name}${c.type?` · ${c.type}`:''}`);option.value=c.name;value.append(option);});value.value=current;
  value.disabled=readonly || !choices.length;value.addEventListener('change',()=>{draft=value.value!==current;api.onDraft();});label.append(value);form.append(label);
  const actions=el('div',undefined,'edit-actions');const apply=el('button','Apply ladder change');apply.id='apply-ladder-change';apply.type='submit';apply.disabled=readonly || !choices.length;
  const cancel=el('button','Cancel ladder change');cancel.type='button';cancel.addEventListener('click',()=>{discard();capturedRevision=api.getRevision();showInspector(readonly);});actions.append(apply,cancel);form.append(actions);
  const help=readonly?'This container is read-only.':!choices.length?'No compatible declared variable is available.':'Choose a compatible declared variable. New elements and wiring are not supported.';
  form.append(el('p',help,'edit-help'));
  form.addEventListener('submit',e=>{e.preventDefault();if(!draft){api.message('No ladder change to apply.');return;}if(capturedRevision!==api.getRevision()){api.message('The workspace changed; cancel and reopen this record before applying.',true);return;}
   const patch={operation:action==='kind'?'kind':target.category==='function'?'functionOperand':'elementOperand',offset:target.offset,expectedKind:target.expectedKind,expectedValue:target.value,replacement:value.value};
   api.apply(target.programIndex,patch,capturedRevision);});
  inspector.append(form);
  section.onkeydown = e =>{if(e.key==='Escape'){e.preventDefault();if(selectedSettingKey){settingsSelect(null);section.querySelector('.project-program[aria-current="true"]')?.focus();return;}discard();showInspector(readonly);(section.querySelector('.workspace-project').open?section.querySelector(`[data-ladder-key="${selectedKey}"]`):section.querySelector('.selected-ladder-cell, .function-pin.selected'))?.focus();}};
 }
 return {render, hasDraft:()=>draft, selectProgram(index){if(!canLeave())return false;discard();selectedProgram=index;selectedKey=undefined;return true;}, showLadder(){settingsSelect?.(null);}, reset(){selectedSettingKey=undefined;selectedProgram=0;selectedKey=undefined;draft=false;target=undefined;}};
}
