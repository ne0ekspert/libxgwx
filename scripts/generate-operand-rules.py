#!/usr/bin/env python3
"""Extract factual operand tables from a locally extracted XGK InstructionHelp CHM.

Usage: 7z x MANUAL.chm -o/tmp/xgk-help
       python3 scripts/generate-operand-rules.py /tmp/xgk-help
Manual prose/images are not distributed. Shared variant tables retain type unions
unless the individual instruction's semantics have been reviewed below.
"""
from pathlib import Path
from html.parser import HTMLParser
import argparse
import json
import subprocess
import re
import sys

class Tables(HTMLParser):
    def __init__(self):
        super().__init__()
        self.tables, self.rows, self.cells, self.cell = [], None, None, None
    def handle_starttag(self, tag, attrs):
        if tag == 'table': self.rows = []
        elif tag == 'tr': self.cells = []
        elif tag in ('td', 'th'): self.cell = []
    def handle_data(self, data):
        if self.cell is not None: self.cell.append(data)
    def handle_endtag(self, tag):
        if tag in ('td', 'th') and self.cell is not None:
            if self.cells is not None: self.cells.append(' '.join(''.join(self.cell).split()))
            self.cell = None
        elif tag == 'tr' and self.rows is not None:
            self.rows.append(self.cells or []); self.cells = None
        elif tag == 'table' and self.rows is not None:
            self.tables.append(self.rows); self.rows = None

root = Path(__file__).resolve().parents[1]
arguments = argparse.ArgumentParser(description=__doc__)
arguments.add_argument('manual', type=Path, help='directory containing extracted CHM pages')
arguments.add_argument('--check', action='store_true', help='verify generated Rust without writing it')
options = arguments.parse_args()
manual = options.manual
if not manual.is_dir() or not any(manual.glob('*.htm')):
    arguments.error('manual directory must contain extracted .htm pages')
catalog = dict((n, int(c)) for n, c in re.findall(
    r'mnemonic: "([^"]+)".*?operand_count: (\d+)',
    (root / 'src/instruction_catalog.rs').read_text(), re.S))
comparisons = dict((n, int(c)) for n, c in re.findall(
    r'mnemonic: "([^"]+)".*?operand_count: (\d+)',
    (root / 'src/comparison_catalog.rs').read_text(), re.S))
catalog.update(comparisons)
types = {'BIT','NIBBLE','BYTE','WORD','DWORD','LWORD','INT','UINT','DINT','UDINT',
         'LINT','ULINT','REAL','LREAL','STRING'}
# Reviewed per-instruction distinctions from the explanations on these same pages.
precise = {
    'MOV': ['WORD','WORD'], 'DMOV': ['DWORD','DWORD'],
    'I2R': ['INT','REAL'], 'I2L': ['INT','LREAL'],
    'D2R': ['DINT','REAL'], 'D2L': ['DINT','LREAL'],
    'R2I': ['REAL','INT'], 'R2D': ['REAL','DINT'],
    'R2L': ['REAL','LREAL'], 'L2R': ['LREAL','REAL'],
    'RMOV': ['REAL','REAL'], 'LMOV': ['LREAL','LREAL'],
    'ADD': ['INT']*3, 'DADD': ['DINT']*3,
    'SUB': ['INT']*3, 'DSUB': ['DINT']*3,
    'MUL': ['INT','INT','DINT'], 'DMUL': ['DINT','DINT','LINT'],
    'RADD': ['REAL']*3, 'LADD': ['LREAL']*3,
    'RSUB': ['REAL']*3, 'LSUB': ['LREAL']*3,
    'RMUL': ['REAL']*3, 'LMUL': ['LREAL']*3,
    'RDIV': ['REAL']*3, 'LDIV': ['LREAL']*3,
    **{name: ['INT']*2 for name in comparisons},
    'INC': ['INT'], 'DINC': ['DINT'], 'DEC': ['INT'], 'DDEC': ['DINT'],
    'INCU': ['UINT'], 'DINCU': ['UDINT'], 'DECU': ['UINT'], 'DDECU': ['UDINT'],
    'INC4': ['NIBBLE'], 'INC8': ['BYTE'], 'DEC4': ['NIBBLE'], 'DEC8': ['BYTE'],
    'MOV4': ['NIBBLE']*2, 'MOV8': ['BYTE']*2,
}
result = {}; skipped = []
for path in sorted(manual.glob('*.htm')):
    text = path.read_bytes().decode('cp949', errors='replace')
    parser = Tables(); parser.feed(text)
    tables = [t for t in parser.tables if t and len(t[0]) == 3 and t[0][0] == '오퍼랜드']
    if not tables: continue
    title = re.search(r'<title>(.*?)</title>', text, re.S)
    if not title: continue
    names = [n for n in re.findall(r'[A-Za-z$][A-Za-z0-9$]*', title[1]) if n in catalog]
    if path.name == '4151loadxloaddx.htm': names.extend(comparisons)
    rows = [r for r in tables[0][1:] if len(r) == 3]
    usage = next((t for t in parser.tables if len(t) > 2 and '상수' in t[1]), None)
    for name in names:
        if len(rows) != catalog[name]: skipped.append(name); continue
        rules = []
        for label, _, type_text in rows:
            allowed = re.findall(r'[A-Z]+', type_text)
            allowed = [t for t in allowed if t in types]
            if not allowed: rules = []; break
            regions = None; constant = None
            if usage:
                header = usage[1][:usage[1].index('상수') + 1]
                # Remaining device columns after constants are present too.
                header += [h for h in usage[1][len(header):] if h in ['U','N','D','R']]
                for row in usage[2:]:
                    offset = 0 if row and row[0] == label else 1 if len(row)>1 and row[1] == label else None
                    if offset is None: continue
                    permissions = row[offset+1:offset+1+len(header)]
                    if len(permissions) != len(header) or any(v not in ['O','○','-','X','×'] for v in permissions): continue
                    regions = [h for h, v in zip(header, permissions) if h != '상수' and v in ['O','○']]
                    constant = permissions[header.index('상수')] in ['O','○']
                    break
            rules.append((label, allowed, regions, constant))
        if not rules: continue
        base = name[:-1] if name.endswith('P') else name
        refined = precise.get(name, precise.get(base))
        if refined and len(refined) == len(rules):
            rules = [(r[0], [t], r[2], r[3]) for r,t in zip(rules,refined)]
        entry = (path.name, rules)
        if name in result and result[name] != entry: raise ValueError('Conflicting manual entries: '+name)
        result[name] = entry

def quoted(value): return json.dumps(value, ensure_ascii=False)
def array(values): return '&['+', '.join(map(quoted, values))+']'
output = '''//! Factual XGK operand metadata from InstructionHelp_Kr_V3.5.
//! Generated by scripts/generate-operand-rules.py. No manual prose is included.
use super::LadderOperandSpec;

pub(super) fn rules(name: &str) -> &'static [LadderOperandSpec] {
    match name {
'''
groups = {}
for name, entry in sorted(result.items()):
    groups.setdefault(repr(entry), (entry, []))[1].append(name)
for (page,rules), names in groups.values():
    output += '        ' + ' | '.join(map(quoted, names)) + ' => &[\n'
    for label,allowed,regions,constant in rules:
        areas = 'None' if regions is None else 'Some('+array(regions)+')'
        const = 'None' if constant is None else 'Some('+str(constant).lower()+')'
        output += f'            LadderOperandSpec {{ label: {quoted(label)}, data_types: {array(allowed)}, device_areas: {areas}, allows_constant: {const}, manual_page: {quoted(page)} }},\n'
    output += '        ],\n'
output += '        _ => &[],\n    }\n}\n'
if not result:
    arguments.error('no matching operand tables found; refusing to replace the catalog')
formatted = subprocess.run(['rustfmt', '--edition', '2024'], input=output,
                           text=True, capture_output=True, check=True).stdout
output_path = root / 'src/instruction_operand_data.rs'
if options.check:
    if not output_path.exists() or output_path.read_text() != formatted:
        sys.exit('generated operand rules are out of date')
else:
    output_path.write_text(formatted)
print(f'{len(result)}/{len(catalog)} instruction names have type rules; {len(skipped)} arity mismatches omitted')
print('Missing comparison rules:', [n for n in comparisons if n not in result])
