import test from 'node:test';
import assert from 'node:assert/strict';
import {ladderTargets, ladderChoices} from './ladder-edit.js';

const contact = 'Normally open contact variable';
const coil = 'Output coil variable';
const symbol = (name, dataTypeCode=1, storageClass='V', isInstance=false) =>
  ({name,dataType:dataTypeCode===1?'BOOL':'INT',dataTypeCode,storageClass,isInstance});

function summary(records, symbols) {
 return {ladder:[{programIndex:0,projectType:2,version:'LD VER 1.1',sourceStrings:records.map((kind,i)=>({iecElementKind:kind,offset:i,value:'Existing'}))}],localVariables:[symbols]};
}

test('initial target construction does not expand symbols for every record', () => {
 const data=summary(Array(2000).fill(contact),Array.from({length:3000},(_,i)=>symbol(`S${i}`)));
 const targets=ladderTargets(data,0);
 assert.equal(targets.length,2000);
 assert.ok(targets.every(t=>t.mask===1 && !Object.hasOwn(t,'choices')));
 assert.equal(ladderChoices(data,targets[0]).total,3000);
});

test('selected choices keep type and direction rules and declaration order', () => {
 const data=summary([contact,coil],[symbol('First'),symbol('Input',1,'I'),symbol('Integer',7),symbol('Instance',1,'V',true),symbol('Last')]);
 const [input,output]=ladderTargets(data,0);
 assert.deepEqual(ladderChoices(data,input).choices.map(c=>c.name),['First','Input','Last']);
 assert.deepEqual(ladderChoices(data,output).choices.map(c=>c.name),['First','Last']);
});

test('search reaches later declarations with bounded visible options', () => {
 const data=summary([contact],Array.from({length:1000},(_,i)=>symbol(`Local${i}`)));
 const [target]=ladderTargets(data,0);
 const initial=ladderChoices(data,target);
 assert.equal(initial.total,1000);
 assert.equal(initial.choices.length,256);
 assert.equal(initial.choices[255].name,'Local255');
 assert.deepEqual(ladderChoices(data,target,'Local999').choices.map(c=>c.name),['Local999']);
});

test('an oversized local table is rejected before scanning its entries', () => {
 const data=summary([contact],new Proxy({length:4097}, {get(target,key){if(key==='length')return target.length;throw new Error('scanned symbols');}}));
 const [target]=ladderTargets(data,0);
 assert.deepEqual(ladderChoices(data,target),{choices:[],total:0,limited:true});
});
