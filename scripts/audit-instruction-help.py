#!/usr/bin/env python3
"""Audit chapter 4 instruction names against the native DB and writable catalog.

Inputs are local extracted CHM pages and a private XGTCodeDB TSV export. The
report contains factual identifiers and coverage, never manual prose/images.
Catalog coverage does not imply native execution validation or CPU availability.
"""
import argparse
import csv
import json
import re
from pathlib import Path

args = argparse.ArgumentParser(description=__doc__)
args.add_argument('manual', type=Path)
args.add_argument('database', type=Path)
args.add_argument('--output', required=True, type=Path)
options = args.parse_args()
root = Path(__file__).resolve().parents[1]
with options.database.open(encoding='utf-8') as stream:
    if next(stream).strip() != 'XGTCodeDB': raise ValueError('Expected XGTCodeDB TSV')
    native = {r['Command'].strip(): r for r in csv.DictReader(stream, delimiter='\t')}
catalog = {}
for filename in ['instruction_catalog.rs', 'comparison_catalog.rs']:
    for name, opcode, count in re.findall(
            r'mnemonic: "([^"]+)".*?opcode: (\d+).*?operand_count: (\d+)',
            (root/'src'/filename).read_text(), re.S):
        catalog[name] = {'opcode': int(opcode), 'operandCount': int(count)}
metadata = (root/'src/instruction_operand_data.rs').read_text()
typed = set()
for names in re.findall(r'((?:"[^"\n]+"\s*\|\s*)*"[^"\n]+")\s*=>\s*&\[', metadata):
    typed.update(re.findall(r'"([^"\n]+)"', names))
relations = ['=', '>', '<', '>=', '<=', '<>']
# These commands already use native contact/coil records, rather than the
# application-block catalog. AND/OR select series/parallel wiring in LD.
# Keep their native DB IDs separate from the physical element marker.
diagram_commands = {
    'LOAD': 'NormallyOpen', 'AND': 'NormallyOpen', 'OR': 'NormallyOpen',
    'LOAD NOT': 'NormallyClosed', 'AND NOT': 'NormallyClosed', 'OR NOT': 'NormallyClosed',
    **{prefix+suffix: kind for prefix in ['LOAD', 'AND', 'OR'] for suffix, kind in [
        ('P', 'AddressedRisingPulse'), ('N', 'AddressedFallingPulse'),
        ('P NOT', 'AddressedRisingPulseNot'), ('N NOT', 'AddressedFallingPulseNot')]},
    'OUT': 'Output', 'OUT NOT': 'InverseOutput', 'SET': 'Set', 'RST': 'Reset',
    'OUTP': 'RisingPulseOutput', 'OUTN': 'FallingPulseOutput',
    'NOT': 'Inverse', 'R_EDGE': 'RisingPulse', 'F_EDGE': 'FallingPulse',
}
element_source = (root/'src/ladder_records.rs').read_text().split('impl LadderEditKind')[0]
diagram_commands = {name: kind for name, kind in diagram_commands.items()
                    if re.search(r'^\s*'+kind+r',\s*$', element_source, re.M)}
pages = []
for path in sorted(options.manual.glob('*.htm')):
    html = path.read_bytes().decode('cp949', errors='replace')
    title = re.search(r'<title>(.*?)</title>', html, re.S)
    if not title: continue
    section = re.fullmatch(r'\s*(4\.\d+\.\d+)\s+(.+?)\s*', title[1])
    if not section: continue
    heading = re.sub(r'([A-Z$][A-Z0-9$]*)\(EX\)', lambda m: m[1]+','+m[1]+'EX', section[2])
    # Product annotations and applicability text are not mnemonics.
    heading = re.sub(r'\([^)]*\)', '', heading).split(':')[0]
    names = []
    for token in heading.split(','):
        comparison = re.fullmatch(r'\s*([A-Z$0-9]+)\s*X\s*', token)
        if comparison and section[1].startswith('4.15.'):
            prefix = comparison[1]
            if prefix.startswith('U'):
                prefix = re.sub(r'^(LOAD|AND|OR)', r'\1U', prefix[1:])
            # Manual LOAD3 X / LOADD3 X use native LOAD=3 / LOADD=3 IDs.
            suffix = '3' if prefix.endswith('3') else ''
            if suffix: prefix = prefix[:-1]
            names.extend(prefix + relation + suffix for relation in relations)
        else:
            names.extend(re.findall(r'[A-Z$][A-Z0-9$_]*(?: NOT)?', token))
    # Syyyxx is the operand template for an SFC step, not an instruction.
    if section[1].startswith('4.6.'):
        names = [n for n in names if n != 'S']
    entries = []
    for name in dict.fromkeys(names):
        db = native.get(name)
        writable_name = {'LOADB': 'B', 'LOADBN': 'BN'}.get(name,
            name[4:] if db and name.startswith('LOAD') and db['byCmdList'] == '6' else name)
        spec = catalog.get(writable_name)
        entry = {'manualName': name, 'catalogName': writable_name,
                 'nativeFound': db is not None, 'cataloged': spec is not None,
                 'operandRules': writable_name in typed}
        if name in diagram_commands:
            entry['diagramElement'] = diagram_commands[name]
            entry['placement'] = ('parallel' if name.startswith('OR') else
                                  'series' if name.startswith('AND') else 'cell')
        entry['editorCovered'] = spec is not None or name in diagram_commands
        if db:
            entry.update(opcode=int(db['nIndex']), operandCount=int(db['bySize']),
                         category=int(db['byCmdList']), visibility=db['strShowList'])
        if spec and db and spec != {'opcode': int(db['nIndex']), 'operandCount': int(db['bySize'])}:
            raise ValueError('Native DB disagrees with writable catalog: '+name)
        entries.append(entry)
    pages.append({'section': section[1], 'page': path.name, 'instructions': entries})
if not pages: raise ValueError('No chapter 4 instruction pages found')
report = {'scope': 'XGK chapter 4 LD instruction coverage; cataloged is not native validation',
          'pages': pages,
          'catalogWithoutOperandRules': sorted(set(catalog)-typed)}
options.output.parent.mkdir(parents=True, exist_ok=True)
options.output.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
entries = [e for p in pages for e in p['instructions']]
print(json.dumps({'pages': len(pages), 'manualEntries': len(entries),
                  'nativeFound': sum(e['nativeFound'] for e in entries),
                  'cataloged': sum(e['cataloged'] for e in entries),
                  'editorCovered': sum(e['editorCovered'] for e in entries),
                  'catalogWithOperandRules': len(set(catalog)&typed)}, indent=2))
