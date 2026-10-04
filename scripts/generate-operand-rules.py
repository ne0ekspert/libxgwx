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
    'INC': ['INT'], 'DINC': ['DINT'], 'DEC': ['INT'], 'DDEC': ['DINT'],
    'INCU': ['UINT'], 'DINCU': ['UDINT'], 'DECU': ['UINT'], 'DDECU': ['UDINT'],
    'INC4': ['NIBBLE'], 'INC8': ['BYTE'], 'DEC4': ['NIBBLE'], 'DEC8': ['BYTE'],
    'MOV4': ['NIBBLE']*2, 'MOV8': ['BYTE']*2,
    'SCAL': ['INT']*4, 'DSCAL': ['DINT']*4, 'RSCAL': ['REAL']*4,
    'SCAL2': ['INT']*5, 'DSCAL2': ['DINT']*5, 'RSCAL2': ['REAL']*5,
    'BINHA': ['WORD','DWORD'], 'DBINHA': ['DWORD','DWORD'],
}
comparison_pages = {}
for name, count in comparisons.items():
    if name in ['B', 'BN']:
        precise[name] = ['WORD', 'WORD']
        comparison_pages.setdefault('4381loadbloadbn.htm', []).append(name)
        continue
    match = re.fullmatch(r'([^<>=]*)([<>=]+)(3?)', name)
    if not match: raise ValueError('Unknown comparison spelling: '+name)
    prefix, _, suffix = match.groups()
    type_name = {'': 'INT', 'D': 'DINT', 'R': 'REAL', 'L': 'LREAL', '$': 'STRING',
                 'G': 'INT', 'DG': 'DINT', '4': 'NIBBLE', '8': 'BYTE',
                 'U': 'UINT', 'UD': 'UDINT'}[prefix]
    precise[name] = [type_name]*count
    if prefix in ['G', 'DG']: precise[name][-1] = 'WORD'
    page = ('41513load3xloadd3x.htm' if suffix else {
        '': '4151loadxloaddx.htm', 'D': '4151loadxloaddx.htm',
        'R': '4154loadrxloadlx.htm', 'L': '4154loadrxloadlx.htm',
        '$': '4157loadx.htm', 'G': '41510loadgxloaddgx.htm', 'DG': '41510loadgxloaddgx.htm',
        '4': '41516load4xload8x.htm', '8': '41516load4xload8x.htm',
        'U': '41519uloadxuloaddx1.htm', 'UD': '41519uloadxuloaddx1.htm'}[prefix])
    comparison_pages.setdefault(page, []).append(name)
result = {}; skipped = []
# Pages with image-only layouts or demonstrably pasted operand tables. These
# factual labels/types are reviewed against their usage tables, diagrams and
# explanations; keep the manual page as provenance for every generated rule.
reviewed_rows = {
    '4141cmpcmppdcmpdcmpp.htm': [('S1', '', 'UINT/UDINT'), ('S2', '', 'UINT/UDINT')],
    '4142cmp4cmp4pcmp8cmp8p.htm': [('S1', '', 'NIBBLE/BYTE'), ('S2', '', 'NIBBLE/BYTE')],
    '4364ebwrite.htm': [('S1', '', 'WORD'), ('S2', '', 'WORD')],
    '44121srs1.htm': [('sl', '', 'WORD'), ('ax', '', 'WORD'), ('n1', '', 'WORD')],
    '44244xswr.htm': [('sl', '', 'WORD'), ('ax', '', 'WORD'), ('S', '', 'WORD'), ('n1', '', 'WORD')],
}
# These title typos are independently resolved by the command usage table on
# the same page and by the native instruction database (no opcode inference).
reviewed_names = {
    '4211addbaddcpdaddbdaddbp.htm': ['ADDBP'],
    '4248betowbtowp.htm': ['BTOW'],
    '4433xturn.htm': ['XTRUN'],
}
precise.update({'CMP': ['UINT']*2, 'DCMP': ['UDINT']*2,
                'CMP4': ['NIBBLE']*2, 'CMP8': ['BYTE']*2})
for path in sorted(manual.glob('*.htm')):
    text = path.read_bytes().decode('cp949', errors='replace')
    parser = Tables(); parser.feed(text)
    tables = [t for t in parser.tables if t and len(t[0]) == 3 and t[0][0] == '오퍼랜드']
    if not tables and path.name not in reviewed_rows: continue
    title = re.search(r'<title>(.*?)</title>', text, re.S)
    if not title: continue
    # The manual abbreviates motion-module variants as NAME(EX).
    expanded_title = re.sub(r'([A-Za-z$][A-Za-z0-9$]*)\(EX\)',
                            lambda m: m[1] + ', ' + m[1] + 'EX', title[1])
    names = list(dict.fromkeys(n for n in re.findall(r'[A-Za-z$][A-Za-z0-9$]*', expanded_title)
                               if n in catalog))
    names.extend(n for n in reviewed_names.get(path.name, []) if n in catalog and n not in names)
    names.extend(comparison_pages.get(path.name, []))
    # S2+1 describes memory addressed relative to S2, not another argument.
    # Retain short rows: Word's vertically merged cells inherit the type.
    rows = []
    inherited_types = ''
    for row in (tables[0][1:] if tables else []):
        if len(row) == 3: inherited_types = row[2]
        elif len(row) == 2: row = row + [inherited_types]
        else: continue
        if not re.fullmatch(r'.+\s*\+\s*\d+', row[0]): rows.append(row)
    if path.name in reviewed_rows: rows = reviewed_rows[path.name]
    usage = next((t for t in parser.tables if len(t) > 2
                  and any(h in t[1] for h in ['상수', '문자열'])), None)
    for name in names:
        instruction_rows = rows
        if len(rows) != catalog[name] and usage:
            # Some pages contain stale extra operand-table rows. Only discard
            # them when the independent usage table identifies every argument.
            labels = []
            for row in usage[2:]:
                if len(row) > 1:
                    label = row[0] if row[1] in ['O','○','-','X','×'] else row[1]
                    if label not in labels: labels.append(label)
            selected = [r for label in labels for r in rows if r[0] == label]
            if len(labels) == len(selected) == catalog[name]: instruction_rows = selected
        # PIDINIT's operand table is a pasted GWXNR table. Its usage table and
        # explanation both specify the single constant PID loop number S.
        if name == 'PIDINIT' and path.name == '4288pidinit.htm':
            instruction_rows = [('S', '', '상수')]
        if len(instruction_rows) != catalog[name]: skipped.append(name); continue
        rules = []
        for label, _, type_text in instruction_rows:
            type_text = re.sub(r'BIN\s*32', 'DWORD', type_text)
            type_text = re.sub(r'BIN\s*16', 'WORD', type_text)
            if type_text == '상수': type_text = 'WORD'
            allowed = re.findall(r'[A-Z]+', type_text)
            allowed = [t for t in allowed if t in types]
            if not allowed: rules = []; break
            regions = None; constant = None
            if usage:
                literal_column = '상수' if '상수' in usage[1] else '문자열'
                header = usage[1][:usage[1].index(literal_column) + 1]
                # Remaining device columns after constants are present too.
                header += [h for h in usage[1][len(header):] if h in ['U','N','D','R']]
                for row in usage[2:]:
                    offset = 0 if row and row[0] == label else 1 if len(row)>1 and row[1] == label else None
                    if offset is None: continue
                    permissions = row[offset+1:offset+1+len(header)]
                    if len(permissions) != len(header) or any(v not in ['O','○','-','X','×'] for v in permissions): continue
                    regions = [h for h, v in zip(header, permissions) if h != literal_column and v in ['O','○']]
                    constant = permissions[header.index(literal_column)] in ['O','○']
                    break
            rules.append((label, allowed, regions, constant))
        if not rules: continue
        if name in ['B', 'BN']:
            # The source row has an empty F permission cell. The native B/BN
            # usage dialog independently excludes F and constants for S.
            rules[0] = (rules[0][0], rules[0][1], ['PMK', 'L', 'T', 'C', 'Z', 'U', 'N', 'D', 'R'], False)
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
