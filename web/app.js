import init, { parse_xgwx } from "./pkg/libxgwx.js";

const fileInput = document.querySelector("#file-input");
const dropZone = document.querySelector("#drop-zone");
const statusEl = document.querySelector("#status");
const panels = {
  summary: document.querySelector("#summary"),
  programs: document.querySelector("#programs"),
  variables: document.querySelector("#variables"),
  hardware: document.querySelector("#hardware"),
  networks: document.querySelector("#networks"),
  parameters: document.querySelector("#parameters"),
};

let wasmReady = init();

document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((item) => item.classList.remove("active"));
    document.querySelectorAll(".panel").forEach((item) => item.classList.remove("active"));
    tab.classList.add("active");
    document.querySelector(`#${tab.dataset.tab}`).classList.add("active");
  });
});

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
  setStatus(`Parsing ${file.name}...`);
  clearPanels();

  try {
    await wasmReady;
    const bytes = new Uint8Array(await file.arrayBuffer());
    const summary = parse_xgwx(bytes);
    render(summary, file);
    setStatus(`Parsed ${file.name} (${formatBytes(file.size)}).`);
  } catch (error) {
    setStatus(error instanceof Error ? error.message : String(error), true);
  }
}

function render(summary, file) {
  renderSummary(summary, file);
  renderPrograms(
    summary.programs,
    summary.ladder ?? [],
    summaryIndicatesSkippedLadderDecode(summary),
  );
  renderVariables(summary.variables ?? []);
  renderHardware(summary.hardware ?? { bases: [], modules: [] });
  renderNetworks(summary.networks, summary.cnet, summary.fenet);
  renderParameters(summary);
}

function renderSummary(summary, file) {
  const warnings = summary.warnings ?? [];
  const payloadDecodeSkipped = summaryIndicatesSkippedPayloadDecode(summary);
  const ladderDecodeSkipped = summaryIndicatesSkippedLadderDecode(summary);
  panels.summary.innerHTML = [
    section("Project", details([
      ["Uploaded file", file.name],
      ["File size", formatBytes(file.size)],
      ["Project name", value(summary.project.name)],
      ["File version", value(summary.project.fileVersion)],
      ["Last write time", value(summary.project.fileLastWriteTime)],
      ["GUID", value(summary.project.guid)],
      ["Header label", value(summary.header.label)],
      ["Header bytes", summary.header.headerBytes],
      ["Trailer bytes", summary.header.trailerBytes],
    ])),
    `<div class="grid">${metric("Programs", summary.counts.programs)}${metric("Networks", summary.counts.networks)}${metric("Modules", summary.counts.modules)}${metric("Variables", value(summary.counts.variables))}${metric("Payloads", payloadDecodeSkipped ? "Skipped" : summary.counts.decodedPayloads)}${metric("Ladder", ladderDecodeSkipped ? "Skipped" : summary.counts.ladderPrograms)}</div>`,
    warnings.length
      ? section("Warnings", `<div class="list">${warnings.map((warning) => `<div class="list-item warning">${escapeHtml(warning)}</div>`).join("")}</div>`)
      : "",
  ].join("");
}

function renderPrograms(programs, ladderPrograms, decodeSkipped = false) {
  if (!programs.length) {
    panels.programs.innerHTML = empty("No programs found.");
    return;
  }

  panels.programs.innerHTML = `
    <div class="program-layout">
      <aside class="program-sidebar" aria-label="Program list">
        ${programs.map((program, index) => {
          const ladder = ladderPrograms.find((item) => item.programIndex === index);
          const meta = ladder
            ? `${ladder.rungs.length} rungs, ${ladder.cells.length} cells`
            : decodeSkipped ? "Not decoded in browser summary" : "No ladder data";
          return `<button class="program-button ${index === 0 ? "active" : ""}" type="button" data-program-index="${index}">
            <span>${escapeHtml(value(program.name, `<program ${index + 1}>`))}</span>
            <small>${escapeHtml(meta)}</small>
          </button>`;
        }).join("")}
      </aside>
      <div id="program-detail" class="program-detail"></div>
    </div>`;

  const detail = panels.programs.querySelector("#program-detail");
  const buttons = panels.programs.querySelectorAll(".program-button");
  const showProgram = (index) => {
    buttons.forEach((button) => {
      button.classList.toggle("active", Number(button.dataset.programIndex) === index);
    });
    const ladder = ladderPrograms.find((item) => item.programIndex === index);
    detail.innerHTML = renderProgramDetail(programs[index], ladder, index, decodeSkipped);
    bindLadderViewer(detail, ladder);
  };

  buttons.forEach((button) => {
    button.addEventListener("click", () => showProgram(Number(button.dataset.programIndex)));
  });

  showProgram(0);
}

function renderProgramDetail(program, ladder, index, decodeSkipped = false) {
  return [
    section(value(program.name, `<program ${index + 1}>`), details([
      ["Task", value(program.task)],
      ["Kind", value(program.kind)],
      ["Version", value(program.version)],
      ["Object ID", value(program.objectId)],
      ["Comment", value(program.comment)],
    ])),
    ladder
      ? renderLadderViewer(ladder)
      : empty(
        decodeSkipped
          ? "Ladder decode was skipped in browser summary."
          : "No decoded ladder structure found for this program.",
      ),
  ].join("");
}

function summaryIndicatesSkippedPayloadDecode(summary) {
  return (summary.warnings ?? []).some((warning) => (
    warning.includes("payload decode skipped in browser summary")
    || warning.includes("payload and ladder decode skipped in browser summary")
  ));
}

function summaryIndicatesSkippedLadderDecode(summary) {
  return (summary.warnings ?? []).some((warning) => (
    warning.includes("ladder decode skipped in browser summary")
    || warning.includes("payload and ladder decode skipped in browser summary")
  ));
}

function renderVariables(variables) {
  if (!variables.length) {
    panels.variables.innerHTML = empty("No variables found.");
    return;
  }

  panels.variables.innerHTML = section("Variables", `
    <div class="table-wrap">
      <table class="data-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Address</th>
            <th>Type</th>
            <th>Description</th>
            <th>Comment</th>
            <th>Range</th>
            <th>Source</th>
          </tr>
        </thead>
        <tbody>${variables.map(renderVariableRow).join("")}</tbody>
      </table>
    </div>
  `);
}

function renderVariableRow(variable) {
  const address = variable.address
    ?? [variable.addressArea, variable.addressNumber]
      .filter((part) => part !== null && part !== undefined && part !== "")
      .join("");
  return `<tr>
    <td>${escapeHtml(value(variable.name))}</td>
    <td>${escapeHtml(value(address))}</td>
    <td>${escapeHtml(value(variable.dataType))}</td>
    <td>${escapeHtml(value(variable.description))}</td>
    <td>${escapeHtml(value(variable.comment))}</td>
    <td>${escapeHtml(value(variable.range))}</td>
    <td>${escapeHtml(value(variable.sourceRef ?? variable.formatVersion))}</td>
  </tr>`;
}

function renderHardware(hardware) {
  const bases = hardware.bases ?? [];
  const modules = hardware.modules ?? [];

  if (!bases.length && !modules.length) {
    panels.hardware.innerHTML = empty("No hardware modules found.");
    return;
  }

  const baseHtml = bases.length
    ? section("Bases", `
      <div class="table-wrap">
        <table class="data-table is-compact">
          <thead>
            <tr>
              <th>Base</th>
              <th>Slots</th>
            </tr>
          </thead>
          <tbody>${bases.map(renderBaseRow).join("")}</tbody>
        </table>
      </div>
    `)
    : "";

  const moduleHtml = modules.length
    ? section("Modules", `
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>Base</th>
              <th>Slot</th>
              <th>ID</th>
              <th>Subtype</th>
              <th>Name</th>
              <th>Comment</th>
              <th>Details</th>
            </tr>
          </thead>
          <tbody>${modules.map(renderModuleRow).join("")}</tbody>
        </table>
      </div>
    `)
    : "";

  panels.hardware.innerHTML = baseHtml + moduleHtml;
}

function renderBaseRow(base) {
  return `<tr>
    <td>${escapeHtml(value(base.base))}</td>
    <td>${escapeHtml(value(base.slotCount))}</td>
  </tr>`;
}

function renderModuleRow(module) {
  return `<tr>
    <td>${escapeHtml(value(module.base))}</td>
    <td>${escapeHtml(value(module.slot))}</td>
    <td>${escapeHtml(value(module.id))}</td>
    <td>${escapeHtml(value(module.subType))}</td>
    <td>${escapeHtml(value(module.name))}</td>
    <td>${escapeHtml(value(module.comment))}</td>
    <td>${renderModuleDetails(module)}</td>
  </tr>`;
}

function renderModuleDetails(module) {
  const rawDetails = module?.details;
  if (rawDetails === null || rawDetails === undefined || rawDetails === "") {
    return escapeHtml(value(rawDetails));
  }

  const hexDetails = parseHexDetails(rawDetails);
  if (!hexDetails) {
    return `<div class="module-details">
      <span class="module-details-value">Unrecognized details format</span>
      <details><summary>Show raw value</summary><code>${escapeHtml(String(rawDetails))}</code></details>
    </div>`;
  }

  const decoded = module.inputFilter
    ? `Input filter: ${module.inputFilter}`
    : null;
  const byteLabel = `${hexDetails.bytes.length} ${hexDetails.bytes.length === 1 ? "byte" : "bytes"}`;
  const zeroLabel = hexDetails.bytes.every((byte) => byte === "00") ? " · all 00" : "";
  const rawHex = hexDetails.bytes.join(" ");

  if (decoded) {
    return `<div class="module-details">
      <span class="module-details-value">${escapeHtml(decoded)}</span>
      <details><summary>Raw hex · ${escapeHtml(byteLabel)}</summary><code>${escapeHtml(rawHex)}</code></details>
    </div>`;
  }

  const previewBytes = hexDetails.bytes.slice(0, 16);
  const truncated = previewBytes.length < hexDetails.bytes.length;
  return `<div class="module-details">
    <span class="module-details-value">Hex payload · ${escapeHtml(byteLabel + zeroLabel)}</span>
    <code class="module-details-preview">${escapeHtml(previewBytes.join(" ") + (truncated ? " …" : ""))}</code>
    ${truncated ? `<details><summary>Show all raw hex</summary><code>${escapeHtml(rawHex)}</code></details>` : ""}
  </div>`;
}

function parseHexDetails(rawDetails) {
  const compact = String(rawDetails).replace(/\s+/g, "");
  if (!compact || compact.length % 2 !== 0 || !/^[0-9a-f]+$/i.test(compact)) {
    return null;
  }

  return {
    bytes: compact.match(/.{2}/g).map((byte) => byte.toUpperCase()),
  };
}

function renderLadderViewer(ladder) {
  const unknownCount = ladder.unknownRecords?.length ?? 0;
  const hasDrawableLadder =
    ladder.cells.length ||
    ladder.rungComments?.length ||
    ladder.outputComments?.length ||
    ladder.verticalLines.length ||
    ladder.horizontalLines.length;
  return section("Ladder", [
    `<div class="ladder-viewer-toolbar">
      <div class="ladder-meta">${escapeHtml(String(ladder.rungs.length))} rungs · ${escapeHtml(String(ladder.cells.length))} cells · ${escapeHtml(String(ladder.branchGroups?.length ?? ladder.verticalLines.length))} branch groups · ${escapeHtml(String(ladder.verticalLines.length))} vertical segments · ${escapeHtml(String(ladder.horizontalLines.length))} horizontal segments · ${escapeHtml(String(ladder.rungComments?.length ?? 0))} rung comments · ${escapeHtml(String(ladder.outputComments?.length ?? 0))} output comments</div>
      <label class="ladder-toggle ${unknownCount ? "" : "is-disabled"}">
        <input type="checkbox" data-ladder-unknown-toggle ${unknownCount ? "" : "disabled"}>
        <span>Unknown markers${unknownCount ? ` (${escapeHtml(String(unknownCount))})` : ""}</span>
      </label>
    </div>`,
    hasDrawableLadder ? `<div class="ladder-scroll" data-ladder-table-host>${ladderTable(ladder, false)}</div>` : empty("No positioned ladder cells found."),
  ].join(""));
}

function bindLadderViewer(root, ladder) {
  if (!ladder) {
    return;
  }

  const toggle = root.querySelector("[data-ladder-unknown-toggle]");
  const tableHost = root.querySelector("[data-ladder-table-host]");
  if (!toggle || !tableHost) {
    return;
  }

  toggle.addEventListener("change", () => {
    tableHost.innerHTML = ladderTable(ladder, toggle.checked);
  });
}

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
  return `<div class="ladder-rung-comment-text" title="${escapeHtml(`0x${comment.offset.toString(16)} @ ${comment.rawX},${comment.rawY}`)}">${escapeHtml(comment.text)}</div>`;
}

function renderOutputComment(comment) {
  return `<div class="ladder-output-comment-text" title="${escapeHtml(`0x${comment.offset.toString(16)} @ ${comment.rawX},${comment.rawY}`)}">${escapeHtml(comment.text)}</div>`;
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
    <div class="ladder-command-cell-token is-mnemonic" title="${escapeHtml(title)}">${escapeHtml(cell.value || cell.kind)}</div>
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
  return `<div class="${classes.join(" ")}" title="${escapeHtml(title)}">
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

function renderNetworks(networks, cnet, fenet) {
  const networkHtml = networks.length
    ? networks
        .map((network) => section(
          value(network.name, "<unnamed network>"),
          [
            details([
              ["Type", value(network.typeName)],
              ["Network type", value(network.networkType)],
              ["Modules", network.modules.length],
            ]),
            network.modules.length
              ? `<div class="list">${network.modules.map((module) => listItem(value(module.name, "<unnamed module>"), details([
                  ["Type", value(module.typeName)],
                  ["ID", value(module.id)],
                  ["Base", value(module.base)],
                  ["Slot", value(module.slot)],
                  ["Alias", value(module.alias)],
                  ["Description", value(module.description)],
                ]))).join("")}</div>`
              : "",
          ].join(""),
        ))
        .join("")
    : empty("No network records found.");

  const cnetHtml = cnet.length
    ? section("Cnet", `<div class="list">${cnet.map(renderCnet).join("")}</div>`)
    : "";
  const fenetHtml = fenet.length
    ? section("FEnet", `<div class="list">${fenet.map(renderFenet).join("")}</div>`)
    : "";

  panels.networks.innerHTML = networkHtml + cnetHtml + fenetHtml;
}

function renderCnet(config) {
  const ports = config.ports
    .map((port, index) => listItem(`Port ${index + 1}`, details([
      ["Station", value(port.stationNo)],
      ["Mode", value(port.mode)],
      ["Baud", value(port.baudRate)],
      ["Data bits", value(port.dataBits)],
      ["Stop bits", value(port.stopBits)],
      ["Parity", value(port.parity)],
      ["DI", value(port.diAddress)],
      ["DO", value(port.doAddress)],
      ["AI", value(port.aiAddress)],
      ["AO", value(port.aoAddress)],
    ])))
    .join("");

  return listItem(`Cnet type ${value(config.typeCode)}`, details([
    ["Station", value(config.stationNo)],
    ["Base", value(config.base)],
    ["Slot", value(config.slot)],
    ["Subtype", value(config.subType)],
    ["Ports", config.ports.length],
  ]) + `<div class="list">${ports}</div>`);
}

function renderFenet(config) {
  return listItem(`FEnet type ${value(config.typeCode)}`, details([
    ["Station", value(config.stationNo)],
    ["Base", value(config.base)],
    ["Slot", value(config.slot)],
    ["Subtype", value(config.subType)],
    ["IP", value(config.ipAddress)],
    ["Subnet", value(config.subnet)],
    ["Gateway", value(config.gateway)],
    ["DNS", value(config.dns)],
  ]));
}

function renderParameters(summary) {
  const parameters = summary.parameters ?? [];
  if (!parameters.length) {
    panels.parameters.innerHTML = empty("No parameters found.");
    return;
  }

  panels.parameters.innerHTML = `
    <div class="parameter-layout">
      <aside class="parameter-sidebar" aria-label="Parameter list">
        ${parameters.map((parameter, index) => `<button class="parameter-button ${index === 0 ? "active" : ""}" type="button" data-parameter-index="${index}">
          <span>${escapeHtml(value(parameter.parameterType, `<parameter ${index + 1}>`))}</span>
          <small>${escapeHtml(`${parameter.sections.length} sections`)}</small>
        </button>`).join("")}
      </aside>
      <div id="parameter-detail" class="parameter-detail"></div>
    </div>`;

  const detail = panels.parameters.querySelector("#parameter-detail");
  const buttons = panels.parameters.querySelectorAll(".parameter-button");
  const showParameter = (index) => {
    buttons.forEach((button) => {
      button.classList.toggle("active", Number(button.dataset.parameterIndex) === index);
    });
    detail.innerHTML = renderParameterDetail(summary, parameters[index], index);
  };

  buttons.forEach((button) => {
    button.addEventListener("click", () => showParameter(Number(button.dataset.parameterIndex)));
  });

  showParameter(0);
}

function renderParameterDetail(summary, parameter, index) {
  const parameterType = value(parameter.parameterType, `<parameter ${index + 1}>`);
  const output = [
    section(parameterType, [
      details([
        ["Sections", parameter.sections.length],
        ["Attributes", parameter.attributes.length],
      ]),
      renderAttributeTable(parameter.attributes),
    ].join("")),
    ...parameter.sections.map((parameterSection) => {
      const module = parameter.parameterType === "IO PARAMETER"
        ? moduleFromParameterSection(parameterSection, summary.hardware?.modules ?? [])
        : null;
      return renderParameterSection(
        parameterSection,
        parameter.parameterType === "BASIC PARAMETER",
        module,
      );
    }),
  ];

  if (parameter.parameterType === "BASIC PARAMETER") {
    const timerAreaCharts = renderTimerAreaCharts(parameter);
    if (timerAreaCharts) {
      output.splice(1, 0, timerAreaCharts);
    }
  }

  const typeIndex = (summary.parameters ?? [])
    .slice(0, index + 1)
    .filter((item) => item.parameterType === parameter.parameterType)
    .length - 1;

  if (parameter.parameterType?.includes("FENET") && summary.safetyComm) {
    output.push(renderSafetyComm(summary.safetyComm));
  }
  if (parameter.parameterType === "HSC PARAMETER" && summary.hsc?.[typeIndex]) {
    output.push(...renderHscParameter(summary.hsc[typeIndex]));
  }
  if (parameter.parameterType === "POSITION PARAMETER" && summary.position?.[typeIndex]) {
    output.push(...renderPositionParameter(summary.position[typeIndex]));
  }
  if (parameter.parameterType === "IO PARAMETER") {
    output.push(...renderIoParameter(summary.cnet ?? [], summary.fenet ?? []));
  }
  if (parameter.parameterType === "PID CAL PARAMETER" && summary.pid?.calculation?.[typeIndex]) {
    output.push(...renderPidCalculation(summary.pid.calculation[typeIndex]));
  }
  if (parameter.parameterType === "PID TUNE PARAMETER" && summary.pid?.tuning?.[typeIndex]) {
    output.push(...renderPidTuning(summary.pid.tuning[typeIndex]));
  }

  return output.join("");
}

function renderParameterSection(parameterSection, groupIndexed, module = null) {
  const rows = [["Children", parameterSection.childCount]];
  if (parameterSection.text !== null && parameterSection.text !== undefined) {
    rows.push(["Text", parameterSection.text]);
  }
  return section(parameterSection.name, [
    details(rows),
    renderAttributeTable(parameterSection.attributes, groupIndexed, module),
  ].join(""));
}

function renderAttributeTable(attributes, groupIndexed = false, module = null) {
  if (!attributes?.length) {
    return "";
  }

  const rows = groupIndexed
    ? groupParameterAttributes(attributes)
    : attributes.map((attribute) => ({
        name: attribute.name,
        value: attribute.value,
        indexed: false,
      }));

  return `<div class="table-wrap parameter-attributes">
    <table class="data-table is-compact">
      <thead><tr><th>Attribute</th><th>Value</th></tr></thead>
      <tbody>${rows.map((row) => `<tr class="${row.indexed ? "is-indexed" : ""}">
        <td>${escapeHtml(row.name)}</td>
        <td>${row.name === "Details" && module
          ? renderModuleDetails(module)
          : escapeHtml(value(row.value))}</td>
      </tr>`).join("")}</tbody>
    </table>
  </div>`;
}

function moduleFromParameterSection(parameterSection, modules) {
  if (parameterSection.name !== "Module") {
    return null;
  }

  const attributes = Object.fromEntries(
    parameterSection.attributes.map((attribute) => [attribute.name, attribute.value]),
  );
  const matchingModule = modules.find((module) =>
    String(module.base) === attributes.Base
    && String(module.slot) === attributes.Slot
    && module.name === attributes.Name,
  );

  return {
    ...matchingModule,
    name: matchingModule?.name ?? attributes.Name,
    details: matchingModule?.details ?? attributes.Details,
  };
}

function groupParameterAttributes(attributes) {
  const groups = [];

  attributes.forEach((attribute) => {
    const match = attribute.name.match(/^(.*)_(\d+)$/);
    if (!match || !match[1]) {
      groups.push({
        name: attribute.name,
        scalar: attribute.value,
        values: [],
      });
      return;
    }

    const [, name, rawIndex] = match;
    let group = groups.find((item) => item.name === name && item.scalar === undefined);
    if (!group) {
      group = { name, scalar: undefined, values: [] };
      groups.push(group);
    }
    group.values.push([Number(rawIndex), attribute.value]);
  });

  return groups.flatMap((group) => {
    if (group.scalar !== undefined) {
      return [{ name: group.name, value: group.scalar, indexed: false }];
    }

    group.values.sort(([left], [right]) => left - right);
    return [
      { name: group.name, value: `${group.values.length} values`, indexed: false },
      ...group.values.map(([index, attributeValue]) => ({
        name: `[${index}]`,
        value: attributeValue,
        indexed: true,
      })),
    ];
  });
}

function renderTimerAreaCharts(parameter) {
  const attributes = parameter.sections
    .flatMap((parameterSection) => parameterSection.attributes);
  const areaRangeGroups = collectTimerRangeGroups(attributes, (name) => {
    const match = name.match(/^T_(.+)_AREA_RANGE_(START|END)$/);
    if (!match) {
      return null;
    }
    const [, timerCode, bound] = match;
    return {
      bound,
      field: `T_${timerCode}_AREA_RANGE`,
      groupLabel: "RANGE",
      timerCode,
    };
  });
  const latchGroups = collectTimerRangeGroups(attributes, (name) => {
    const match = name.match(/^T_(.+)_AREA_LATCH(\d+)_(START|END)$/);
    if (!match) {
      return null;
    }
    const [, timerCode, latch, bound] = match;
    return {
      bound,
      field: `T_${timerCode}_AREA_LATCH${latch}`,
      groupLabel: `LATCH${latch}`,
      timerCode,
    };
  });

  return [
    renderTimerRangeChart("Timer Area Range", areaRangeGroups),
    renderTimerRangeChart("Timer Latch Allocation", latchGroups),
  ].filter(Boolean).join("");
}

function collectTimerRangeGroups(attributes, parseName) {
  const groups = new Map();

  attributes.forEach((attribute) => {
    const parsed = parseName(attribute.name);
    const numericValue = Number(attribute.value);
    if (!parsed || !Number.isFinite(numericValue)) {
      return;
    }

    if (!groups.has(parsed.groupLabel)) {
      groups.set(parsed.groupLabel, new Map());
    }
    const timerRanges = groups.get(parsed.groupLabel);
    if (!timerRanges.has(parsed.timerCode)) {
      timerRanges.set(parsed.timerCode, {
        field: parsed.field,
        timerCode: parsed.timerCode,
      });
    }
    timerRanges.get(parsed.timerCode)[parsed.bound.toLowerCase()] = numericValue;
  });

  return [...groups.entries()]
    .sort(([left], [right]) => left.localeCompare(right, undefined, { numeric: true }))
    .map(([label, timerRanges]) => ({
      label,
      ranges: [...timerRanges.values()]
        .filter((range) => range.start !== undefined
          && range.end !== undefined
          && range.end >= range.start)
        .sort((left, right) => left.start - right.start),
    }))
    .filter((group) => group.ranges.length);
}

function renderTimerRangeChart(title, groups) {
  if (!groups.length) {
    return "";
  }

  const scaleStart = Math.min(...groups.flatMap((group) => group.ranges.map((range) => range.start)));
  const scaleEnd = Math.max(...groups.flatMap((group) => group.ranges.map((range) => range.end)));
  const scaleSize = scaleEnd - scaleStart + 1;

  const rows = groups.map((group) => {
    const segments = group.ranges.map((range, colorIndex) => {
      const left = ((range.start - scaleStart) / scaleSize) * 100;
      const width = ((range.end - range.start + 1) / scaleSize) * 100;
      const label = timerCodeLabel(range.timerCode);
      const count = range.end - range.start + 1;
      return {
        ...range,
        colorIndex,
        count,
        label,
        left,
        width,
      };
    });

    return `<div class="timer-range-row">
      <div class="timer-range-row-label">${escapeHtml(group.label)}</div>
      <div class="timer-range-track" role="img" aria-label="${escapeHtml(`${group.label} timer allocation from ${scaleStart} to ${scaleEnd}`)}">
        ${segments.map((segment) => `<span class="timer-range-segment timer-range-color-${segment.colorIndex % 4}" style="left:${segment.left.toFixed(4)}%;width:${segment.width.toFixed(4)}%" title="${escapeHtml(`${segment.field}: ${segment.start}-${segment.end} (${segment.count})`)}"></span>`).join("")}
      </div>
      <div class="timer-range-legend">
        ${segments.map((segment) => `<div class="timer-range-legend-item">
          <span class="timer-range-swatch timer-range-color-${segment.colorIndex % 4}"></span>
          <span><strong>${escapeHtml(segment.label)}</strong> ${escapeHtml(`${segment.start}-${segment.end}`)} <small>${escapeHtml(`${segment.count} points`)}</small></span>
          <code>${escapeHtml(segment.field)}</code>
        </div>`).join("")}
      </div>
    </div>`;
  }).join("");

  return section(title, `
    <div class="timer-range-chart">
      <div class="timer-range-scale"><span>${escapeHtml(scaleStart)}</span><span>${escapeHtml(scaleEnd)}</span></div>
      ${rows}
    </div>`);
}

function timerCodeLabel(timerCode) {
  return {
    "100US": "100 us",
    "001MS": "1 ms",
    "010MS": "10 ms",
    "100MS": "100 ms",
  }[timerCode] ?? timerCode;
}

function renderSafetyComm(safety) {
  const channelRows = safety.channels.map((channel) => [
    channel.name,
    `${value(channel.address)} (type=${value(channel.dataType)} | size=${value(channel.size)} | raw=${value(channel.addr)})`,
  ]);

  return section("Safety Comm", details([
    ["Rcv wait", value(safety.rcvWaitTime)],
    ["Retrans", value(safety.retransTime)],
    ["Glofa sockets", value(safety.glofaSocketCount)],
    ["Driver type", value(safety.driverType)],
    ["IP", `${value(safety.ipAddress)} (${value(safety.ipAddressRaw)})`],
    ["Gateway", `${value(safety.gateway)} (${value(safety.gatewayRaw)})`],
    ["Subnet", `${value(safety.subnet)} (${value(safety.subnetRaw)})`],
    ["Host table", value(safety.enableHostTable)],
    ...channelRows,
  ]));
}

function renderHscParameter(parameter) {
  return [
    section("HSC Payload", details([
      ["Payload ASCII", value(parameter.payloadAscLength)],
      ["Payload bytes", parameter.payloadBytes],
      ["Initial unknown", value(parameter.initialUnknownNibble)],
      ["Channels", parameter.channels.length],
    ])),
    ...parameter.channels.map((channel) => section(`HSC Channel ${channel.channel}`, details([
      ["Counter mode", modeWithRaw(channel.counterMode, channel.counterModeRaw)],
      ["Pulse input mode", modeWithRaw(channel.pulseInputMode, channel.pulseInputModeRaw)],
      ["Compare output mode", modeWithRaw(channel.compareOutputMode, channel.compareOutputModeRaw)],
      ["Internal preset", formatHex(channel.internalPreset, 2)],
      ["External preset", formatHex(channel.externalPreset, 2)],
      ["Ring counter max", formatHex(channel.ringCounterMax, 8)],
      ["Compare output min", formatHex(channel.compareOutputMin, 8)],
      ["Compare output max", formatHex(channel.compareOutputMax, 8)],
      ["Unit time", formatHex(channel.unitTimeMs, 4)],
      ["Pulses per revolution", formatHex(channel.pulsesPerRevolution, 4)],
      ["Raw bytes", channel.rawBytes],
    ]))),
  ];
}

function renderPositionParameter(parameter) {
  return [
    section("Position Parameter", details([
      ["Axis count", value(parameter.axisCount)],
      ["Parsed axes", parameter.axes.length],
    ])),
    ...parameter.axes.map((axis) => {
      const axisParameter = axis.parameter;
      const rows = [
        ["Step count", value(axis.stepCount)],
        ["Steps parsed", axis.parsedSteps],
      ];
      if (axisParameter) {
        rows.push(
          ["Bias velocity", value(axisParameter.biasVelocity)],
          ["Velocity limit", value(axisParameter.velocityLimit)],
          ["Accel times", formatOptionArray(axisParameter.accelTimes)],
          ["Decel times", formatOptionArray(axisParameter.decelTimes)],
          ["Soft upper limit", value(axisParameter.softUpperLimit)],
          ["Soft lower limit", value(axisParameter.softLowerLimit)],
          ["Backlash compensation", value(axisParameter.backlashCompensation)],
          ["S-curve ratio", value(axisParameter.sCurveRatio)],
          ["Use limit", value(axisParameter.useLimit)],
          ["Pulse output mode", value(axisParameter.pulseOutputMode)],
          ["Orientation", value(axisParameter.orientation)],
          ["Return velocity", `high=${value(axisParameter.returnVelocityHigh)} | low=${value(axisParameter.returnVelocityLow)}`],
          ["Return timing", `accel=${value(axisParameter.returnAccelTime)} | decel=${value(axisParameter.returnDecelTime)} | dwell=${value(axisParameter.returnDwellTime)}`],
          ["Return mode", `policy=${value(axisParameter.returnPolicy)} | direction=${value(axisParameter.returnDirection)}`],
          ["Jog timing", `accel=${value(axisParameter.jogAccelTime)} | decel=${value(axisParameter.jogDecelTime)} | inching=${value(axisParameter.inchingTime)}`],
          ["Jog velocity", `high=${value(axisParameter.jogVelocityHigh)} | low=${value(axisParameter.jogVelocityLow)}`],
          ["Interpolation method", value(axisParameter.interpolationMethod)],
        );
      } else {
        rows.push(["Axis parameter", "<missing>"]);
      }
      if (axis.firstStep) {
        rows.push(
          ["First step target", value(axis.firstStep.targetPosition)],
          ["First step velocity", value(axis.firstStep.operationVelocity)],
          ["First step dwell", value(axis.firstStep.dwellTime)],
          ["First step mode", value(axis.firstStep.operationMode)],
        );
      }
      return section(`${axis.axisName} Axis`, details(rows));
    }),
  ];
}

function renderIoParameter(cnet, fenet) {
  return [
    ...cnet.flatMap((config, configIndex) => [
      section(`Cnet Module ${configIndex + 1}`, details([
        ["Station", value(config.stationNo)],
        ["Base", value(config.base)],
        ["Slot", value(config.slot)],
        ["Type", value(config.typeCode)],
        ["Subtype", value(config.subType)],
        ["Ports", config.ports.length],
      ])),
      ...config.ports.map((port, portIndex) => section(`Cnet Port ${portIndex + 1}`, details([
        ["Station", value(port.stationNo)],
        ["Mode", modeWithRaw(port.mode, port.modeRaw)],
        ["Serial", `baud=${value(port.baudRate)} (${value(port.bps)}) | data=${modeWithRaw(port.dataBits, port.dataBitRaw)} | stop=${modeWithRaw(port.stopBits, port.stopBitRaw)} | parity=${modeWithRaw(port.parity, port.parityRaw)}`],
        ["Timeout", `rx=${value(port.rxTimeout)} | char=${value(port.charTimeout)} | inter-char=${value(port.interCharTimeout)}`],
        ["Driver type", value(port.driverType)],
        ["DI", formatCnetDevice(port, "di")],
        ["DO", formatCnetDevice(port, "do")],
        ["AI", formatCnetDevice(port, "ai")],
        ["AO", formatCnetDevice(port, "ao")],
        ["Terminating resistor", value(port.terminatingResister)],
        ["Repeater", value(port.repeater)],
      ]))),
    ]),
    ...fenet.map((config, index) => section(`FEnet Module ${index + 1}`, details([
      ["Station", value(config.stationNo)],
      ["Base", value(config.base)],
      ["Slot", value(config.slot)],
      ["Type", value(config.typeCode)],
      ["Subtype", value(config.subType)],
      ["IP", value(config.ipAddress)],
      ["Subnet", value(config.subnet)],
      ["Gateway", value(config.gateway)],
      ["DNS", value(config.dns)],
      ["Secondary IP", value(config.ipAddress2)],
      ["Secondary subnet", value(config.subnet2)],
      ["Secondary gateway", value(config.gateway2)],
      ["Secondary DNS", value(config.dns2)],
      ["DHCP", value(config.dhcp)],
      ["Driver type", value(config.driverType)],
      ["Rcv wait", value(config.rcvWaitTime)],
      ["Client wait", value(config.clientWaitTime)],
      ["Glofa sockets", value(config.glofaSocketCount)],
    ]))),
  ];
}

function renderPidCalculation(parameter) {
  return [
    section("PID Calculation", details([
      ["Loops", parameter.loops.length],
      ["Header", formatOptionArray(parameter.header)],
      ["Parameter size", value(parameter.parameterSize)],
      ["Set PID out", value(parameter.setPidOut)],
      ["Set direction", value(parameter.setDirection)],
      ["Prevent anti windup", value(parameter.preventAntiWindup)],
      ["Control method", `P=${value(parameter.proportionalControlMethod)} | D=${value(parameter.differentialControlMethod)}`],
      ["Permit PWM", value(parameter.permitPwm)],
    ])),
    ...parameter.loops.map((loop) => section(`PID Calculation Loop ${loop.loopIndex}`, details([
      ["Target value", value(loop.targetValue)],
      ["Scan time", value(loop.scanTime)],
      ["Gain", `P=${formatDecimalPair(loop.proportionalGain)} | I=${formatDecimalPair(loop.integralGain)} | D=${formatDecimalPair(loop.differentialGain)}`],
      ["MV", `min=${value(loop.mvMin)} | max=${value(loop.mvMax)} | manual=${value(loop.mvManual)} | limit=${value(loop.mvLimit)}`],
      ["PV", `min=${value(loop.pvMin)} | max=${value(loop.pvMax)} | limit=${value(loop.pvLimit)} | tracking=${value(loop.pvTrackingSetValue)}`],
      ["Dead band", value(loop.deadBand)],
      ["PWM", `forward=${value(loop.forwardPwm)} (${value(loop.forwardPwmAddress)}) | period=${value(loop.pwmOutPeriod)}`],
    ]))),
  ];
}

function renderPidTuning(parameter) {
  return [
    section("PID Tuning", details([
      ["Loops", parameter.loops.length],
      ["Set direction", value(parameter.setDirection)],
      ["Permit PWM", value(parameter.permitPwm)],
      ["Checksum", value(parameter.checksum)],
      ["Footer", formatOptionArray(parameter.footer)],
    ])),
    ...parameter.loops.map((loop) => section(`PID Tuning Loop ${loop.loopIndex}`, details([
      ["Target value", value(loop.targetValue)],
      ["Scan time", value(loop.scanTime)],
      ["MV", `min=${value(loop.mvMin)} | max=${value(loop.mvMax)}`],
      ["PWM", `point=${value(loop.setPwmAtPoint)} (${value(loop.setPwmAtAddress)}) | period=${value(loop.outPeriod)}`],
      ["Hysteresis", value(loop.hysteresis)],
    ]))),
  ];
}

function modeWithRaw(mode, raw) {
  return `${value(mode)} (${value(raw, "?")})`;
}

function formatCnetDevice(port, prefix) {
  const property = (suffix) => `${prefix}${suffix}`;
  return `address=${value(port[property("Address")])} | device=${value(port[property("Device")])} (${value(port[property("DeviceType")])}) | data=${value(port[property("DataType")])} | size=${value(port[property("Size")])} | addr=${value(port[property("Addr")])}`;
}

function formatHex(item, width) {
  if (item === null || item === undefined) {
    return "<none>";
  }
  const unsigned = item < 0 ? item >>> 0 : item;
  return `0x${unsigned.toString(16).toUpperCase().padStart(width, "0")} (${item})`;
}

function formatOptionArray(items) {
  return (items ?? []).map((item) => value(item)).join(", ");
}

function formatDecimalPair(items) {
  return `${value(items?.[0])}.${value(items?.[1])}`;
}

function clearPanels() {
  Object.values(panels).forEach((panel) => {
    panel.innerHTML = "";
  });
}

function setStatus(message, isError = false) {
  statusEl.textContent = message;
  statusEl.classList.toggle("error", isError);
}

function metric(label, valueText) {
  return `<div class="metric"><span class="label">${escapeHtml(label)}</span><span class="value">${escapeHtml(String(valueText))}</span></div>`;
}

function section(title, content) {
  return `<div class="section"><h2>${escapeHtml(title)}</h2>${content}</div>`;
}

function listItem(title, content) {
  return `<div class="list-item"><h3>${escapeHtml(title)}</h3>${content}</div>`;
}

function details(rows) {
  return `<dl class="details">${rows
    .map(([key, rowValue]) => `<dt>${escapeHtml(key)}</dt><dd>${escapeHtml(String(rowValue))}</dd>`)
    .join("")}</dl>`;
}

function empty(message) {
  return `<div class="section muted">${escapeHtml(message)}</div>`;
}

function value(item, fallback = "<none>") {
  return item === null || item === undefined || item === "" ? fallback : item;
}

function formatBytes(bytes) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = units.shift();
  while (value >= 1024 && units.length) {
    value /= 1024;
    unit = units.shift();
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
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
