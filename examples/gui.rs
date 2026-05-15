use std::env;
use std::error::Error;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use xgwx::{
    LadderCell, LadderCoil, LadderContact, LadderElementKind, LadderHorizontalLine,
    LadderProgramData, LadderVerticalLine, XgwxDocument, XgwxError,
};

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "usage: cargo run --features gui --example gui -- <file.xgwx>",
        )
    })?;
    let doc = XgwxDocument::from_path(&path)?;
    let project = doc.project_info();
    let app = LadderGui::new(
        path,
        project.name.unwrap_or_else(|| "<unnamed>".to_owned()),
        project
            .file_version
            .unwrap_or_else(|| "<unknown>".to_owned()),
        doc.ladder_programs(),
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };
    eframe::run_native("XGWX Ladder", options, Box::new(|_| Ok(Box::new(app))))?;
    Ok(())
}

struct LadderGui {
    path: String,
    project_name: String,
    file_version: String,
    programs: Vec<Result<LadderProgramData, XgwxError>>,
    selected_program: usize,
    zoom: f32,
    show_offsets: bool,
}

impl LadderGui {
    fn new(
        path: String,
        project_name: String,
        file_version: String,
        programs: Vec<Result<LadderProgramData, XgwxError>>,
    ) -> Self {
        Self {
            path,
            project_name,
            file_version,
            programs,
            selected_program: 0,
            zoom: 1.0,
            show_offsets: false,
        }
    }
}

impl eframe::App for LadderGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&self.project_name);
                ui.label(format!("FileVer {}", self.file_version));
                ui.label(&self.path);
            });
        });

        egui::SidePanel::left("programs")
            .resizable(true)
            .default_width(240.0)
            .show(ctx, |ui| {
                ui.heading("Programs");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, program) in self.programs.iter().enumerate() {
                        let label = match program {
                            Ok(program) => program
                                .program_name
                                .as_deref()
                                .unwrap_or("<unnamed>")
                                .to_owned(),
                            Err(_) => format!("Program {} decode error", index + 1),
                        };
                        if ui
                            .selectable_label(self.selected_program == index, label)
                            .clicked()
                        {
                            self.selected_program = index;
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut self.zoom, 0.5..=2.5).text("Zoom"));
                ui.checkbox(&mut self.show_offsets, "Offsets");
            });
            ui.separator();

            match self.programs.get(self.selected_program) {
                Some(Ok(program)) => {
                    ui.horizontal(|ui| {
                        ui.heading(program.program_name.as_deref().unwrap_or("<unnamed>"));
                        ui.label(format!(
                            "{} rungs, {} cells, {} vertical segments, {} horizontal segments",
                            program.structure.rungs.len(),
                            program
                                .structure
                                .rungs
                                .iter()
                                .map(|rung| rung.cells.len())
                                .sum::<usize>(),
                            program.structure.vertical_lines.len(),
                            program.structure.horizontal_lines.len()
                        ));
                    });
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            render_ladder(ui, program, self.zoom, self.show_offsets)
                        });
                }
                Some(Err(error)) => {
                    ui.colored_label(Color32::RED, format!("Decode error: {error}"));
                }
                None => {
                    ui.label("No ladder programs found.");
                }
            }
        });
    }
}

fn render_ladder(ui: &mut egui::Ui, program: &LadderProgramData, zoom: f32, show_offsets: bool) {
    let Some(layout) = LadderLayout::new(program, zoom) else {
        ui.label("No positioned ladder cells found.");
        return;
    };

    let (rect, _) = ui.allocate_exact_size(layout.size, Sense::hover());
    let painter = ui.painter_at(rect);
    let colors = LadderColors::default();
    let origin = rect.min + Vec2::new(layout.left_margin, layout.top_margin);

    for (index, rung) in program.structure.rungs.iter().enumerate() {
        let y = origin.y + layout.y_for(rung.raw_y);
        painter.text(
            Pos2::new(rect.min.x + 8.0, y),
            Align2::LEFT_CENTER,
            format!("Rung {:03}", index + 1),
            FontId::monospace(13.0 * zoom),
            colors.label,
        );
        painter.line_segment(
            [
                Pos2::new(origin.x - 14.0 * zoom, y),
                Pos2::new(origin.x - 14.0 * zoom, y + layout.rung_gap * 0.35),
            ],
            Stroke::new(1.5, colors.rail),
        );
    }

    for horizontal in &program.structure.horizontal_lines {
        draw_horizontal_line(&painter, &layout, origin, horizontal, colors.wire);
    }

    for vertical in &program.structure.vertical_lines {
        draw_vertical(&painter, &layout, origin, vertical, colors.wire);
    }

    for rung in &program.structure.rungs {
        for cell in &rung.cells {
            draw_cell(&painter, &layout, origin, cell, colors, show_offsets);
        }
    }
}

struct LadderLayout {
    raw_xs: Vec<u8>,
    raw_ys: Vec<u8>,
    x_positions: Vec<f32>,
    y_positions: Vec<f32>,
    cell_height: f32,
    rung_gap: f32,
    left_margin: f32,
    top_margin: f32,
    char_width: f32,
    size: Vec2,
}

impl LadderLayout {
    fn new(program: &LadderProgramData, zoom: f32) -> Option<Self> {
        let mut raw_xs = program
            .structure
            .rungs
            .iter()
            .flat_map(|rung| rung.cells.iter().map(|cell| cell.raw_x))
            .chain(
                program
                    .structure
                    .vertical_lines
                    .iter()
                    .map(|line| line.raw_x),
            )
            .chain(
                program
                    .structure
                    .horizontal_lines
                    .iter()
                    .flat_map(|line| [line.raw_x_start, line.raw_x_end]),
            )
            .collect::<Vec<_>>();
        raw_xs.sort_unstable();
        raw_xs.dedup();

        let raw_ys = program
            .structure
            .rungs
            .iter()
            .map(|rung| rung.raw_y)
            .collect::<Vec<_>>();

        if raw_xs.is_empty() || raw_ys.is_empty() {
            return None;
        }

        let char_width = 8.0 * zoom;
        let cell_height = 28.0 * zoom;
        let rung_gap = 74.0 * zoom;
        let left_margin = 96.0 * zoom;
        let top_margin = 42.0 * zoom;

        let mut x_positions = Vec::with_capacity(raw_xs.len());
        let mut current = 0.0;
        for (index, raw_x) in raw_xs.iter().enumerate() {
            if index > 0 {
                let delta = raw_x.saturating_sub(raw_xs[index - 1]) as f32;
                current += (delta * 16.0 * zoom).clamp(82.0 * zoom, 210.0 * zoom);
            }
            x_positions.push(current);
        }

        for rung in &program.structure.rungs {
            let mut cells = rung.cells.iter().collect::<Vec<_>>();
            cells.sort_by_key(|cell| (cell.raw_x, cell.offset));
            for pair in cells.windows(2) {
                let left = pair[0];
                let right = pair[1];
                let Some(left_index) = raw_xs.iter().position(|raw_x| *raw_x == left.raw_x) else {
                    continue;
                };
                let Some(right_index) = raw_xs.iter().position(|raw_x| *raw_x == right.raw_x)
                else {
                    continue;
                };
                let required = x_positions[left_index] + cell_width(left, char_width) + 24.0 * zoom;
                if x_positions[right_index] < required {
                    let shift = required - x_positions[right_index];
                    for position in &mut x_positions[right_index..] {
                        *position += shift;
                    }
                }
            }
        }

        let y_positions = raw_ys
            .iter()
            .enumerate()
            .map(|(index, _)| index as f32 * rung_gap)
            .collect::<Vec<_>>();

        let width = left_margin
            + x_positions.last().copied().unwrap_or_default()
            + max_cell_width(program, char_width)
            + 80.0 * zoom;
        let height = top_margin + y_positions.last().copied().unwrap_or_default() + rung_gap;

        Some(Self {
            raw_xs,
            raw_ys,
            x_positions,
            y_positions,
            cell_height,
            rung_gap,
            left_margin,
            top_margin,
            char_width,
            size: Vec2::new(width.max(600.0), height.max(360.0)),
        })
    }

    fn x_for(&self, raw_x: u8) -> Option<f32> {
        self.raw_xs
            .iter()
            .position(|candidate| *candidate == raw_x)
            .and_then(|index| self.x_positions.get(index).copied())
    }

    fn y_for(&self, raw_y: u8) -> f32 {
        self.raw_ys
            .iter()
            .position(|candidate| *candidate == raw_y)
            .and_then(|index| self.y_positions.get(index).copied())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy)]
struct LadderColors {
    rail: Color32,
    wire: Color32,
    stroke: Color32,
    block_fill: Color32,
    contact_fill: Color32,
    text: Color32,
    label: Color32,
}

impl Default for LadderColors {
    fn default() -> Self {
        Self {
            rail: Color32::from_rgb(112, 125, 139),
            wire: Color32::from_rgb(70, 82, 96),
            stroke: Color32::from_rgb(22, 30, 38),
            block_fill: Color32::from_rgb(239, 244, 248),
            contact_fill: Color32::from_rgb(252, 253, 255),
            text: Color32::from_rgb(16, 24, 32),
            label: Color32::from_rgb(82, 94, 108),
        }
    }
}

fn draw_horizontal_line(
    painter: &egui::Painter,
    layout: &LadderLayout,
    origin: Pos2,
    horizontal: &LadderHorizontalLine,
    color: Color32,
) {
    let Some(left_x) = layout.x_for(horizontal.raw_x_start) else {
        return;
    };
    let Some(right_x) = layout.x_for(horizontal.raw_x_end) else {
        return;
    };
    let y = origin.y + layout.y_for(horizontal.raw_y);
    let start = origin.x + left_x;
    let end = origin.x + right_x;
    if end > start {
        painter.line_segment(
            [Pos2::new(start, y), Pos2::new(end, y)],
            Stroke::new(1.5, color),
        );
    }
}

fn draw_vertical(
    painter: &egui::Painter,
    layout: &LadderLayout,
    origin: Pos2,
    vertical: &LadderVerticalLine,
    color: Color32,
) {
    let Some(x) = layout.x_for(vertical.raw_x) else {
        return;
    };
    let y1 = origin.y + layout.y_for(vertical.raw_y_start);
    let y2 = origin.y + layout.y_for(vertical.raw_y_end);
    painter.line_segment(
        [Pos2::new(origin.x + x, y1), Pos2::new(origin.x + x, y2)],
        Stroke::new(1.5, color),
    );
}

fn draw_cell(
    painter: &egui::Painter,
    layout: &LadderLayout,
    origin: Pos2,
    cell: &LadderCell,
    colors: LadderColors,
    show_offsets: bool,
) {
    let Some(x) = layout.x_for(cell.raw_x) else {
        return;
    };
    let center = Pos2::new(origin.x + x, origin.y + layout.y_for(cell.raw_y));
    let width = cell_width(cell, layout.char_width);
    let rect = Rect::from_center_size(
        center + Vec2::new(width * 0.5, 0.0),
        Vec2::new(width, layout.cell_height),
    );
    let text = cell_text(cell);

    if cell.coil.is_some() {
        draw_coil(painter, rect, &text, colors, layout.char_width);
    } else {
        match cell.kind {
            LadderElementKind::DeviceRef | LadderElementKind::InternalRef => {
                draw_contact(painter, rect, cell, &text, colors, layout.char_width);
            }
            LadderElementKind::Operation
                if matches!(
                    cell.value.as_str(),
                    "OUT" | "OUTP" | "SET" | "RST" | "RESET" | "FF"
                ) =>
            {
                draw_coil(painter, rect, &text, colors, layout.char_width);
            }
            _ => {
                painter.rect(
                    rect,
                    4.0,
                    colors.block_fill,
                    Stroke::new(1.0, colors.stroke),
                );
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    text,
                    FontId::monospace(13.0 * layout.char_width / 8.0),
                    colors.text,
                );
            }
        }
    }

    if show_offsets {
        painter.text(
            Pos2::new(rect.left(), rect.bottom() + 4.0),
            Align2::LEFT_TOP,
            format!(
                "@0x{:04x} x={:02x} y={:02x}",
                cell.offset, cell.raw_x, cell.raw_y
            ),
            FontId::monospace(10.0 * layout.char_width / 8.0),
            colors.label,
        );
    }
}

fn draw_contact(
    painter: &egui::Painter,
    rect: Rect,
    cell: &LadderCell,
    text: &str,
    colors: LadderColors,
    char_width: f32,
) {
    painter.rect(
        rect,
        4.0,
        colors.contact_fill,
        Stroke::new(1.0, colors.stroke),
    );
    let left = rect.left() + 15.0;
    let right = rect.right() - 15.0;
    painter.line_segment(
        [
            Pos2::new(left, rect.top() + 5.0),
            Pos2::new(left, rect.bottom() - 5.0),
        ],
        Stroke::new(1.5, colors.stroke),
    );
    painter.line_segment(
        [
            Pos2::new(right, rect.top() + 5.0),
            Pos2::new(right, rect.bottom() - 5.0),
        ],
        Stroke::new(1.5, colors.stroke),
    );
    if cell.contact == Some(LadderContact::NormallyClosed) {
        painter.line_segment(
            [
                Pos2::new(left - 3.0, rect.bottom() - 5.0),
                Pos2::new(right + 3.0, rect.top() + 5.0),
            ],
            Stroke::new(1.2, colors.stroke),
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(13.0 * char_width / 8.0),
        colors.text,
    );
}

fn draw_coil(
    painter: &egui::Painter,
    rect: Rect,
    text: &str,
    colors: LadderColors,
    char_width: f32,
) {
    painter.rect(
        rect,
        4.0,
        colors.block_fill,
        Stroke::new(1.0, colors.stroke),
    );
    painter.circle_stroke(
        Pos2::new(rect.left() + 17.0, rect.center().y),
        9.0,
        Stroke::new(1.4, colors.stroke),
    );
    painter.circle_stroke(
        Pos2::new(rect.right() - 17.0, rect.center().y),
        9.0,
        Stroke::new(1.4, colors.stroke),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(13.0 * char_width / 8.0),
        colors.text,
    );
}

fn cell_width(cell: &LadderCell, char_width: f32) -> f32 {
    (cell_text(cell).chars().count() as f32 * char_width + 28.0).max(74.0)
}

fn max_cell_width(program: &LadderProgramData, char_width: f32) -> f32 {
    program
        .structure
        .rungs
        .iter()
        .flat_map(|rung| &rung.cells)
        .map(|cell| cell_width(cell, char_width))
        .fold(74.0, f32::max)
}

fn cell_text(cell: &LadderCell) -> String {
    let operand_text = if cell.operands.is_empty() {
        String::new()
    } else {
        format!(" {}", cell.operands.join(" "))
    };

    match cell.kind {
        LadderElementKind::DeviceRef | LadderElementKind::InternalRef => match cell.coil {
            Some(LadderCoil::Output) => cell.value.clone(),
            Some(LadderCoil::Inverse) => format!("/ {}", cell.value),
            Some(LadderCoil::Set) => format!("S {}", cell.value),
            Some(LadderCoil::Reset) => format!("R {}", cell.value),
            Some(LadderCoil::RisingPulse) => format!("P {}", cell.value),
            Some(LadderCoil::FallingPulse) => format!("N {}", cell.value),
            None => match cell.contact {
                Some(LadderContact::NormallyClosed) => format!("/{}", cell.value),
                Some(LadderContact::Inverse) => format!("* {}", cell.value),
                Some(LadderContact::RisingPulse) => format!("^^ {}", cell.value),
                Some(LadderContact::FallingPulse) => format!("vv {}", cell.value),
                Some(LadderContact::AddressedRisingPulse) => format!("P {}", cell.value),
                Some(LadderContact::AddressedRisingPulseNot) => format!("P/ {}", cell.value),
                Some(LadderContact::AddressedFallingPulse) => format!("N {}", cell.value),
                Some(LadderContact::AddressedFallingPulseNot) => format!("N/ {}", cell.value),
                _ => cell.value.clone(),
            },
        },
        LadderElementKind::Operation
            if matches!(
                cell.value.as_str(),
                "OUT" | "OUTP" | "SET" | "RST" | "RESET" | "FF"
            ) =>
        {
            format!("{}{}", cell.value, operand_text)
        }
        LadderElementKind::Comparison
        | LadderElementKind::Timer
        | LadderElementKind::Logic
        | LadderElementKind::Operation
        | LadderElementKind::InstructionCall => format!("{}{}", cell.value, operand_text),
        LadderElementKind::Constant | LadderElementKind::Comment => cell.value.clone(),
    }
}
