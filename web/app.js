import init, { parse_xgwx } from "./pkg/libxgwx.js";

const fileInput = document.querySelector("#file-input");
const dropZone = document.querySelector("#drop-zone");
const statusEl = document.querySelector("#status");
const panels = {
  summary: document.querySelector("#summary"),
  programs: document.querySelector("#programs"),
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
  renderPrograms(summary.programs, summary.ladder ?? []);
  renderNetworks(summary.networks, summary.cnet, summary.fenet);
  renderParameters(summary);
}

function renderSummary(summary, file) {
  const warnings = summary.warnings ?? [];
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
    `<div class="grid">${metric("Programs", summary.counts.programs)}${metric("Networks", summary.counts.networks)}${metric("Modules", summary.counts.modules)}${metric("Variables", value(summary.counts.variables))}${metric("Payloads", summary.counts.decodedPayloads)}${metric("Ladder", summary.counts.ladderPrograms)}</div>`,
    warnings.length
      ? section("Warnings", `<div class="list">${warnings.map((warning) => `<div class="list-item warning">${escapeHtml(warning)}</div>`).join("")}</div>`)
      : "",
  ].join("");
}

function renderPrograms(programs, ladderPrograms) {
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
            : "No ladder data";
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
    detail.innerHTML = renderProgramDetail(programs[index], ladder, index);
    bindLadderViewer(detail, ladder);
  };

  buttons.forEach((button) => {
    button.addEventListener("click", () => showProgram(Number(button.dataset.programIndex)));
  });

  showProgram(0);
}

function renderProgramDetail(program, ladder, index) {
  return [
    section(value(program.name, `<program ${index + 1}>`), details([
      ["Task", value(program.task)],
      ["Kind", value(program.kind)],
      ["Version", value(program.version)],
      ["Object ID", value(program.objectId)],
      ["Comment", value(program.comment)],
    ])),
    ladder ? renderLadderViewer(ladder) : empty("No decoded ladder structure found for this program."),
  ].join("");
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

  const title = [cell.kind, cell.value, cell.operands?.join(", ")].filter(Boolean).join(" ");
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
  const title = [cell.kind, cell.value, cell.operands?.join(", ")].filter(Boolean).join(" ");
  const classes = ["ladder-token", ladderCellClass(cell)];
  return `<div class="${classes.join(" ")}" title="${escapeHtml(title)}">
    ${ladderCellBody(cell)}
  </div>`;
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
  const hsc = summary.hsc.length
    ? section("HSC", `<div class="list">${summary.hsc.map((parameter, index) => listItem(`HSC parameter ${index + 1}`, [
        details([["Payload bytes", parameter.payloadBytes], ["Channels", parameter.channels.length]]),
        `<div class="list">${parameter.channels.map((channel) => listItem(`Channel ${channel.channel}`, details([
          ["Counter mode", value(channel.counterMode)],
          ["Pulse input", value(channel.pulseInputMode)],
          ["Compare output", value(channel.compareOutputMode)],
          ["Ring max", value(channel.ringCounterMax)],
          ["Compare min", value(channel.compareOutputMin)],
          ["Compare max", value(channel.compareOutputMax)],
          ["Unit time ms", value(channel.unitTimeMs)],
          ["Pulses/rev", value(channel.pulsesPerRevolution)],
        ]))).join("")}</div>`,
      ].join(""))).join("")}</div>`)
    : "";

  const position = summary.position.length
    ? section("Position", `<div class="list">${summary.position.map((parameter, index) => listItem(`Position parameter ${index + 1}`, [
        details([["Axis count", value(parameter.axisCount)], ["Parsed axes", parameter.axes.length]]),
        `<div class="list">${parameter.axes.map((axis) => listItem(`${axis.axisName} axis`, details([
          ["Step count", value(axis.stepCount)],
          ["Parsed steps", axis.parsedSteps],
        ]))).join("")}</div>`,
      ].join(""))).join("")}</div>`)
    : "";

  const pid = section("PID", details([
    ["CAL parameters", summary.pid.calParameters],
    ["TUNE parameters", summary.pid.tuneParameters],
    ["CAL loops", summary.pid.calLoops],
    ["TUNE loops", summary.pid.tuneLoops],
  ]));

  panels.parameters.innerHTML = hsc + position + pid;
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
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
