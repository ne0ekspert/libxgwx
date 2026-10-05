import { createNetworkEditor } from './network-edit.js';
import { createHardwareEditor } from "./hardware-edit.js";
import { createLadderEditor } from "./ladder-edit.js";
// The browser exposes a bounded metadata editor, backed by the Rust writer.
export function createEditor(api) {
  const panel = document.querySelector('#editing');
  const toolbar = document.querySelector('#edit-toolbar');
  let supportError = "", revision = 0, draftRevision = 0;
  let original, current, summary, file, history = [], draft = false;
  let programIndex = 0, variableIndex = 0, redo = [], variablePropertiesOpen=false;
  const networkEditor=createNetworkEditor({revision:()=>revision,probe:patch=>api.editNetwork(current,patch),apply:applyNetwork,onDraft:()=>{draftRevision++;updateToolbar();}});
  const hardwareEditor=createHardwareEditor({catalog:api.hardwareCatalog, options:(base,slot)=>api.hardwareOptions(current,base,slot),
    revision:()=>revision, probe:patch=>api.editHardware(current,patch), apply:applyHardware,
    onDraft:()=>{draftRevision++;updateToolbar();}});
  const ladderEditor = createLadderEditor({ getRevision:() => revision, onDraft:() => {draftRevision++; updateToolbar();},
    networkView:(node,parsed,readonly)=>networkEditor.render(node,parsed,readonly),networkCanSelect:node=>networkEditor.canSelect(node),hardwareView:(base,parsed,readonly)=>hardwareEditor.render(base,parsed,readonly), hardwareCanSelect:base=>hardwareEditor.canSelectBase(base), selectVariable, selectVariableTable:index=>selectVariable(index,true), programSelected, apply:applyLadder, message:api.setStatus, ladderMarkup:api.ladderMarkup, bindLadder:api.bindLadder });
  const equal = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
  const dirty = () => original && !equal(original, current);
  const ask = () => !(dirty() || draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft())) || window.confirm('Discard changes and unapplied text in this workspace? Your original file is unchanged.');
  function programSelected(index){
    if(draft||index===programIndex)return;
    programIndex=index;renderForms();updateToolbar();
    const project=panel.querySelector('.workspace-project');
    (project?.open?panel.querySelector('.project-program[aria-current="true"]'):document.getElementById('ladder-field'))?.focus({preventScroll:true});
  }
  function selectVariable(index,keepTable=false){
    if(index!==variableIndex&&(draft||ladderEditor.hasDraft()||(hardwareEditor.hasDraft() || networkEditor.hasDraft()))&&!window.confirm('Discard unapplied text to edit another variable?'))return;
    variablePropertiesOpen=keepTable;
    if(index!==variableIndex){hardwareEditor.clearDraft();networkEditor.clearDraft();draft=false;variableIndex=index;if(!keepTable)ladderEditor.showLadder();renderForms();updateToolbar();}
    else if(!keepTable)ladderEditor.showLadder();
    const disclosure=panel.querySelector('.sidebar-metadata');disclosure.hidden=false;disclosure.open=true;document.getElementById('variable-description')?.focus();
  }
  function hasDraft() {
    const p = summary?.programs?.[programIndex], v = summary?.variables?.[variableIndex];
    return [['program-name', p?.name], ['program-comment', p?.comment], ['variable-description', v?.description]].some(([id, value]) => value != null && document.getElementById(id)?.value !== value);
  }
  const error = (e) => api.setStatus(`Change not applied. ${e instanceof Error ? e.message : String(e)}`, true);
  const input = (label, id, text, multiline = false, disabled = false) => {
    const wrapper = document.createElement('label'); wrapper.textContent = label;
    const field = document.createElement(multiline ? 'textarea' : 'input');
    field.id = id; field.value = text ?? ''; field.disabled = disabled;
    field.maxLength = 4096;
    if (multiline) field.rows = 3; else field.type = 'text';
    field.addEventListener('input', () => { revision++; draft = hasDraft(); updateToolbar(); });
    wrapper.append(field); return wrapper;
  };
  function updateToolbar() {
    toolbar.hidden = !original;
    if (!original) return;
    toolbar.replaceChildren();
    const state = document.createElement('strong'); state.id = 'edit-state';
    state.textContent = `${dirty() ? 'Modified copy' : 'Original unchanged'}${draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft()) ? ' · Unapplied text' : ''}`;
    toolbar.append(state);
    const controls = document.createElement('div'); controls.className = 'edit-actions';
    const action = (name, fn, disabled = false) => {
      const button = document.createElement('button'); button.type = 'button';
      button.textContent = name; button.disabled = disabled; button.addEventListener('click', fn); controls.append(button);
    };
    action('Undo last change', () => {
      if ((draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft())) && !window.confirm('Discard unapplied text and undo the last applied change?')) return;
      revision++; const previous = history.pop(); redo.push({bytes:current.slice(),summary,label:previous.label}); current = previous.bytes; summary = previous.summary;
      draft = false; hardwareEditor.clearDraft();networkEditor.clearDraft();refresh(); api.setStatus('Last change undone.');
    }, !history.length);
    action('Redo change', () => {
      if ((draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft())) && !window.confirm('Discard unapplied text and redo?')) return;
      const next = redo.pop(); revision++; history.push({bytes:current.slice(),summary,label:next.label});
      current=next.bytes; summary=next.summary; draft=false; hardwareEditor.clearDraft();networkEditor.clearDraft();refresh(); api.setStatus('Change redone.');
    }, !redo.length);
    action('Download edited copy', download, !dirty() || draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft()));
    toolbar.append(controls);
  }
  function applyNetwork(patch,expectedRevision){
    try{
      if(supportError||expectedRevision!==revision)throw new Error(supportError||'The workspace changed; cancel and reopen this network field.');
      if(draft||ladderEditor.hasDraft()||hardwareEditor.hasDraft())throw new Error('Apply or cancel the other pending edit first.');
      const bytes=api.editNetwork(current,patch),next=api.parse(bytes),expected=structuredClone(summary);
      expected.networks=next.networks;expected.header.compressedSizeHint=next.header.compressedSizeHint;expected.header.trailerBytes=next.header.trailerBytes;
      if(JSON.stringify(expected)!==JSON.stringify(next)||!api.verify(bytes))throw new Error('Non-target workspace data changed; export rejected.');
      if(equal(bytes,current)){networkEditor.clearDraft();refresh();api.setStatus('No network change to apply.');return;}
      history.push({bytes:current.slice(),summary,label:`Network metadata: ${patch.field}`});redo=[];revision++;current=bytes.slice();summary=next;networkEditor.clearDraft();refresh();api.setStatus('Network change verified.');
    }catch(e){error(e);}
  }
  function applyHardware(patch,expectedRevision){
    try {
      if(supportError||expectedRevision!==revision)throw new Error(supportError||'The workspace changed; cancel and reopen this hardware field.');
      if(ladderEditor.hasDraft()||draft||networkEditor.hasDraft())throw new Error('Apply or cancel the other pending edit first.');
      const bytes=api.editHardware(current,patch),next=api.parse(bytes),expected=structuredClone(summary);
      expected.hardware=next.hardware;expected.parameters=next.parameters;
      expected.header.compressedSizeHint=next.header.compressedSizeHint;expected.header.trailerBytes=next.header.trailerBytes;
      if(JSON.stringify(expected)!==JSON.stringify(next)||!api.verify(bytes))throw new Error('Non-target workspace data changed; export rejected.');
      if(equal(bytes,current)){hardwareEditor.clearDraft();networkEditor.clearDraft();refresh();api.setStatus('No hardware change to apply.');return;}
      history.push({bytes:current.slice(),summary,label:`Hardware ${patch.base}/${patch.slot??'base'}: ${patch.operation}`});redo=[];revision++;current=bytes.slice();summary=next;
      hardwareEditor.clearDraft();networkEditor.clearDraft();refresh();api.setStatus('Hardware change verified.');
    }catch(e){error(e);}
  }
  function applyLadder(index, patch, expectedRevision) {
    try {
      if((hardwareEditor.hasDraft() || networkEditor.hasDraft()))throw new Error('Apply or cancel the pending hardware change first.');
      if (supportError || expectedRevision !== revision) throw new Error(supportError || 'Stale workspace; reopen the record.');
      const pending = ['program-name','program-comment','variable-description'].map(id=>[id,document.getElementById(id)?.value]);
      const bytes = api.editLadder(current,index,patch); const next = api.parse(bytes);
      const expected=structuredClone(summary); const updated=next.ladder?.find(p=>p.programIndex===index);
      if (!updated) throw new Error('Edited ladder could not be decoded.');
      expected.ladder=expected.ladder.map(p=>p.programIndex===index?updated:p);
      expected.header.compressedSizeHint=next.header.compressedSizeHint;expected.header.trailerBytes=next.header.trailerBytes;
      if (JSON.stringify(expected)!==JSON.stringify(next) || !api.verify(bytes)) throw new Error('Non-target summary changed; export rejected.');
      history.push({bytes:current.slice(),summary,label:`Ladder program ${index+1}: ${patch.operation} → ${patch.replacement}`});
      redo=[];revision++;current=bytes.slice();summary=next;draft=false;hardwareEditor.clearDraft();networkEditor.clearDraft();refresh();
      for (const [id,text] of pending) {const field=document.getElementById(id);if(field && text!==undefined && field.value!==text){field.value=text;draft=true;}}
      updateToolbar();api.setStatus('Ladder change verified.');
    } catch(e) {error(e);}
  }
  function refresh() { const activeId=document.activeElement?.id;const recordKey=document.activeElement?.dataset.ladderKey;const hardwareSlot=document.activeElement?.dataset.hardwareSlot;const variableEdit=document.activeElement?.dataset.variableEdit;const settingKey=document.activeElement?.dataset.settingKey;renderForms(); updateToolbar();const focus=activeId?document.getElementById(activeId):recordKey?panel.querySelector(`[data-ladder-key="${recordKey}"]`):hardwareSlot!==undefined?panel.querySelector(`[data-hardware-slot="${hardwareSlot}"]`):variableEdit!==undefined?panel.querySelector(`[data-variable-edit="${variableEdit}"]`):settingKey?panel.querySelector(`[data-setting-key="${settingKey}"]`):null;(focus?.disabled ? panel.querySelector("#network-field")||panel.querySelector("#hardware-field") : focus)?.focus({preventScroll:true}); }
  function renderForms() {
    panel.replaceChildren();
    if (!summary) { const p = document.createElement('p'); p.textContent = 'Open an .xgwx file to edit its PLC programs.'; panel.append(p); return; }
    const workspace=ladderEditor.render(summary,!!supportError);panel.append(workspace);
    const properties=workspace.querySelector('.workspace-properties');
    const metadata=document.createElement('details');metadata.className='sidebar-metadata';metadata.open=true;
    const metadataTitle=document.createElement('summary');metadataTitle.textContent='Program / variable properties';metadata.append(metadataTitle);properties.append(metadata);metadata.hidden=!!workspace.querySelector('.project-setting-view')&&!(variablePropertiesOpen&&workspace.querySelector('.global-variables-view'));
    if (supportError) { const p = document.createElement('p'); p.className = 'warning'; p.textContent = `Read-only workspace: ${supportError}`; panel.append(p); }
    const grid = document.createElement('div'); grid.className = 'edit-grid'; metadata.append(grid);
    function makeForm(title, list, selected, change) {
      const form = document.createElement('form'); form.className = 'edit-card';
      const h = document.createElement('h3'); h.textContent = title; form.append(h);
      const label = document.createElement('label'); label.textContent = `Select ${title.toLowerCase()}`;
      const select = document.createElement('select'); select.setAttribute('aria-label', label.textContent);
      list.forEach((item, i) => { const option = document.createElement('option'); option.value = i; option.textContent = item.name ?? `${title} ${i + 1}`; select.append(option); });
      select.value = String(selected); select.disabled = !list.length;
      select.addEventListener('change', () => {
        if ((draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft())) && !window.confirm('Discard unapplied text to change selection?')) {select.value = String(selected); return;}
        draft = false; hardwareEditor.clearDraft();networkEditor.clearDraft();change(Number(select.value)); renderForms(); updateToolbar();
      }); label.append(select); form.append(label); grid.append(form); return form;
    }
    const programs = summary.programs ?? [];
    programIndex = Math.min(programIndex, Math.max(0, programs.length - 1));
    const pf = makeForm('Program', programs, programIndex, i => programIndex = i);
    const p = programs[programIndex];
    pf.append(input('Program name', 'program-name', p?.name, false, p?.name == null));
    pf.append(input('Program description', 'program-comment', p?.comment, true, p?.comment == null));
    const ph = document.createElement('p'); ph.className = 'edit-help';
    ph.textContent = !p ? 'No program found.' : 'Edit the program name or description.'; pf.append(ph);
    addSubmit(pf, !p || p.name == null && p.comment == null);
    pf.addEventListener('submit', e => {
      e.preventDefault(); const patch = {};
      if (p?.name != null) {const name = pf.querySelector('#program-name').value; if (!name.trim()) return error('Program name cannot be empty.'); if (name !== p.name) patch.name = name;}
      if (p?.comment != null) {const text = pf.querySelector('#program-comment').value; if (text !== p.comment) patch.comment = text;}
      apply('program', programIndex, patch);
    });
    const variables = summary.variables ?? [];
    variableIndex = Math.min(variableIndex, Math.max(0, variables.length - 1));
    const vf = makeForm('Global variable', variables, variableIndex, i => variableIndex = i);
    const v = variables[variableIndex];
    if (!variables.length) vf.remove();
    vf.append(input('Variable description', 'variable-description', v?.description, true, v?.description == null));
    const vh = document.createElement('p'); vh.className = 'edit-help';
    vh.textContent = v?.description != null ? `Keep the original description length (${v.description.length}).` : 'No editable variable description is available.'; vf.append(vh);
    addSubmit(vf, v?.description == null);
    vf.addEventListener('submit', e => {
      e.preventDefault(); const text = vf.querySelector('#variable-description').value;
      if (text.length !== v.description.length) return error(`Description must keep ${v.description.length} UTF-16 code units; received ${text.length}.`);
      apply('variable', variableIndex, text === v.description ? {} : {description:text});
    });
  }
  function addSubmit(form, disabled) {
    const actions = document.createElement('div'); actions.className = 'edit-actions';
    const button = document.createElement('button'); button.type = 'submit'; button.textContent = 'Apply change'; button.disabled = disabled || !!supportError;
    const cancel = document.createElement('button'); cancel.type = 'button'; cancel.textContent = 'Discard all unapplied text';
    cancel.addEventListener('click', () => { draft = false; hardwareEditor.clearDraft();networkEditor.clearDraft();renderForms(); updateToolbar(); api.setStatus('Unapplied text discarded.'); });
    actions.append(button, cancel); form.append(actions);
  }
  function apply(kind, index, patch) {
    if (!Object.keys(patch).length) { draft = hasDraft(); updateToolbar(); api.setStatus('No changes to apply.'); return; }
    try {
      if ((hardwareEditor.hasDraft() || networkEditor.hasDraft())) throw new Error('Apply or cancel the pending hardware change first.');
      if (ladderEditor.hasDraft()) throw new Error('Apply or cancel the pending ladder change first.');
      if (supportError) throw new Error(supportError);
      const pending = ['program-name', 'program-comment', 'variable-description'].filter(id => kind === 'program' ? id.startsWith('variable') : id.startsWith('program')).map(id => [id, document.getElementById(id)?.value]);
      const bytes = (kind === 'program' ? api.updateProgram : api.updateVariable)(current, index, patch);
      const next = api.parse(bytes);
      const expected = structuredClone(summary);
      Object.assign((kind === 'program' ? expected.programs : expected.variables)[index], patch);
      if (kind === 'variable' && patch.description !== undefined) expected.variables[index].comment = patch.description;
      // Container compression size and alignment padding are validated in Rust.
      if (kind === 'program' && patch.name !== undefined) {
        for (const ladder of expected.ladder ?? []) if (ladder.programIndex === index) ladder.programName = patch.name;
      }
      expected.header.compressedSizeHint = next.header.compressedSizeHint;
      expected.header.trailerBytes = next.header.trailerBytes;
      if (JSON.stringify(expected) !== JSON.stringify(next) || !api.verify(bytes)) throw new Error('Export verification failed; current workspace kept.');
      redo=[]; revision++; history.push({bytes:current.slice(), summary, label:`${kind === 'program' ? 'Program' : 'Variable'} ${index + 1}: ${Object.keys(patch).join(', ')}`});
      current = bytes.slice(); summary = next; draft = false; hardwareEditor.clearDraft();networkEditor.clearDraft();refresh();
      for (const [id, text] of pending) { const field = document.getElementById(id); if (field && text !== undefined && text !== field.value) { field.value = text; draft = true; } }
      updateToolbar(); api.setStatus('Change applied and verified.');
    } catch(e) { error(e); }
  }
  function download() {
    try {
      if (!dirty() || draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft())) return;
      const next = api.parse(current);
      if (!api.verify(current) || JSON.stringify(next) !== JSON.stringify(summary)) throw new Error('Download verification failed.');
      const url = URL.createObjectURL(new Blob([current.slice()], {type:'application/octet-stream'}));
      const a = document.createElement('a'); a.href = url; a.download = file.name.replace(/\.xgwx$/i,'') + '-edited.xgwx'; document.body.append(a); a.click(); a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 10000);
      api.setStatus('Verified edited copy downloaded. Your original file is unchanged.');
    } catch(e) { error(e); }
  }
  window.addEventListener('beforeunload', e => {if (dirty() || draft || ladderEditor.hasDraft() || (hardwareEditor.hasDraft() || networkEditor.hasDraft())) {e.preventDefault(); e.returnValue = '';}});
  renderForms();
  document.addEventListener('keydown', e => {
    if (!(e.ctrlKey || e.metaKey) || ['INPUT','TEXTAREA','SELECT'].includes(e.target.tagName)) return;
    const name = e.key.toLowerCase()==='y' || e.key.toLowerCase()==='z' && e.shiftKey ? 'Redo change' : e.key.toLowerCase()==='z' ? 'Undo last change' : null;
    if (!name) return;const button=[...toolbar.querySelectorAll('button')].find(b=>b.textContent===name);
    if (button && !button.disabled) {e.preventDefault();button.click();}
  });
  return {confirmReplace:ask, getRevision:() => `${revision}:${draftRevision}`, load(bytes, parsed, uploaded) {revision++; original = bytes.slice(); current = bytes.slice(); summary = parsed; file = uploaded; history = []; redo = []; draft = false; supportError = ""; try { api.support(bytes); } catch(e) { supportError = String(e); } programIndex = 0; variableIndex = 0; variablePropertiesOpen=false; hardwareEditor.reset();networkEditor.reset();ladderEditor.reset(); renderForms(); updateToolbar();}};
}
