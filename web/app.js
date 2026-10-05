import init, { parse_xgwx, update_xgwx_program, update_xgwx_variable, verify_xgwx_bytes, check_xgwx_edit_support, edit_xgwx_browser_iec, edit_xgwx_browser_hardware, edit_xgwx_browser_network, xgk_module_catalog, xgwx_module_option_values } from "./pkg/libxgwx.js";

import { createEditor } from "./editing.js";

const fileInput = document.querySelector("#file-input");
const dropZone = document.querySelector("#drop-zone");
const statusEl = document.querySelector("#status");
let wasmReady = init();
const editor = createEditor({ parse: parse_xgwx, updateProgram: update_xgwx_program,
  updateVariable: update_xgwx_variable, verify: verify_xgwx_bytes, support: check_xgwx_edit_support, editLadder: edit_xgwx_browser_iec, editHardware:edit_xgwx_browser_hardware, editNetwork:edit_xgwx_browser_network, hardwareCatalog:xgk_module_catalog, hardwareOptions:xgwx_module_option_values, ladderMarkup:renderLadderViewer, bindLadder:bindLadderViewer, setStatus });
let loadSequence = 0;

fileInput.addEventListener("change", () => {
  const [file] = fileInput.files;
  if (file) {
    parseFile(file);
  }
});

dropZone.addEventListener("dragover", (event) => {
  event.preventDefault();
  dropZone.classList.add("dragging");
});

dropZone.addEventListener("dragleave", () => {
  dropZone.classList.remove("dragging");
});

dropZone.addEventListener("drop", (event) => {
  event.preventDefault();
  dropZone.classList.remove("dragging");
  const [file] = event.dataTransfer.files;
  if (file) {
    parseFile(file);
  }
});

async function parseFile(file) {
  if (!editor.confirmReplace()) { fileInput.value = ""; return; }
  const sequence = ++loadSequence;
  const revision = editor.getRevision();
  setStatus(`Opening ${file.name}...`);
  try {
    await wasmReady;
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (sequence !== loadSequence) return;
    const summary = parse_xgwx(bytes);
    if (editor.getRevision() !== revision && !editor.confirmReplace()) { setStatus("File replacement cancelled; current workspace kept."); return; }
    editor.load(bytes, summary, file);
    setStatus(`Opened ${file.name}.`);
  } catch (error) {
    if (sequence === loadSequence) setStatus(`File not loaded; current workspace kept. ${error instanceof Error ? error.message : String(error)}`, true);
  } finally { if (sequence === loadSequence) fileInput.value = ""; }
}

function renderLadderViewer(ladder) {
  const hasDrawableLadder =
    ladder.cells.length ||
    ladder.rungComments?.length ||
    ladder.outputComments?.length ||
    ladder.verticalLines.length ||
    ladder.horizontalLines.length;
  return section("Ladder", [
    hasDrawableLadder ? `<div class="ladder-scroll" data-ladder-table-host>${ladderTable(ladder, false)}</div>` : empty("No positioned ladder cells found."),
  ].join(""));
}

function bindLadderViewer() {}

function ladderTable(ladder, showUnknownMarkers) {
  let rawXs = uniqueSorted([
    ...ladder.cells.map((cell) => cell.rawX),
    ...(showUnknownMarkers ? (ladder.unknownRecords ?? []).map((record) => record.rawX) : []),
  ]);
  const rawYs = uniqueSorted([
    ...ladder.cells.map((cell) => cell.rawY),
    ...(showUnknownMarkers ? (ladder.unknownRecords ?? []).map((record) => record.rawY) : []),
    ...(ladder.rungComments ?? []).map((comment) => comment.rawY),
    ...(ladder.outputComments ?? []).map((comment) => comment.rawY),
    ...ladderBranchGroups(ladder).flatMap((group) => [group.rawYStart, group.rawYEnd]),
    ...ladder.verticalLines.flatMap((line) => [line.rawYStart, line.rawYEnd]),
    ...ladder.horizontalLines.map((line) => line.rawY),
    ...ladder.rungs.map((rung) => rung.rawY),
  ]);

  if (!rawXs.length && rawYs.length && (ladder.rungComments?.length || ladder.outputComments?.length)) {
    rawXs = [1];
  }

  if (!rawXs.length || !rawYs.length) {
    return empty("No drawable ladder coordinates found.");
  }

  const rungByY = new Map(ladder.rungs.map((rung, index) => [rung.rawY, index + 1]));
  const cellsByCoordinate = new Map();
  for (const cell of ladder.cells) {
    const key = coordinateKey(cell.rawX, cell.rawY);
    const bucket = cellsByCoordinate.get(key) ?? [];
    bucket.push(cell);
    cellsByCoordinate.set(key, bucket);
  }
  const unknownByCoordinate = new Map();
  if (showUnknownMarkers) {
    for (const record of ladder.unknownRecords ?? []) {
      const key = coordinateKey(record.rawX, record.rawY);
      const bucket = unknownByCoordinate.get(key) ?? [];
      bucket.push(record);
      unknownByCoordinate.set(key, bucket);
    }
  }
  const rungCommentsByY = new Map();
  for (const comment of ladder.rungComments ?? []) {
    const bucket = rungCommentsByY.get(comment.rawY) ?? [];
    bucket.push(comment);
    rungCommentsByY.set(comment.rawY, bucket);
  }
  const outputCommentsByY = new Map();
  for (const comment of ladder.outputComments ?? []) {
    const bucket = outputCommentsByY.get(comment.rawY) ?? [];
    bucket.push(comment);
    outputCommentsByY.set(comment.rawY, bucket);
  }

  const extraCommandColumns = maxExtraCommandColumnsByRow(ladder);
  const codeColumnCount = rawXs.length + extraCommandColumns;

  const body = `<tbody>${rawYs.map((rawY) => {
    const rungLabel = rungByY.has(rawY) ? String(rawY) : "";
    const rungComments = rungCommentsByY.get(rawY) ?? [];
    const outputComments = outputCommentsByY.get(rawY) ?? [];
    if (rungComments.length) {
      return `<tr>
        <th scope="row">${escapeHtml(rungLabel)}</th>
        <td class="ladder-rung-comment" colspan="${escapeHtml(String(codeColumnCount))}">${rungComments.map(renderRungComment).join("")}</td>
        <td class="ladder-output-comment">${outputComments.map(renderOutputComment).join("")}</td>
      </tr>`;
    }
    return `<tr>
      <th scope="row">${escapeHtml(rungLabel)}</th>
      ${renderLadderTableRowCells(ladder, rawXs, cellsByCoordinate, unknownByCoordinate, rawY, codeColumnCount)}
      <td class="ladder-output-comment">${outputComments.map(renderOutputComment).join("")}</td>
    </tr>`;
  }).join("")}</tbody>`;

  return `<table class="ladder-table">${body}</table>`;
}

function renderRungComment(comment) {
  return `<div class="ladder-rung-comment-text">${escapeHtml(comment.text)}</div>`;
}

function renderOutputComment(comment) {
  return `<div class="ladder-output-comment-text">${escapeHtml(comment.text)}</div>`;
}

function renderLadderTableRowCells(ladder, rawXs, cellsByCoordinate, unknownByCoordinate, rawY, codeColumnCount) {
  const rendered = [];
  for (const rawX of rawXs) {
    const key = coordinateKey(rawX, rawY);
    const cells = cellsByCoordinate.get(key) ?? [];
    const unknownRecords = unknownByCoordinate.get(key) ?? [];
    if (cells.length === 1 && isSplitCommandCell(cells[0]) && !unknownRecords.length) {
      rendered.push(renderLadderCommandTableCells(ladder, cells[0], rawX, rawY, rawXs));
    } else {
      rendered.push(renderLadderTableCell(ladder, cells, unknownRecords, rawX, rawY, rawXs));
    }
  }

  return padLadderTableRow(rendered, codeColumnCount);
}

function padLadderTableRow(rendered, codeColumnCount) {
  let count = 0;
  for (const html of rendered) {
    count += Number(html.match(/<td\b/g)?.length ?? 0);
  }
  while (count < codeColumnCount) {
    rendered.push(`<td class="ladder-table-cell is-padding"><span class="wire-placeholder"></span></td>`);
    count += 1;
  }
  return rendered.join("");
}

function ladderCellLabel(cell) {
  if (!cell.value && (cell.contact === "PUP" || cell.contact === "PDN")) {
    return "";
  }
  if (cell.value) {
    return cell.value;
  }
  if (cell.operands?.length) {
    return cell.operands.join(", ");
  }
  return cell.kind;
}

function renderLadderTableCell(ladder, cells, unknownRecords, rawX, rawY, rawXs) {
  const classes = ["ladder-table-cell"];
  if (hasHorizontalLine(ladder, rawX, rawY)) {
    classes.push("has-horizontal");
  }
  classes.push(...verticalLineClasses(ladder, rawX, rawY, rawXs));
  if (cells.length) {
    classes.push("has-cell");
    classes.push("has-horizontal");
  }
  if (unknownRecords.length) {
    classes.push("has-unknown");
  }

  const content = [
    cells.map(renderLadderCellToken).join(""),
    unknownRecords.map(renderUnknownRecordToken).join(""),
  ].join("") || `<span class="wire-placeholder">${hasHorizontalLine(ladder, rawX, rawY) ? "line" : ""}</span>`;

  return `<td class="${classes.join(" ")}">${content}</td>`;
}

function renderLadderCommandTableCells(ladder, cell, rawX, rawY, rawXs) {
  const baseClasses = ["ladder-table-cell", "has-cell", "is-command-part"];
  baseClasses.push(...verticalLineClasses(ladder, rawX, rawY, rawXs));

  const title = ladderCellTitle(cell);
  const command = `<td class="${[...baseClasses, "is-command-mnemonic-cell"].join(" ")}">
    <div class="ladder-command-cell-token is-mnemonic" data-cell-offset="${cell.offset}" title="${escapeHtml(title)}">${escapeHtml(cell.value || cell.kind)}</div>
  </td>`;
  const operands = cell.operands.map((operand, index) => `<td class="ladder-table-cell has-cell is-command-part is-command-operand-cell ${index === cell.operands.length - 1 ? "is-command-last-cell" : ""}">
    <div class="ladder-command-cell-token is-operand" title="${escapeHtml(title)}">${escapeHtml(operand)}</div>
  </td>`);

  return [command, ...operands].join("");
}

function renderUnknownRecordToken(record) {
  return `<div class="ladder-token is-unknown" title="${escapeHtml(record.bytes)}">
    <span class="unknown-marker">${escapeHtml(record.marker)}</span>
    <span class="unknown-offset">@${escapeHtml(String(record.offset))}</span>
    <span class="unknown-bytes">${escapeHtml(record.bytes)}</span>
  </div>`;
}

function renderLadderCellToken(cell) {
  const title = ladderCellTitle(cell);
  const classes = ["ladder-token", ladderCellClass(cell)];
  return `<div class="${classes.join(" ")}" data-cell-offset="${cell.offset}" title="${escapeHtml(title)}">
    ${ladderCellBody(cell)}
  </div>`;
}

function ladderCellTitle(cell) {
  return [
    cell.kind,
    cell.value,
    cell.operands?.join(", "),
    cell.mnemonicCategory,
    cell.mnemonicDescription,
  ].filter(Boolean).join(" | ");
}

function isSplitCommandCell(cell) {
  return Boolean(cell.operands?.length && !cell.contact && !cell.coil && cell.kind !== "Comment");
}

function maxExtraCommandColumnsByRow(ladder) {
  const extrasByY = new Map();
  for (const cell of ladder.cells) {
    if (isSplitCommandCell(cell)) {
      extrasByY.set(cell.rawY, (extrasByY.get(cell.rawY) ?? 0) + cell.operands.length);
    }
  }
  return Math.max(0, ...extrasByY.values());
}

function ladderCellBody(cell) {
  if (cell.contact) {
    return [
      `<span class="ladder-token-label">${escapeHtml(ladderCellLabel(cell))}</span>`,
      `<span class="ladder-token-symbol">${escapeHtml(ladderCellSymbol(cell))}</span>`,
    ].join("");
  }

  if (cell.coil) {
    return [
      `<span class="ladder-token-symbol">${escapeHtml(ladderCellSymbol(cell))}</span>`,
      `<span class="ladder-token-label">${escapeHtml(ladderCellLabel(cell))}</span>`,
    ].join("");
  }

  if (!cell.operands?.length) {
    return `<span class="ladder-token-mnemonic">${escapeHtml(cell.value || cell.kind)}</span>`;
  }

  const operands = cell.operands.map((operand) => `<span>${escapeHtml(operand)}</span>`).join("");
  return [
    `<span class="ladder-token-mnemonic">${escapeHtml(cell.value || cell.kind)}</span>`,
    `<span class="ladder-token-operands">${operands}</span>`,
  ].join("");
}

function ladderCellClass(cell) {
  if (cell.contact) {
    return "is-contact";
  }
  if (cell.coil) {
    return "is-coil";
  }
  if (cell.kind === "Comment") {
    return "is-comment";
  }
  if (!cell.operands?.length) {
    return "is-block is-zero-operand";
  }
  return "is-block";
}

function hasHorizontalLine(ladder, rawX, rawY) {
  return ladder.horizontalLines.some((line) => {
    const minX = Math.min(line.rawXStart, line.rawXEnd);
    const maxX = Math.max(line.rawXStart, line.rawXEnd);
    return line.rawY === rawY && rawX >= minX && rawX <= maxX;
  });
}

function ladderBranchGroups(ladder) {
  return ladder.branchGroups?.length ? ladder.branchGroups : ladder.verticalLines;
}

function verticalLineClasses(ladder, rawX, rawY, rawXs) {
  const group = ladderBranchGroups(ladder).find((line) => {
    const minY = Math.min(line.rawYStart, line.rawYEnd);
    const maxY = Math.max(line.rawYStart, line.rawYEnd);
    return verticalLineColumnX(line.rawX, rawXs) === rawX && rawY >= minY && rawY <= maxY;
  });
  if (!group) {
    return [];
  }

  const minY = Math.min(group.rawYStart, group.rawYEnd);
  const maxY = Math.max(group.rawYStart, group.rawYEnd);
  if (rawY === minY) {
    return ["has-vertical", "has-vertical-start"];
  }
  if (rawY === maxY) {
    return ["has-vertical", "has-vertical-end"];
  }
  return ["has-vertical", "has-vertical-middle"];
}

function verticalLineColumnX(rawX, rawXs) {
  return rawXs.find((candidate) => candidate >= rawX) ?? rawXs[rawXs.length - 1] ?? rawX;
}

function ladderCellSymbol(cell) {
  if (cell.contact === "P_CONTACT") {
    return "-|P|-";
  }
  if (cell.contact === "P_NOT_CONTACT") {
    return "-|P/|-";
  }
  if (cell.contact === "N_CONTACT") {
    return "-|N|-";
  }
  if (cell.contact === "N_NOT_CONTACT") {
    return "-|N/|-";
  }
  if (cell.contact === "PUP") {
    return "^^| |";
  }
  if (cell.contact === "PDN") {
    return "| |vv";
  }
  if (cell.contact === "INV") {
    return "-*-";
  }
  if (cell.contact === "NC") {
    return "|/|";
  }
  if (cell.contact === "NO") {
    return "| |";
  }
  if (cell.coil === "Set") {
    return "(S)";
  }
  if (cell.coil === "Reset") {
    return "(R)";
  }
  if (cell.coil === "Inverse") {
    return "(/)";
  }
  if (cell.coil === "P_COIL") {
    return "-(P)-";
  }
  if (cell.coil === "N_COIL") {
    return "-(N)-";
  }
  if (cell.coil === "Output") {
    return "( )";
  }
  return cell.kind;
}

function coordinateKey(rawX, rawY) {
  return `${rawX}:${rawY}`;
}

function uniqueSorted(values) {
  return [...new Set(values)].sort((a, b) => a - b);
}

function setStatus(message, isError = false) {
  statusEl.textContent = message;
  statusEl.classList.toggle("error", isError);
}

function section(title, content) {
  return `<div class="section"><h2>${escapeHtml(title)}</h2>${content}</div>`;
}

function empty(message) {
  return `<div class="section muted">${escapeHtml(message)}</div>`;
}

function escapeHtml(text) {
  const replacements = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#039;",
  };
  return String(text).replace(/[&<>"']/g, (character) => replacements[character]);
}
