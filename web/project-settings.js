// Independent project navigation built only from the parsed libxgwx summary.
const make=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n;};
const item=(key,label,data,reason,extra={})=>({key,label,data,reason,...extra});
const readOnly='Read-only: this browser editor has no validated writer for these settings.';
const group=(label,children)=>({label,children});
export function projectSettings(s){
 const groups=[];
 const hardware=[];
 if(s.cpu)hardware.push(item('cpu',s.cpu.model?`CPU · ${s.cpu.model}`:s.cpu.configurationName||'PLC / CPU',s.cpu,readOnly));
 for(const [i,base] of (s.hardware?.bases??[]).entries())hardware.push(item(`base:${i}`,`Base ${base.base??i}`,base,'Select a slot to edit supported module settings.',{hardwareBase:base.base}));
 for(const [i,m] of (s.hardware?.modules??[]).entries())if(!(s.hardware?.bases??[]).some(b=>b.base===m.base))hardware.push(item(`module:${i}`,`Module · ${m.name||`Slot ${m.slot??i}`}`,m,readOnly));
 if(hardware.length)groups.push(group('PLC / Hardware',hardware));
 const parameters=(s.parameters??[]).map((p,i)=>item(`parameter:${i}`,p.parameterType||`Parameter ${i+1}`,p,readOnly));
 for(const [key,label] of [['hsc','High-speed counter'],['position','Positioning']])for(const [i,p] of (s[key]??[]).entries())parameters.push(item(`${key}:${i}`,`${label} ${i+1}`,p,readOnly));
 for(const [key,label] of [['calculation','PID calculation'],['tuning','PID tuning']])for(const [i,p] of (s.pid?.[key]??[]).entries())parameters.push(item(`pid:${key}:${i}`,`${label} ${i+1}`,p,readOnly));
 if(s.safetyComm)parameters.push(item('safety','Safety communication',s.safetyComm,readOnly));
 if(parameters.length)groups.push(group('Parameters',parameters));
 const networks=[];
 for(const [i,n] of (s.networks??[]).entries())networks.push(group(n.name||`Network ${i+1}`,[item(`network:${i}`,'Network properties',n,'Edit supported existing metadata; protocol parameters remain read-only.',{networkView:true,networkTarget:{networkIndex:i}}),...(n.modules??[]).map((m,j)=>item(`network-module:${i}:${j}`,m.name||m.configName||`Communication module ${j+1}`,m,'Edit existing module metadata; linked services remain read-only.',{networkView:true,networkTarget:{networkIndex:i,moduleIndex:j}}))]));
 for(const [key,label] of [['cnet','Cnet'],['fenet','FEnet'],['xgpd','XGPD']])for(const [i,n] of (s[key]??[]).entries())networks.push(item(`${key}:${i}`,`${label} ${i+1}`,n,'Decoded communication parameters; no validated writer for protocol fields.',{networkView:true}));
 if(networks.length)groups.push(group('Network / Communication',networks));
 groups.push(item('globals','Global Variables',s.variables??[],'View decoded global variables as a table; edit existing descriptions only.',{globalVariables:true}));
 const locals=[];
 for(const [i,variables] of (s.localVariables??[]).entries())if(variables.length)locals.push(group(s.programs?.[i]?.name||`Program ${i+1}`,variables.map((v,j)=>item(`local:${i}:${j}`,v.name,v,'Read-only declaration. Compatible variables can be selected as existing ladder operands.'))));
 if(locals.length)groups.push(group('Local variables',locals));
 return groups;
}
const excluded=/^(?:header|counts|warnings|guid|index|version|formatVersion|sourceRef|childCount|recordOffset|offset|payload.*|raw.*|.*Raw|details|attributes|sections|text|dataTypeCode|typeCode|subType)$/i;
const fieldName=s=>s.replace(/([a-z])([A-Z])/g,'$1 $2').replace(/^./,c=>c.toUpperCase());
function values(data,host,prefix='',depth=0){
 if(depth>6)return;
 const dl=make('dl',undefined,'setting-fields');
 for(const [key,value] of Object.entries(data??{})){
  if(value==null||excluded.test(key))continue;
  if(Array.isArray(value)){
   if(!value.length)continue;
   const details=make('details',undefined,'setting-section');details.open=true;details.append(make('summary',fieldName(key)));
   value.forEach((v,i)=>{if(v&&typeof v==='object'){const row=make('div');row.append(make('h4',v.name||`${fieldName(key)} ${i+1}`));values(v,row,'',depth+1);details.append(row);}else details.append(make('p',String(v)));});host.append(details);
  }else if(typeof value==='object'){
   const details=make('details',undefined,'setting-section');details.open=true;details.append(make('summary',fieldName(key)));values(value,details,'',depth+1);host.append(details);
  }else if(typeof value==='string'||typeof value==='number'||typeof value==='boolean'){
   dl.append(make('dt',`${prefix}${fieldName(key)}`),make('dd',typeof value==='boolean'?(value?'Enabled':'Disabled'):String(value)));
  }
 }
 if(dl.children.length)host.prepend(dl);
 // Expose named decoded parameter fields, never payload text or encoded blobs.
 const attrs=(data?.attributes??[]).filter(a=>!excluded.test(a.name)&&!/(?:SIGNATURE|PAYLOAD|GUID|LENGTH|DETAILS)/i.test(a.name));
 if(attrs.length){const fields=make('dl',undefined,'setting-fields');for(const a of attrs)fields.append(make('dt',a.name),make('dd',String(a.value)));host.append(fields);}
 for(const section of data?.sections??[]){const box=make('details',undefined,'setting-section');box.open=true;box.append(make('summary',section.name));values({attributes:section.attributes},box,'',depth+1);if(box.children.length===1)box.append(make('p','This section is present; its detailed values are not decoded in the browser.','edit-help'));host.append(box);}
}
export function settingDetails(node,editVariable){
 const panel=make('section',undefined,'project-setting-view');panel.id='project-setting-view';panel.setAttribute('aria-label',node.label);
 panel.append(make('h2',node.label),make('p',node.reason,'setting-support'));
 values(node.data,panel);
 if(panel.children.length===2)panel.append(make('p','This setting is present, but its detailed values are not decoded.','edit-help'));
 if(node.editVariable!==undefined){const edit=make('button','Edit description','edit-setting-description');edit.type='button';edit.addEventListener('click',()=>editVariable(node.editVariable));panel.append(edit);}
 return panel;
}
export function settingsTree(groups,select){
 const nodes=new Map(),root=make('div',undefined,'project-setting-tree');
 function append(items,host){for(const node of items){if(node.children){const details=make('details');details.open=true;details.append(make('summary',node.label));append(node.children,details);host.append(details);}else{nodes.set(node.key,node);const button=make('button',node.label,'project-setting-item');button.type='button';button.dataset.settingKey=node.key;button.setAttribute('aria-label',node.label);button.setAttribute('aria-controls','project-setting-view');button.addEventListener('click',()=>select(node));host.append(button);}}}
 append(groups,root);return {root,nodes};
}
