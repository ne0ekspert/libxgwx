use crate::{
    LadderCell, LadderCoil, LadderContact, LadderElementKind, LadderHorizontalLine,
    LadderProgramData, LadderStructure, LadderVerticalLine, VariableSummary,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

/// Instruction-list representation of one decoded ladder program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlProgram {
    pub program_name: Option<String>,
    /// Instructions and rung comments in execution order.
    pub steps: Vec<IlStep>,
}

impl IlProgram {
    /// Return a copy whose operands use unique, exact variable-name matches.
    ///
    /// Operands without a matching variable, and addresses or names that occur
    /// in duplicate symbol records, remain unchanged.
    pub fn with_variable_names(&self, variables: &[VariableSummary]) -> Self {
        let aliases = unique_variable_aliases(variables);
        let steps = self
            .steps
            .iter()
            .map(|step| match step {
                IlStep::Comment(text) => IlStep::Comment(text.clone()),
                IlStep::Instruction { mnemonic, operands } => IlStep::Instruction {
                    mnemonic: mnemonic.clone(),
                    operands: operands
                        .iter()
                        .map(|operand| {
                            aliases
                                .get(operand.as_str())
                                .copied()
                                .unwrap_or(operand)
                                .to_owned()
                        })
                        .collect(),
                },
            })
            .collect();

        Self {
            program_name: self.program_name.clone(),
            steps,
        }
    }
}

impl fmt::Display for IlProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, step) in self.steps.iter().enumerate() {
            if index != 0 {
                writeln!(f)?;
            }
            write!(f, "{step}")?;
        }
        Ok(())
    }
}

/// One line in an instruction-list program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IlStep {
    Comment(String),
    Instruction {
        mnemonic: String,
        operands: Vec<String>,
    },
}

impl fmt::Display for IlStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comment(text) => write!(f, "Comment: {}", single_line_comment(text)),
            Self::Instruction { mnemonic, operands } if operands.is_empty() => {
                write!(f, "{mnemonic}")
            }
            Self::Instruction { mnemonic, operands } => {
                write!(f, "{mnemonic} {}", operands.join(" "))
            }
        }
    }
}

fn single_line_comment(text: &str) -> String {
    text.replace("\r\n", "\\n")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

/// Failure to convert partially decoded LD structure into unambiguous IL.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LdToIlError {
    UnknownRecord {
        offset: usize,
        marker: [u8; 2],
        raw_x: u8,
        raw_y: u8,
    },
    UnsupportedCell {
        offset: usize,
        raw_x: u8,
        raw_y: u8,
        value: String,
    },
    DisconnectedCell {
        offset: usize,
        raw_x: u8,
        raw_y: u8,
        value: String,
    },
    BranchWithoutInput {
        raw_x: u8,
        raw_y_start: u8,
        raw_y_end: u8,
    },
    BranchWithoutOutput {
        raw_x: u8,
        raw_y_start: u8,
        raw_y_end: u8,
    },
    StatefulBranchFanout {
        raw_x: u8,
        raw_y_start: u8,
        raw_y_end: u8,
    },
    AmbiguousActionOrder {
        first_raw_x: u8,
        first_raw_y: u8,
        second_raw_x: u8,
        second_raw_y: u8,
    },
    NetworkWithoutOutput {
        raw_y_start: u8,
        raw_y_end: u8,
    },
    CoordinateOverflow {
        offset: usize,
        raw_x: u8,
        raw_y: u8,
    },
}

impl fmt::Display for LdToIlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRecord {
                offset,
                marker,
                raw_x,
                raw_y,
            } => write!(
                f,
                "unknown ladder record 0x{:02x}{:02x} at byte 0x{offset:x} ({raw_x},{raw_y})",
                marker[0], marker[1]
            ),
            Self::UnsupportedCell {
                offset,
                raw_x,
                raw_y,
                value,
            } => write!(
                f,
                "unsupported ladder cell {value:?} at byte 0x{offset:x} ({raw_x},{raw_y})"
            ),
            Self::DisconnectedCell {
                offset,
                raw_x,
                raw_y,
                value,
            } => write!(
                f,
                "disconnected ladder cell {value:?} at byte 0x{offset:x} ({raw_x},{raw_y})"
            ),
            Self::BranchWithoutInput {
                raw_x,
                raw_y_start,
                raw_y_end,
            } => write!(
                f,
                "ladder branch at x={raw_x}, y={raw_y_start}..{raw_y_end} has no input"
            ),
            Self::BranchWithoutOutput {
                raw_x,
                raw_y_start,
                raw_y_end,
            } => write!(
                f,
                "ladder branch at x={raw_x}, y={raw_y_start}..{raw_y_end} has no output"
            ),
            Self::StatefulBranchFanout {
                raw_x,
                raw_y_start,
                raw_y_end,
            } => write!(
                f,
                "ladder branch at x={raw_x}, y={raw_y_start}..{raw_y_end} fans out stateful edge logic"
            ),
            Self::AmbiguousActionOrder {
                first_raw_x,
                first_raw_y,
                second_raw_x,
                second_raw_y,
            } => write!(
                f,
                "parallel ladder actions at ({first_raw_x},{first_raw_y}) and ({second_raw_x},{second_raw_y}) have ambiguous execution order"
            ),
            Self::NetworkWithoutOutput {
                raw_y_start,
                raw_y_end,
            } => write!(
                f,
                "ladder network at y={raw_y_start}..{raw_y_end} has no output"
            ),
            Self::CoordinateOverflow {
                offset,
                raw_x,
                raw_y,
            } => write!(
                f,
                "ladder cell coordinate overflow at byte 0x{offset:x} ({raw_x},{raw_y})"
            ),
        }
    }
}

impl std::error::Error for LdToIlError {}

impl LadderProgramData {
    /// Convert the decoded LD structure to typed instruction-list steps.
    ///
    /// Literal device addresses are retained. The conversion fails closed when
    /// positioned records or connectivity cannot be represented unambiguously.
    pub fn to_il(&self) -> Result<IlProgram, LdToIlError> {
        convert_program(self)
    }

    /// Convert LD to IL and replace exact, uniquely matched addresses with
    /// variable names.
    pub fn to_il_with_variable_names(
        &self,
        variables: &[VariableSummary],
    ) -> Result<IlProgram, LdToIlError> {
        self.to_il()
            .map(|program| program.with_variable_names(variables))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    Contact {
        contact: LadderContact,
        operand: String,
    },
    Unary {
        operation: UnaryOperation,
        input: Box<Expr>,
    },
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOperation {
    Not,
    RisingEdge,
    FallingEdge,
}

#[derive(Debug, Clone)]
struct RowState {
    boundary_x: u8,
    expression: Expr,
}

#[derive(Debug, Clone)]
struct Action<'a> {
    expression: Option<Expr>,
    cell: &'a LadderCell,
}

fn convert_program(program: &LadderProgramData) -> Result<IlProgram, LdToIlError> {
    if let Some(record) = program.structure.unknown_records.first() {
        return Err(LdToIlError::UnknownRecord {
            offset: record.offset,
            marker: record.marker,
            raw_x: record.raw_x,
            raw_y: record.raw_y,
        });
    }

    let networks = logical_networks(&program.structure);
    let mut comments = program.structure.rung_comments.iter().collect::<Vec<_>>();
    comments.sort_by_key(|comment| (comment.raw_y, comment.raw_x, comment.offset));
    let mut next_comment = 0;
    let mut steps = Vec::new();

    for rows in networks {
        let raw_y_start = rows.first().copied().unwrap_or_default();
        while comments
            .get(next_comment)
            .is_some_and(|comment| comment.raw_y <= raw_y_start)
        {
            steps.push(IlStep::Comment(comments[next_comment].text.clone()));
            next_comment += 1;
        }

        let actions = convert_network(&program.structure, &rows)?;
        emit_actions(&actions, &mut steps);
    }

    for comment in &comments[next_comment..] {
        steps.push(IlStep::Comment(comment.text.clone()));
    }

    Ok(IlProgram {
        program_name: program.program_name.clone(),
        steps,
    })
}

fn logical_networks(structure: &LadderStructure) -> Vec<Vec<u8>> {
    let mut rows = structure
        .rungs
        .iter()
        .filter(|rung| !rung.cells.is_empty())
        .map(|rung| rung.raw_y)
        .collect::<Vec<_>>();
    rows.sort_unstable();
    rows.dedup();

    let mut parents = (0..rows.len()).collect::<Vec<_>>();
    for line in &structure.vertical_lines {
        let connected = rows
            .iter()
            .enumerate()
            .filter_map(|(index, raw_y)| {
                (*raw_y >= line.raw_y_start && *raw_y <= line.raw_y_end).then_some(index)
            })
            .collect::<Vec<_>>();
        if let Some((&first, rest)) = connected.split_first() {
            for &index in rest {
                union(&mut parents, first, index);
            }
        }
    }

    let mut grouped = BTreeMap::<usize, Vec<u8>>::new();
    for (index, raw_y) in rows.into_iter().enumerate() {
        let root = find(&mut parents, index);
        grouped.entry(root).or_default().push(raw_y);
    }
    let mut networks = grouped.into_values().collect::<Vec<_>>();
    networks.sort_by_key(|rows| rows.first().copied());
    networks
}

fn find(parents: &mut [usize], index: usize) -> usize {
    if parents[index] != index {
        parents[index] = find(parents, parents[index]);
    }
    parents[index]
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left = find(parents, left);
    let right = find(parents, right);
    if left != right {
        parents[right] = left;
    }
}

fn convert_network<'a>(
    structure: &'a LadderStructure,
    rows: &[u8],
) -> Result<Vec<Action<'a>>, LdToIlError> {
    let cells = structure
        .rungs
        .iter()
        .filter(|rung| rows.binary_search(&rung.raw_y).is_ok())
        .flat_map(|rung| rung.cells.iter())
        .collect::<Vec<_>>();
    let branches = structure
        .vertical_lines
        .iter()
        .filter(|line| rows.iter().any(|raw_y| line_contains_y(line, *raw_y)))
        .collect::<Vec<_>>();
    let mut event_xs = cells.iter().map(|cell| cell.raw_x).collect::<BTreeSet<_>>();
    event_xs.extend(branches.iter().map(|line| line.raw_x));

    let mut states = HashMap::<u8, RowState>::new();
    let mut actions = Vec::new();

    for raw_x in event_xs {
        let mut conditions = cells
            .iter()
            .copied()
            .filter(|cell| cell.raw_x == raw_x && is_condition(cell))
            .collect::<Vec<_>>();
        conditions.sort_by_key(|cell| (cell.raw_y, cell.offset));
        reject_duplicate_cells(&conditions)?;
        for cell in conditions {
            apply_condition(structure, &mut states, cell)?;
        }

        let mut verticals = branches
            .iter()
            .copied()
            .filter(|line| line.raw_x == raw_x)
            .collect::<Vec<_>>();
        verticals.sort_by_key(|line| (line.raw_y_start, line.raw_y_end));
        for line in verticals {
            apply_branch(structure, rows, &cells, &branches, &mut states, line)?;
        }

        let mut terminals = cells
            .iter()
            .copied()
            .filter(|cell| cell.raw_x == raw_x && !is_condition(cell))
            .collect::<Vec<_>>();
        terminals.sort_by_key(|cell| (cell.raw_y, cell.offset));
        reject_duplicate_cells(&terminals)?;
        for cell in terminals {
            if !is_action(cell) || cell.value.is_empty() {
                return Err(unsupported_cell(cell));
            }
            let expression = match states.get(&cell.raw_y) {
                Some(state)
                    if row_connects_between(
                        structure,
                        cell.raw_y,
                        state.boundary_x,
                        cell.raw_x,
                    ) =>
                {
                    Some(state.expression.clone())
                }
                None if is_unconditional_action(cell)
                    && row_connects_from_left(structure, cell.raw_y, cell.raw_x) =>
                {
                    None
                }
                _ => return Err(disconnected_cell(cell)),
            };
            actions.push(Action { expression, cell });
            states.remove(&cell.raw_y);
        }
    }

    if actions.is_empty() {
        return Err(LdToIlError::NetworkWithoutOutput {
            raw_y_start: rows.first().copied().unwrap_or_default(),
            raw_y_end: rows.last().copied().unwrap_or_default(),
        });
    }
    if let Some(pair) = actions
        .windows(2)
        .find(|pair| pair[0].cell.raw_x != pair[1].cell.raw_x)
    {
        return Err(LdToIlError::AmbiguousActionOrder {
            first_raw_x: pair[0].cell.raw_x,
            first_raw_y: pair[0].cell.raw_y,
            second_raw_x: pair[1].cell.raw_x,
            second_raw_y: pair[1].cell.raw_y,
        });
    }

    Ok(actions)
}

fn reject_duplicate_cells(cells: &[&LadderCell]) -> Result<(), LdToIlError> {
    for pair in cells.windows(2) {
        if (pair[0].raw_x, pair[0].raw_y) == (pair[1].raw_x, pair[1].raw_y) {
            return Err(unsupported_cell(pair[1]));
        }
    }
    Ok(())
}

fn apply_condition(
    structure: &LadderStructure,
    states: &mut HashMap<u8, RowState>,
    cell: &LadderCell,
) -> Result<(), LdToIlError> {
    let boundary_x = cell
        .raw_x
        .checked_add(2)
        .ok_or_else(|| coordinate_overflow(cell))?;
    let contact = cell.contact.ok_or_else(|| unsupported_cell(cell))?;

    if let Some(operation) = unary_operation(cell) {
        let Some(state) = states.get_mut(&cell.raw_y) else {
            return Err(disconnected_cell(cell));
        };
        if !row_connects_between(structure, cell.raw_y, state.boundary_x, cell.raw_x) {
            return Err(disconnected_cell(cell));
        }
        state.expression = Expr::Unary {
            operation,
            input: Box::new(state.expression.clone()),
        };
        state.boundary_x = boundary_x;
        return Ok(());
    }

    if cell.value.is_empty()
        || matches!(
            contact,
            LadderContact::Inverse | LadderContact::RisingPulse | LadderContact::FallingPulse
        )
    {
        return Err(unsupported_cell(cell));
    }
    let operand = Expr::Contact {
        contact,
        operand: cell.value.clone(),
    };

    match states.get_mut(&cell.raw_y) {
        Some(state)
            if row_connects_between(structure, cell.raw_y, state.boundary_x, cell.raw_x) =>
        {
            state.expression = and_expression(state.expression.clone(), operand);
            state.boundary_x = boundary_x;
        }
        Some(_) if cell.raw_x == 1 => {
            states.insert(
                cell.raw_y,
                RowState {
                    boundary_x,
                    expression: operand,
                },
            );
        }
        None if cell.raw_x == 1 => {
            states.insert(
                cell.raw_y,
                RowState {
                    boundary_x,
                    expression: operand,
                },
            );
        }
        _ => return Err(disconnected_cell(cell)),
    }

    Ok(())
}

fn apply_branch(
    structure: &LadderStructure,
    network_rows: &[u8],
    cells: &[&LadderCell],
    branches: &[&LadderVerticalLine],
    states: &mut HashMap<u8, RowState>,
    line: &LadderVerticalLine,
) -> Result<(), LdToIlError> {
    let branch_rows = network_rows
        .iter()
        .copied()
        .filter(|raw_y| line_contains_y(line, *raw_y))
        .collect::<Vec<_>>();
    let mut inputs = Vec::new();
    for raw_y in &branch_rows {
        if let Some(state) = states.get(raw_y)
            && row_connects_between(structure, *raw_y, state.boundary_x, line.raw_x)
        {
            inputs.push(state.expression.clone());
        }
    }
    if inputs.is_empty() {
        return Err(LdToIlError::BranchWithoutInput {
            raw_x: line.raw_x,
            raw_y_start: line.raw_y_start,
            raw_y_end: line.raw_y_end,
        });
    }
    let expression = or_expressions(inputs);

    for raw_y in &branch_rows {
        states.remove(raw_y);
    }
    let outputs = branch_rows
        .into_iter()
        .filter(|raw_y| row_has_right_target(structure, *raw_y, line.raw_x, cells, branches))
        .collect::<Vec<_>>();
    if outputs.is_empty() {
        return Err(LdToIlError::BranchWithoutOutput {
            raw_x: line.raw_x,
            raw_y_start: line.raw_y_start,
            raw_y_end: line.raw_y_end,
        });
    }
    if outputs.len() > 1 && expression_is_stateful(&expression) {
        return Err(LdToIlError::StatefulBranchFanout {
            raw_x: line.raw_x,
            raw_y_start: line.raw_y_start,
            raw_y_end: line.raw_y_end,
        });
    }
    for raw_y in outputs {
        states.insert(
            raw_y,
            RowState {
                boundary_x: line.raw_x,
                expression: expression.clone(),
            },
        );
    }

    Ok(())
}

fn row_has_right_target(
    structure: &LadderStructure,
    raw_y: u8,
    raw_x: u8,
    cells: &[&LadderCell],
    branches: &[&LadderVerticalLine],
) -> bool {
    cells.iter().any(|cell| {
        cell.raw_y == raw_y
            && cell.raw_x >= raw_x
            && row_connects_between(structure, raw_y, raw_x, cell.raw_x)
    }) || branches.iter().any(|line| {
        line.raw_x > raw_x
            && line_contains_y(line, raw_y)
            && row_connects_between(structure, raw_y, raw_x, line.raw_x)
    })
}

fn row_connects_between(structure: &LadderStructure, raw_y: u8, from_x: u8, to_x: u8) -> bool {
    if to_x < from_x {
        return false;
    }
    if to_x <= from_x.saturating_add(1) {
        return true;
    }

    structure
        .horizontal_lines
        .iter()
        .any(|line| line.raw_y == raw_y && horizontal_connects(line, from_x, to_x))
}

fn row_connects_from_left(structure: &LadderStructure, raw_y: u8, to_x: u8) -> bool {
    to_x <= 1
        || structure.horizontal_lines.iter().any(|line| {
            let start = line.raw_x_start.min(line.raw_x_end);
            line.raw_y == raw_y && start <= 1 && horizontal_connects(line, 0, to_x)
        })
}

fn horizontal_connects(line: &LadderHorizontalLine, from_x: u8, to_x: u8) -> bool {
    let start = line.raw_x_start.min(line.raw_x_end);
    let end = line.raw_x_start.max(line.raw_x_end);
    start <= from_x.saturating_add(1) && end.saturating_add(1) >= to_x
}

fn line_contains_y(line: &LadderVerticalLine, raw_y: u8) -> bool {
    raw_y >= line.raw_y_start && raw_y <= line.raw_y_end
}

fn is_condition(cell: &LadderCell) -> bool {
    cell.contact.is_some()
}

fn is_action(cell: &LadderCell) -> bool {
    cell.coil.is_some()
        || cell.kind == LadderElementKind::InstructionCall
        || is_unconditional_action(cell)
}

fn is_unconditional_action(cell: &LadderCell) -> bool {
    cell.kind == LadderElementKind::Operation && cell.value == "END" && cell.operands.is_empty()
}

fn unary_operation(cell: &LadderCell) -> Option<UnaryOperation> {
    if !cell.value.is_empty() {
        return None;
    }
    match cell.contact? {
        LadderContact::Inverse => Some(UnaryOperation::Not),
        LadderContact::RisingPulse => Some(UnaryOperation::RisingEdge),
        LadderContact::FallingPulse => Some(UnaryOperation::FallingEdge),
        _ => None,
    }
}

fn and_expression(left: Expr, right: Expr) -> Expr {
    match left {
        Expr::And(mut items) => {
            items.push(right);
            Expr::And(items)
        }
        left => Expr::And(vec![left, right]),
    }
}

fn or_expressions(expressions: Vec<Expr>) -> Expr {
    let mut flattened = Vec::new();
    for expression in expressions {
        match expression {
            Expr::Or(items) => flattened.extend(items),
            expression => flattened.push(expression),
        }
    }
    // A wire can split and rejoin without introducing a new condition.
    // Preserve stateful evaluations, but collapse identical boolean inputs.
    let mut unique = Vec::new();
    for expression in flattened {
        if expression_is_stateful(&expression) || !unique.contains(&expression) {
            unique.push(expression);
        }
    }
    let mut flattened = unique;
    if flattened.len() == 1 {
        flattened.pop().expect("one expression remains")
    } else {
        Expr::Or(flattened)
    }
}

fn expression_is_stateful(expression: &Expr) -> bool {
    match expression {
        Expr::Contact { contact, .. } => matches!(
            contact,
            LadderContact::AddressedRisingPulse
                | LadderContact::AddressedRisingPulseNot
                | LadderContact::AddressedFallingPulse
                | LadderContact::AddressedFallingPulseNot
        ),
        Expr::Unary {
            operation: UnaryOperation::RisingEdge | UnaryOperation::FallingEdge,
            ..
        } => true,
        Expr::Unary { input, .. } => expression_is_stateful(input),
        Expr::And(expressions) | Expr::Or(expressions) => {
            expressions.iter().any(expression_is_stateful)
        }
    }
}

fn emit_actions(actions: &[Action<'_>], steps: &mut Vec<IlStep>) {
    let mut accumulator = None::<&Expr>;
    for action in actions {
        if let Some(expression) = action.expression.as_ref()
            && accumulator != Some(expression)
        {
            emit_expression(expression, Combine::Load, steps);
        }
        emit_action(action.cell, steps);
        accumulator = if action.cell.coil.is_some() {
            action.expression.as_ref()
        } else {
            None
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Combine {
    Load,
    And,
    Or,
}

fn emit_expression(expression: &Expr, combine: Combine, steps: &mut Vec<IlStep>) {
    match expression {
        Expr::Contact { contact, operand } => {
            emit_instruction(
                contact_mnemonic(*contact, combine),
                [operand.clone()],
                steps,
            );
        }
        Expr::Unary { operation, input } if combine == Combine::Load => {
            emit_expression(input, Combine::Load, steps);
            emit_instruction(unary_mnemonic(*operation), Vec::<String>::new(), steps);
        }
        Expr::And(expressions) if combine == Combine::Load => {
            emit_expression(&expressions[0], Combine::Load, steps);
            for expression in &expressions[1..] {
                emit_expression(expression, Combine::And, steps);
            }
        }
        Expr::Or(expressions) if combine == Combine::Load => {
            emit_expression(&expressions[0], Combine::Load, steps);
            for expression in &expressions[1..] {
                emit_expression(expression, Combine::Or, steps);
            }
        }
        expression => {
            emit_expression(expression, Combine::Load, steps);
            emit_instruction(
                match combine {
                    Combine::And => "AND LOAD",
                    Combine::Or => "OR LOAD",
                    Combine::Load => unreachable!("load expressions are handled above"),
                },
                Vec::<String>::new(),
                steps,
            );
        }
    }
}

fn contact_mnemonic(contact: LadderContact, combine: Combine) -> &'static str {
    match (contact, combine) {
        (LadderContact::NormallyOpen, Combine::Load) => "LOAD",
        (LadderContact::NormallyOpen, Combine::And) => "AND",
        (LadderContact::NormallyOpen, Combine::Or) => "OR",
        (LadderContact::NormallyClosed, Combine::Load) => "LOAD NOT",
        (LadderContact::NormallyClosed, Combine::And) => "AND NOT",
        (LadderContact::NormallyClosed, Combine::Or) => "OR NOT",
        (LadderContact::AddressedRisingPulse, Combine::Load) => "LOADP",
        (LadderContact::AddressedRisingPulse, Combine::And) => "ANDP",
        (LadderContact::AddressedRisingPulse, Combine::Or) => "ORP",
        (LadderContact::AddressedRisingPulseNot, Combine::Load) => "LOADP NOT",
        (LadderContact::AddressedRisingPulseNot, Combine::And) => "ANDP NOT",
        (LadderContact::AddressedRisingPulseNot, Combine::Or) => "ORP NOT",
        (LadderContact::AddressedFallingPulse, Combine::Load) => "LOADN",
        (LadderContact::AddressedFallingPulse, Combine::And) => "ANDN",
        (LadderContact::AddressedFallingPulse, Combine::Or) => "ORN",
        (LadderContact::AddressedFallingPulseNot, Combine::Load) => "LOADN NOT",
        (LadderContact::AddressedFallingPulseNot, Combine::And) => "ANDN NOT",
        (LadderContact::AddressedFallingPulseNot, Combine::Or) => "ORN NOT",
        (LadderContact::Inverse | LadderContact::RisingPulse | LadderContact::FallingPulse, _) => {
            unreachable!("operand-free contacts are emitted as unary operations")
        }
    }
}

fn unary_mnemonic(operation: UnaryOperation) -> &'static str {
    match operation {
        UnaryOperation::Not => "NOT",
        UnaryOperation::RisingEdge => "R_EDGE",
        UnaryOperation::FallingEdge => "F_EDGE",
    }
}

fn emit_action(cell: &LadderCell, steps: &mut Vec<IlStep>) {
    if let Some(coil) = cell.coil {
        emit_instruction(coil_mnemonic(coil), [cell.value.clone()], steps);
    } else {
        emit_instruction(cell.value.clone(), cell.operands.clone(), steps);
    }
}

fn coil_mnemonic(coil: LadderCoil) -> &'static str {
    match coil {
        LadderCoil::Output => "OUT",
        LadderCoil::Inverse => "OUT NOT",
        LadderCoil::Set => "SET",
        LadderCoil::Reset => "RST",
        LadderCoil::RisingPulse => "OUTP",
        LadderCoil::FallingPulse => "OUTN",
    }
}

fn emit_instruction(
    mnemonic: impl Into<String>,
    operands: impl IntoIterator<Item = String>,
    steps: &mut Vec<IlStep>,
) {
    steps.push(IlStep::Instruction {
        mnemonic: mnemonic.into(),
        operands: operands.into_iter().collect(),
    });
}

fn unique_variable_aliases(variables: &[VariableSummary]) -> HashMap<&str, &str> {
    let mut address_counts = HashMap::<&str, usize>::new();
    let mut name_counts = HashMap::<&str, usize>::new();
    for variable in variables {
        if let Some(address) = variable
            .address
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            *address_counts.entry(address).or_default() += 1;
        }
        if let Some(name) = variable.name.as_deref().filter(|value| !value.is_empty()) {
            *name_counts.entry(name).or_default() += 1;
        }
    }

    variables
        .iter()
        .filter_map(|variable| {
            let address = variable.address.as_deref()?;
            let name = variable.name.as_deref()?;
            (address_counts.get(address) == Some(&1) && name_counts.get(name) == Some(&1))
                .then_some((address, name))
        })
        .collect()
}

fn unsupported_cell(cell: &LadderCell) -> LdToIlError {
    LdToIlError::UnsupportedCell {
        offset: cell.offset,
        raw_x: cell.raw_x,
        raw_y: cell.raw_y,
        value: cell.value.clone(),
    }
}

fn disconnected_cell(cell: &LadderCell) -> LdToIlError {
    LdToIlError::DisconnectedCell {
        offset: cell.offset,
        raw_x: cell.raw_x,
        raw_y: cell.raw_y,
        value: cell.value.clone(),
    }
}

fn coordinate_overflow(cell: &LadderCell) -> LdToIlError {
    LdToIlError::CoordinateOverflow {
        offset: cell.offset,
        raw_x: cell.raw_x,
        raw_y: cell.raw_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LadderBranchGroup, LadderRung};

    #[test]
    fn emits_union_instructions_for_nested_expressions() {
        let a = contact("A");
        let b = contact("B");
        let c = contact("C");
        let expression = Expr::And(vec![a, Expr::Or(vec![b, c])]);
        let mut steps = Vec::new();

        emit_expression(&expression, Combine::Load, &mut steps);

        assert_eq!(
            steps.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["LOAD A", "LOAD B", "OR C", "AND LOAD"]
        );
    }

    #[test]
    fn renders_multiline_comments_as_one_step() {
        assert_eq!(
            IlStep::Comment("first\nsecond\r\nthird\rfourth".to_owned()).to_string(),
            "Comment: first\\nsecond\\nthird\\rfourth"
        );
    }

    #[test]
    fn rejects_stateful_edge_fanout() {
        let program = test_program(
            vec![
                LadderRung {
                    raw_y: 4,
                    cells: vec![
                        test_cell(
                            1,
                            4,
                            "A",
                            Some(LadderContact::NormallyOpen),
                            None,
                            LadderElementKind::DeviceRef,
                        ),
                        test_cell(
                            4,
                            4,
                            "",
                            Some(LadderContact::RisingPulse),
                            None,
                            LadderElementKind::DeviceRef,
                        ),
                        test_cell(
                            10,
                            4,
                            "Q1",
                            None,
                            Some(LadderCoil::Output),
                            LadderElementKind::DeviceRef,
                        ),
                    ],
                },
                LadderRung {
                    raw_y: 8,
                    cells: vec![test_cell(
                        10,
                        8,
                        "Q2",
                        None,
                        Some(LadderCoil::Output),
                        LadderElementKind::DeviceRef,
                    )],
                },
            ],
            vec![vertical(6, 4, 8)],
            vec![
                horizontal(4, 1, 6),
                horizontal(4, 7, 10),
                horizontal(8, 6, 10),
            ],
        );

        assert_eq!(
            program.to_il(),
            Err(LdToIlError::StatefulBranchFanout {
                raw_x: 6,
                raw_y_start: 4,
                raw_y_end: 8,
            })
        );
    }

    #[test]
    fn rejects_instruction_eno_chains() {
        let program = test_program(
            vec![LadderRung {
                raw_y: 4,
                cells: vec![
                    test_cell(
                        1,
                        4,
                        "A",
                        Some(LadderContact::NormallyOpen),
                        None,
                        LadderElementKind::DeviceRef,
                    ),
                    LadderCell {
                        operands: vec!["0".to_owned(), "D0".to_owned()],
                        ..test_cell(4, 4, "MOV", None, None, LadderElementKind::InstructionCall)
                    },
                    test_cell(
                        10,
                        4,
                        "Q",
                        None,
                        Some(LadderCoil::Output),
                        LadderElementKind::DeviceRef,
                    ),
                ],
            }],
            Vec::new(),
            vec![horizontal(4, 1, 4), horizontal(4, 7, 10)],
        );

        assert!(matches!(
            program.to_il(),
            Err(LdToIlError::DisconnectedCell {
                raw_x: 10,
                raw_y: 4,
                ..
            })
        ));
    }

    #[test]
    fn rejects_unconditional_application_instructions() {
        let program = test_program(
            vec![LadderRung {
                raw_y: 4,
                cells: vec![LadderCell {
                    operands: vec!["0".to_owned(), "D0".to_owned()],
                    ..test_cell(10, 4, "MOV", None, None, LadderElementKind::InstructionCall)
                }],
            }],
            Vec::new(),
            vec![horizontal(4, 1, 10)],
        );

        assert!(matches!(
            program.to_il(),
            Err(LdToIlError::DisconnectedCell {
                raw_x: 10,
                raw_y: 4,
                ..
            })
        ));
    }

    #[test]
    fn rejects_unmodeled_comparison_cells() {
        let program = test_program(
            vec![LadderRung {
                raw_y: 4,
                cells: vec![
                    test_cell(
                        1,
                        4,
                        "A",
                        Some(LadderContact::NormallyOpen),
                        None,
                        LadderElementKind::DeviceRef,
                    ),
                    LadderCell {
                        operands: vec!["D0".to_owned(), "3".to_owned()],
                        ..test_cell(4, 4, "<=", None, None, LadderElementKind::Comparison)
                    },
                ],
            }],
            Vec::new(),
            vec![horizontal(4, 1, 4)],
        );

        assert!(matches!(
            program.to_il(),
            Err(LdToIlError::UnsupportedCell {
                raw_x: 4,
                raw_y: 4,
                ..
            })
        ));
    }

    #[test]
    fn rejects_parallel_actions_with_ambiguous_order() {
        let program = test_program(
            vec![
                LadderRung {
                    raw_y: 4,
                    cells: vec![
                        test_cell(
                            1,
                            4,
                            "A",
                            Some(LadderContact::NormallyOpen),
                            None,
                            LadderElementKind::DeviceRef,
                        ),
                        test_cell(
                            10,
                            4,
                            "TOP",
                            None,
                            Some(LadderCoil::Output),
                            LadderElementKind::DeviceRef,
                        ),
                    ],
                },
                LadderRung {
                    raw_y: 8,
                    cells: vec![test_cell(
                        7,
                        8,
                        "BOTTOM",
                        None,
                        Some(LadderCoil::Output),
                        LadderElementKind::DeviceRef,
                    )],
                },
            ],
            vec![vertical(3, 4, 8)],
            vec![
                horizontal(4, 1, 3),
                horizontal(4, 4, 10),
                horizontal(8, 3, 7),
            ],
        );

        assert_eq!(
            program.to_il(),
            Err(LdToIlError::AmbiguousActionOrder {
                first_raw_x: 7,
                first_raw_y: 8,
                second_raw_x: 10,
                second_raw_y: 4,
            })
        );
    }

    fn contact(operand: &str) -> Expr {
        Expr::Contact {
            contact: LadderContact::NormallyOpen,
            operand: operand.to_owned(),
        }
    }

    fn test_program(
        rungs: Vec<LadderRung>,
        vertical_lines: Vec<LadderVerticalLine>,
        horizontal_lines: Vec<LadderHorizontalLine>,
    ) -> LadderProgramData {
        LadderProgramData {
            program_name: Some("test".to_owned()),
            version: None,
            project_type: None,
            eno_control_option: None,
            compressed: false,
            encoded_len: 0,
            decoded_len: 0,
            data: Vec::new(),
            strings: Vec::new(),
            elements: Vec::new(),
            structure: LadderStructure {
                rungs,
                vertical_lines: vertical_lines.clone(),
                branch_groups: vertical_lines
                    .iter()
                    .map(|line| LadderBranchGroup {
                        raw_x: line.raw_x,
                        raw_y_start: line.raw_y_start,
                        raw_y_end: line.raw_y_end,
                    })
                    .collect(),
                horizontal_lines,
                rung_comments: Vec::new(),
                output_comments: Vec::new(),
                unknown_records: Vec::new(),
            },
            instructions: Vec::new(),
        }
    }

    fn test_cell(
        raw_x: u8,
        raw_y: u8,
        value: &str,
        contact: Option<LadderContact>,
        coil: Option<LadderCoil>,
        kind: LadderElementKind,
    ) -> LadderCell {
        LadderCell {
            offset: usize::from(raw_y) * 256 + usize::from(raw_x),
            raw_x,
            raw_y,
            kind,
            value: value.to_owned(),
            operands: Vec::new(),
            contact,
            coil,
        }
    }

    fn vertical(raw_x: u8, raw_y_start: u8, raw_y_end: u8) -> LadderVerticalLine {
        LadderVerticalLine {
            raw_x,
            raw_y_start,
            raw_y_end,
        }
    }

    fn horizontal(raw_y: u8, raw_x_start: u8, raw_x_end: u8) -> LadderHorizontalLine {
        LadderHorizontalLine {
            raw_y,
            raw_x_start,
            raw_x_end,
        }
    }
}
