use std::env;
use std::error::Error;
use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use xgwx::{
    CnetConfigInfoSummary, CnetPortConfigSummary, DecodedPayloadSummary, FenetConfigInfoSummary,
    HscChannelSummary, HscParameterSummary, Ipv4Summary, LadderProgramData, NetworkModuleSummary,
    NetworkSummary, ParameterSummary, PidCalLoopSummary, PidCalParameterSummary,
    PidTuneLoopSummary, PidTuneParameterSummary, PositionAxisSummary, PositionParameterSummary,
    ProgramSummary, VariableSummary, XgwxDocument, XgwxError, XmlAttribute, XmlElement,
    XmlSectionSummary,
};

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo run --example tui -- <file.xgwx>",
        )
    })?;
    let doc = XgwxDocument::from_path(&path)?;
    let mut app = App::new(path, doc);

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;

    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Programs,
    Networks,
    Variables,
    Parameters,
    Data,
}

struct App {
    path: String,
    project_name: String,
    file_version: String,
    doc: XgwxDocument,
    ladder_programs: Vec<Result<LadderProgramData, XgwxError>>,
    variables: Result<Vec<VariableSummary>, XgwxError>,
    decoded_payloads: Vec<Result<DecodedPayloadSummary, XgwxError>>,
    view: View,
    program_selected: usize,
    network_selected: usize,
    variable_selected: usize,
    parameter_selected: usize,
    data_selected: usize,
    detail_scroll: u16,
}

impl App {
    fn new(path: String, doc: XgwxDocument) -> Self {
        let project = doc.project_info();
        Self {
            path,
            project_name: project.name.unwrap_or_else(|| "<unnamed>".to_owned()),
            file_version: project
                .file_version
                .unwrap_or_else(|| "<unknown>".to_owned()),
            ladder_programs: doc.ladder_programs(),
            variables: doc.variables(),
            decoded_payloads: doc.decoded_payloads(),
            doc,
            view: View::Programs,
            program_selected: 0,
            network_selected: 0,
            variable_selected: 0,
            parameter_selected: 0,
            data_selected: 0,
            detail_scroll: 0,
        }
    }

    fn programs(&self) -> Vec<ProgramSummary> {
        self.doc.programs()
    }

    fn networks(&self) -> Vec<NetworkSummary> {
        self.doc.networks()
    }

    fn selected_program_element(&self) -> Option<&XmlElement> {
        self.doc
            .root
            .descendants_named("Program")
            .nth(self.program_selected)
    }

    fn selected_ladder_program(&self) -> Option<&Result<LadderProgramData, XgwxError>> {
        self.ladder_programs.get(self.program_selected)
    }

    fn next(&mut self) {
        match self.view {
            View::Programs => {
                self.program_selected = next_index(self.program_selected, self.programs().len())
            }
            View::Networks => {
                self.network_selected = next_index(self.network_selected, self.networks().len())
            }
            View::Variables => {
                let len = self
                    .variables
                    .as_ref()
                    .map(|variables| variables.len())
                    .unwrap_or(0);
                self.variable_selected = next_index(self.variable_selected, len);
            }
            View::Parameters => {
                self.parameter_selected =
                    next_index(self.parameter_selected, self.doc.parameters().len());
            }
            View::Data => {
                self.data_selected = next_index(self.data_selected, self.decoded_payloads.len());
            }
        }
        self.reset_detail_scroll();
    }

    fn previous(&mut self) {
        match self.view {
            View::Programs => {
                self.program_selected =
                    previous_index(self.program_selected, self.programs().len());
            }
            View::Networks => {
                self.network_selected =
                    previous_index(self.network_selected, self.networks().len());
            }
            View::Variables => {
                let len = self
                    .variables
                    .as_ref()
                    .map(|variables| variables.len())
                    .unwrap_or(0);
                self.variable_selected = previous_index(self.variable_selected, len);
            }
            View::Parameters => {
                self.parameter_selected =
                    previous_index(self.parameter_selected, self.doc.parameters().len());
            }
            View::Data => {
                self.data_selected =
                    previous_index(self.data_selected, self.decoded_payloads.len());
            }
        }
        self.reset_detail_scroll();
    }

    fn switch_view(&mut self) {
        self.view = match self.view {
            View::Programs => View::Networks,
            View::Networks => View::Variables,
            View::Variables => View::Parameters,
            View::Parameters => View::Data,
            View::Data => View::Programs,
        };
        self.reset_detail_scroll();
    }

    fn scroll_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(8);
    }

    fn scroll_detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(8);
    }

    fn reset_detail_scroll(&mut self) {
        self.detail_scroll = 0;
    }
}

fn next_index(current: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (current + 1) % len }
}

fn previous_index(current: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        current.checked_sub(1).unwrap_or(len - 1)
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
                .split(vertical[1]);

            render_header(frame, app, vertical[0]);

            match app.view {
                View::Programs => render_programs(frame, app, body[0], body[1]),
                View::Networks => render_networks(frame, app, body[0], body[1]),
                View::Variables => render_variables(frame, app, body[0], body[1]),
                View::Parameters => render_parameters(frame, app, body[0], body[1]),
                View::Data => render_data(frame, app, body[0], body[1]),
            }
        })?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Esc => break,
                KeyCode::Tab => app.switch_view(),
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                KeyCode::PageDown => app.scroll_detail_down(),
                KeyCode::PageUp => app.scroll_detail_up(),
                KeyCode::Home => app.reset_detail_scroll(),
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_header(frame: &mut ratatui::Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let view_label = match app.view {
        View::Programs => "Programs",
        View::Networks => "Networks",
        View::Variables => "Variables",
        View::Parameters => "Parameters",
        View::Data => "Data",
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "XGWX ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&app.project_name),
        Span::raw("  "),
        Span::styled("FileVer ", Style::default().fg(Color::DarkGray)),
        Span::raw(&app.file_version),
        Span::raw("  "),
        Span::styled(view_label, Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" switch  "),
        Span::styled(
            "j/k",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" move  "),
        Span::styled(
            "PgUp/PgDn",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" scroll  "),
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" quit"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.path.as_str()),
    );
    frame.render_widget(title, area);
}

fn render_programs(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let programs = app.programs();
    let items = programs
        .iter()
        .enumerate()
        .map(|(index, program)| {
            let name = program.name.as_deref().unwrap_or("<unnamed>");
            ListItem::new(format!("{:02}. {name}", index + 1))
        })
        .collect::<Vec<_>>();
    let mut list_state =
        ListState::default().with_selected((!items.is_empty()).then_some(app.program_selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Programs ({})", programs.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut list_state);

    let selected = programs.get(app.program_selected);
    let details = program_details(
        selected,
        app.selected_ladder_program(),
        app.selected_program_element(),
    );
    let details = Paragraph::new(details)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Program Structure"),
        )
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(details, detail_area);
}

fn render_networks(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let networks = app.networks();
    let items = networks
        .iter()
        .enumerate()
        .map(|(index, network)| {
            let name = network.name.as_deref().unwrap_or("<unnamed>");
            let module_count = network.modules.len();
            ListItem::new(format!("{:02}. {name} ({module_count} modules)", index + 1))
        })
        .collect::<Vec<_>>();
    let mut list_state =
        ListState::default().with_selected((!items.is_empty()).then_some(app.network_selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Networks ({})", networks.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut list_state);

    let details = network_details(app, networks.get(app.network_selected));
    let details = Paragraph::new(details)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Network Details"),
        )
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(details, detail_area);
}

fn render_variables(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let variables = match &app.variables {
        Ok(variables) => variables,
        Err(error) => {
            let message = Paragraph::new(vec![Line::from(vec![
                Span::styled("Variable decode error: ", Style::default().fg(Color::Red)),
                Span::raw(error.to_string()),
            ])])
            .block(Block::default().borders(Borders::ALL).title("Variables"));
            frame.render_widget(message, list_area);
            return;
        }
    };

    let items = variables
        .iter()
        .enumerate()
        .map(|(index, variable)| {
            let name = variable.name.as_deref().unwrap_or("<unnamed>");
            let data_type = variable.data_type.as_deref().unwrap_or("?");
            ListItem::new(format!("{:04}. {name} : {data_type}", index + 1))
        })
        .collect::<Vec<_>>();
    let mut list_state =
        ListState::default().with_selected((!items.is_empty()).then_some(app.variable_selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Variables ({})", variables.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut list_state);

    let details = variable_details(variables.get(app.variable_selected));
    let details = Paragraph::new(details)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Variable Details"),
        )
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(details, detail_area);
}

fn render_parameters(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let parameters = app.doc.parameters();
    let items = parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let parameter_type = parameter.parameter_type.as_deref().unwrap_or("<unknown>");
            ListItem::new(format!(
                "{:02}. {parameter_type} ({} sections)",
                index + 1,
                parameter.sections.len()
            ))
        })
        .collect::<Vec<_>>();
    let mut list_state =
        ListState::default().with_selected((!items.is_empty()).then_some(app.parameter_selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Parameters ({})", parameters.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut list_state);

    let details = parameter_details(app, parameters.get(app.parameter_selected));
    let details = Paragraph::new(details)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Parameter Details"),
        )
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(details, detail_area);
}

fn render_data(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let items = app
        .decoded_payloads
        .iter()
        .enumerate()
        .map(|(index, payload)| match payload {
            Ok(payload) => ListItem::new(format!(
                "{:02}. {} ({} bytes)",
                index + 1,
                payload.tag,
                payload.decoded_len
            )),
            Err(error) => ListItem::new(format!("{:02}. decode error: {error}", index + 1)),
        })
        .collect::<Vec<_>>();
    let mut list_state =
        ListState::default().with_selected((!items.is_empty()).then_some(app.data_selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Binary Payloads ({})", app.decoded_payloads.len())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut list_state);

    let details = data_details(app);
    let details = Paragraph::new(details)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Parsed Data Sections"),
        )
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(details, detail_area);
}

fn program_details(
    program: Option<&ProgramSummary>,
    ladder: Option<&Result<LadderProgramData, XgwxError>>,
    program_element: Option<&XmlElement>,
) -> Vec<Line<'static>> {
    let Some(program) = program else {
        return vec![Line::from("No programs found.")];
    };

    let mut lines = vec![
        field("Name", program.name.clone()),
        field("Task", program.task.clone()),
        field("Object ID", program.object_id.clone()),
        field("Version", program.version.map(|value| value.to_string())),
        field("Kind", program.kind.map(|value| value.to_string())),
        field(
            "Local Variable",
            program.local_variable.map(|value| value.to_string()),
        ),
        field("Instance", program.instance_name.clone()),
        field("Comment", program.comment.clone()),
        Line::from(""),
        Line::from(Span::styled(
            "Decoded ladder data",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    match ladder {
        Some(Ok(ladder)) => {
            lines.push(field("Decoded bytes", Some(ladder.decoded_len.to_string())));
            lines.push(field("Strings", Some(ladder.strings.len().to_string())));
            lines.push(field("Elements", Some(ladder.elements.len().to_string())));
            lines.push(field(
                "Instructions",
                Some(ladder.instructions.len().to_string()),
            ));
            lines.push(Line::from(""));
            for element in &ladder.elements {
                let operands = if element.operands.is_empty() {
                    String::new()
                } else {
                    format!(" {}", element.operands.join(", "))
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  @0x{:04x} ", element.offset),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{:?}", element.kind),
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        element.value.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(operands),
                ]));
            }
        }
        Some(Err(error)) => {
            lines.push(Line::from(vec![
                Span::styled("Decode error: ", Style::default().fg(Color::Red)),
                Span::raw(error.to_string()),
            ]));
        }
        None => lines.push(Line::from("No ladder payload.")),
    }

    lines.extend([
        Line::from(""),
        Line::from(Span::styled(
            "XML structure",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
    ]);

    if let Some(element) = program_element {
        push_xml_tree(element, 0, &mut lines);
    }

    lines
}

fn parameter_details(app: &App, parameter: Option<&ParameterSummary>) -> Vec<Line<'static>> {
    let Some(parameter) = parameter else {
        return vec![Line::from("No parameters found.")];
    };

    let mut lines = vec![
        field("Type", parameter.parameter_type.clone()),
        field(
            "Attributes",
            Some(summarize_attributes(&parameter.attributes)),
        ),
        field("Sections", Some(parameter.sections.len().to_string())),
    ];

    for section in &parameter.sections {
        lines.push(Line::from(""));
        lines.push(heading(&section.name));
        lines.push(field("Children", Some(section.child_count.to_string())));
        if let Some(text) = section.text.as_deref() {
            lines.push(field("Text", Some(truncate_long(text))));
        }
        if parameter.parameter_type.as_deref() == Some("BASIC PARAMETER") {
            lines.extend(grouped_attribute_lines("  ", &section.attributes));
        } else {
            lines.extend(attribute_lines("  ", &section.attributes));
        }
    }

    if parameter
        .parameter_type
        .as_deref()
        .is_some_and(|parameter_type| parameter_type.contains("FENET"))
        && let Some(safety) = app.doc.safety_comm()
    {
        lines.push(Line::from(""));
        lines.push(heading("Safety Comm"));
        lines.push(field(
            "Rcv Wait",
            safety.rcv_wait_time.map(|value| value.to_string()),
        ));
        lines.push(field(
            "Retrans",
            safety.retrans_time.map(|value| value.to_string()),
        ));
        lines.push(field(
            "Glofa Sockets",
            safety.glofa_socket_count.map(|value| value.to_string()),
        ));
        lines.push(field(
            "Driver Type",
            safety.driver_type.map(|value| value.to_string()),
        ));
        lines.push(field("IP Address", safety.ip_address.clone()));
        lines.push(field(
            "IP Raw",
            safety.ip_address_raw.map(|value| value.to_string()),
        ));
        lines.push(field("Gateway", safety.gateway.clone()));
        lines.push(field(
            "Gateway Raw",
            safety.gateway_raw.map(|value| value.to_string()),
        ));
        lines.push(field("Subnet", safety.subnet.clone()));
        lines.push(field(
            "Subnet Raw",
            safety.subnet_raw.map(|value| value.to_string()),
        ));
        for channel in safety.channels {
            let address = channel.address.unwrap_or_else(|| "<none>".to_owned());
            lines.push(Line::from(format!(
                "  {} address={} data_type={:?} size={:?}",
                channel.name, address, channel.data_type, channel.size
            )));
        }
    }

    if parameter.parameter_type.as_deref() == Some("HSC PARAMETER") {
        lines.push(Line::from(""));
        lines.push(heading("HSC Payload"));
        match app
            .doc
            .hsc_parameters()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(hsc_parameters) => {
                if hsc_parameters.is_empty() {
                    lines.push(Line::from("  <none>"));
                } else {
                    for hsc in hsc_parameters {
                        lines.extend(hsc_parameter_lines(&hsc));
                    }
                }
            }
            Err(error) => {
                lines.push(Line::from(vec![
                    Span::styled("Decode error: ", Style::default().fg(Color::Red)),
                    Span::raw(error.to_string()),
                ]));
            }
        }
    }

    if parameter.parameter_type.as_deref() == Some("POSITION PARAMETER") {
        lines.push(Line::from(""));
        lines.push(heading("Position Parameter"));
        let position_parameters = app.doc.position_parameters();
        if position_parameters.is_empty() {
            lines.push(Line::from("  <none>"));
        } else {
            for position in position_parameters {
                lines.extend(position_parameter_lines(&position));
            }
        }
    }

    if parameter.parameter_type.as_deref() == Some("IO PARAMETER") {
        let cnet_configs = app.doc.cnet_config_infos();
        if !cnet_configs.is_empty() {
            lines.push(Line::from(""));
            lines.push(heading("Cnet Modules"));
            for config in cnet_configs {
                lines.extend(cnet_config_lines(&config));
            }
        }

        let fenet_configs = app.doc.fenet_config_infos();
        if !fenet_configs.is_empty() {
            lines.push(Line::from(""));
            lines.push(heading("FEnet Modules"));
            for config in fenet_configs {
                lines.extend(fenet_config_lines(&config));
            }
        }
    }

    if parameter.parameter_type.as_deref() == Some("PID CAL PARAMETER") {
        lines.push(Line::from(""));
        lines.push(heading("PID Calculation"));
        let pid_parameters = app.doc.pid_cal_parameters();
        if pid_parameters.is_empty() {
            lines.push(Line::from("  <none>"));
        } else {
            for pid in pid_parameters {
                lines.extend(pid_cal_parameter_lines(&pid));
            }
        }
    }

    if parameter.parameter_type.as_deref() == Some("PID TUNE PARAMETER") {
        lines.push(Line::from(""));
        lines.push(heading("PID Tuning"));
        let pid_parameters = app.doc.pid_tune_parameters();
        if pid_parameters.is_empty() {
            lines.push(Line::from("  <none>"));
        } else {
            for pid in pid_parameters {
                lines.extend(pid_tune_parameter_lines(&pid));
            }
        }
    }

    lines
}

fn hsc_parameter_lines(hsc: &HscParameterSummary) -> Vec<Line<'static>> {
    let mut lines = vec![
        field(
            "Payload ASCII",
            hsc.payload_asc_length.map(|value| value.to_string()),
        ),
        field("Payload bytes", Some(hsc.payload_bytes.len().to_string())),
        field(
            "Initial unknown",
            hsc.initial_unknown_nibble.map(|value| value.to_string()),
        ),
    ];

    for channel in &hsc.channels {
        lines.extend(hsc_channel_lines(channel));
    }

    lines
}

fn hsc_channel_lines(channel: &HscChannelSummary) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Channel {}", channel.channel),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field(
            "  Counter Mode",
            Some(format!(
                "{} ({})",
                hsc_counter_mode(channel),
                hsc_raw_nibble(channel.counter_mode_raw),
            )),
        ),
        field(
            "  Pulse Input Mode",
            Some(format!(
                "{} ({})",
                hsc_pulse_input_mode(channel),
                hsc_raw_nibble(channel.pulse_input_mode_raw),
            )),
        ),
        field(
            "  Compare Output Mode",
            Some(hsc_compare_output_mode(channel)),
        ),
        field("  Internal Preset", Some(hsc_u8(channel.internal_preset))),
        field("  External Preset", Some(hsc_u8(channel.external_preset))),
        field(
            "  Ring Counter Max",
            Some(hsc_i32(channel.ring_counter_max)),
        ),
        field(
            "  Compare Output Min",
            Some(hsc_i32(channel.compare_output_min)),
        ),
        field(
            "  Compare Output Max",
            Some(hsc_i32(channel.compare_output_max)),
        ),
        field("  Unit Time", Some(hsc_u16(channel.unit_time_ms))),
        field(
            "  Pulses Per Revolution",
            Some(hsc_u16(channel.pulses_per_revolution)),
        ),
        field("  Raw bytes", Some(channel.raw.len().to_string())),
    ]
}

fn hsc_counter_mode(channel: &HscChannelSummary) -> String {
    channel
        .counter_mode
        .map(|mode| mode.to_string())
        .unwrap_or_else(|| "<none>".to_owned())
}

fn hsc_pulse_input_mode(channel: &HscChannelSummary) -> String {
    channel
        .pulse_input_mode
        .map(|mode| mode.to_string())
        .unwrap_or_else(|| "<none>".to_owned())
}

fn hsc_compare_output_mode(channel: &HscChannelSummary) -> String {
    match (channel.compare_output_mode, channel.compare_output_mode_raw) {
        (Some(mode), Some(raw)) => format!("{mode} ({raw})"),
        (Some(mode), None) => mode.to_string(),
        (None, Some(raw)) => format!("<unknown> ({raw})"),
        (None, None) => "<none>".to_owned(),
    }
}

fn hsc_raw_nibble(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_owned())
}

fn hsc_u8(value: Option<u8>) -> String {
    value
        .map(|value| format!("0x{value:02X} ({value})"))
        .unwrap_or_else(|| "<none>".to_owned())
}

fn hsc_u16(value: Option<u16>) -> String {
    value
        .map(|value| format!("0x{value:04X} ({value})"))
        .unwrap_or_else(|| "<none>".to_owned())
}

fn hsc_i32(value: Option<i32>) -> String {
    value
        .map(|value| format!("0x{:08X} ({value})", value as u32))
        .unwrap_or_else(|| "<none>".to_owned())
}

fn position_parameter_lines(position: &PositionParameterSummary) -> Vec<Line<'static>> {
    let mut lines = vec![field(
        "Axis Count",
        position.axis_count.map(|value| value.to_string()),
    )];

    for axis in &position.axes {
        lines.extend(position_axis_lines(axis));
    }

    lines
}

fn position_axis_lines(axis: &PositionAxisSummary) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Axis", axis.axis_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field(
            "  Step Count",
            axis.step_count.map(|value| value.to_string()),
        ),
        field("  Steps Parsed", Some(axis.steps.len().to_string())),
    ];

    if let Some(parameter) = axis.parameter.as_ref() {
        lines.extend([
            field(
                "  Bias Velocity",
                parameter.bias_velocity.map(|value| value.to_string()),
            ),
            field(
                "  Velocity Limit",
                parameter.velocity_limit.map(|value| value.to_string()),
            ),
            field("  Accel Times", Some(position_array(parameter.accel_times))),
            field("  Decel Times", Some(position_array(parameter.decel_times))),
            field(
                "  Soft Upper Limit",
                parameter.soft_upper_limit.map(|value| value.to_string()),
            ),
            field(
                "  Soft Lower Limit",
                parameter.soft_lower_limit.map(|value| value.to_string()),
            ),
            field(
                "  Backlash Compensation",
                parameter
                    .backlash_compensation
                    .map(|value| value.to_string()),
            ),
            field(
                "  S-Curve Ratio",
                parameter.s_curve_ratio.map(|value| value.to_string()),
            ),
            field(
                "  Use Limit",
                parameter.use_limit.map(|value| value.to_string()),
            ),
            field(
                "  Pulse Output Mode",
                parameter.pulse_output_mode.map(|value| value.to_string()),
            ),
            field(
                "  Orientation",
                parameter.orientation.map(|value| value.to_string()),
            ),
            field(
                "  Return Velocity",
                Some(format!(
                    "high={} | low={}",
                    option_u32(parameter.return_velocity_high),
                    option_u32(parameter.return_velocity_low),
                )),
            ),
            field(
                "  Return Timing",
                Some(format!(
                    "accel={} | decel={} | dwell={}",
                    option_u32(parameter.return_accel_time),
                    option_u32(parameter.return_decel_time),
                    option_u32(parameter.return_dwell_time),
                )),
            ),
            field(
                "  Return Mode",
                Some(format!(
                    "policy={} | direction={}",
                    option_u32(parameter.return_policy),
                    option_u32(parameter.return_direction),
                )),
            ),
            field(
                "  Jog Timing",
                Some(format!(
                    "accel={} | decel={} | inching={}",
                    option_u32(parameter.jog_accel_time),
                    option_u32(parameter.jog_decel_time),
                    option_u32(parameter.inching_time),
                )),
            ),
            field(
                "  Jog Velocity",
                Some(format!(
                    "high={} | low={}",
                    option_u32(parameter.jog_velocity_high),
                    option_u32(parameter.jog_velocity_low),
                )),
            ),
            field(
                "  Interpolation Method",
                parameter
                    .interpolation_method
                    .map(|value| value.to_string()),
            ),
        ]);
    } else {
        lines.push(Line::from("  <missing axis parameter>"));
    }

    if let Some(step) = axis.steps.first() {
        lines.push(field(
            "  First Step",
            Some(format!(
                "target={} | velocity={} | dwell={} | mode={}",
                option_i32(step.target_position),
                option_u32(step.operation_velocity),
                option_u32(step.dwell_time),
                option_u32(step.operation_mode),
            )),
        ));
    }

    lines
}

fn position_array(values: [Option<u32>; 4]) -> String {
    values
        .into_iter()
        .map(option_u32)
        .collect::<Vec<_>>()
        .join(", ")
}

fn option_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<none>".to_owned())
}

fn option_i32(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<none>".to_owned())
}

fn pid_cal_parameter_lines(pid: &PidCalParameterSummary) -> Vec<Line<'static>> {
    let mut lines = vec![
        field("Loops", Some(pid.loops.len().to_string())),
        field("Header", Some(option_u32_array(pid.header))),
        field(
            "Parameter Size",
            pid.parameter_size.map(|value| value.to_string()),
        ),
        field(
            "Set PID Out",
            pid.set_pid_out.map(|value| value.to_string()),
        ),
        field(
            "Set Direction",
            pid.set_direction.map(|value| value.to_string()),
        ),
        field(
            "Prevent Anti Windup",
            pid.prevent_anti_windup.map(|value| value.to_string()),
        ),
        field(
            "Control Method",
            Some(format!(
                "P={} | D={}",
                option_u32(pid.proportional_control_method),
                option_u32(pid.differential_control_method),
            )),
        ),
        field("Permit PWM", pid.permit_pwm.map(|value| value.to_string())),
    ];

    for loop_data in &pid.loops {
        lines.extend(pid_cal_loop_lines(loop_data));
    }

    lines
}

fn pid_cal_loop_lines(loop_data: &PidCalLoopSummary) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Loop {}", loop_data.loop_index),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field(
            "  Target Value",
            loop_data.target_value.map(|value| value.to_string()),
        ),
        field(
            "  Scan Time",
            loop_data.scan_time.map(|value| value.to_string()),
        ),
        field(
            "  Gain",
            Some(format!(
                "P={}.{} | I={}.{} | D={}.{}",
                option_i32(loop_data.proportional_gain_left),
                option_i32(loop_data.proportional_gain_right),
                option_i32(loop_data.integral_gain_left),
                option_i32(loop_data.integral_gain_right),
                option_i32(loop_data.differential_gain_left),
                option_i32(loop_data.differential_gain_right),
            )),
        ),
        field(
            "  MV",
            Some(format!(
                "min={} | max={} | manual={} | limit={}",
                option_i32(loop_data.mv_min),
                option_i32(loop_data.mv_max),
                option_i32(loop_data.mv_manual),
                option_u32(loop_data.mv_limit),
            )),
        ),
        field(
            "  PV",
            Some(format!(
                "min={} | max={} | limit={} | tracking={}",
                option_i32(loop_data.pv_min),
                option_i32(loop_data.pv_max),
                option_u32(loop_data.pv_limit),
                option_i32(loop_data.pv_tracking_set_value),
            )),
        ),
        field(
            "  Dead Band",
            loop_data.dead_band.map(|value| value.to_string()),
        ),
        field(
            "  PWM",
            Some(format!(
                "forward={} ({}) | period={}",
                option_u32(loop_data.forward_pwm),
                pid_pwm_p_address(loop_data.forward_pwm),
                option_u32(loop_data.pwm_out_period),
            )),
        ),
    ]
}

fn pid_tune_parameter_lines(pid: &PidTuneParameterSummary) -> Vec<Line<'static>> {
    let mut lines = vec![
        field("Loops", Some(pid.loops.len().to_string())),
        field(
            "Set Direction",
            pid.set_direction.map(|value| value.to_string()),
        ),
        field("Permit PWM", pid.permit_pwm.map(|value| value.to_string())),
        field("Checksum", pid.checksum.map(|value| value.to_string())),
        field("Footer", Some(option_u32_array(pid.footer))),
    ];

    for loop_data in &pid.loops {
        lines.extend(pid_tune_loop_lines(loop_data));
    }

    lines
}

fn pid_tune_loop_lines(loop_data: &PidTuneLoopSummary) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Loop {}", loop_data.loop_index),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field(
            "  Target Value",
            loop_data.target_value.map(|value| value.to_string()),
        ),
        field(
            "  Scan Time",
            loop_data.scan_time.map(|value| value.to_string()),
        ),
        field(
            "  MV",
            Some(format!(
                "min={} | max={}",
                option_i32(loop_data.mv_min),
                option_i32(loop_data.mv_max),
            )),
        ),
        field(
            "  PWM",
            Some(format!(
                "point={} ({}) | period={}",
                option_u32(loop_data.set_pwm_at_point),
                pid_pwm_p_address(loop_data.set_pwm_at_point),
                option_u32(loop_data.out_period),
            )),
        ),
        field(
            "  Hysteresis",
            loop_data.hysteresis.map(|value| value.to_string()),
        ),
    ]
}

fn option_u32_array(values: [Option<u32>; 2]) -> String {
    values
        .into_iter()
        .map(option_u32)
        .collect::<Vec<_>>()
        .join(", ")
}

fn pid_pwm_p_address(value: Option<u32>) -> String {
    value
        .map(|value| {
            let high = value / 16;
            let low = value % 16;
            format!("P{high:04}{low:X}")
        })
        .unwrap_or_else(|| "<none>".to_owned())
}

fn cnet_config_lines(config: &CnetConfigInfoSummary) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "Cnet base={} slot={} type={}",
                option_u32(config.base),
                option_u32(config.slot),
                option_u32(config.type_code),
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field(
            "  Station",
            config.station_no.map(|value| value.to_string()),
        ),
        field("  SubType", config.sub_type.map(|value| value.to_string())),
        field("  Ports", Some(config.ports.len().to_string())),
    ];

    for (index, port) in config.ports.iter().enumerate() {
        lines.extend(cnet_port_lines(index + 1, port, "  "));
    }

    lines
}

fn cnet_port_lines(
    index: usize,
    port: &CnetPortConfigSummary,
    indent: &'static str,
) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            format!("{indent}Port {index}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field(
            format!("{indent}  Station").as_str(),
            port.station_no.map(|value| value.to_string()),
        ),
        field(format!("{indent}  Mode").as_str(), Some(cnet_mode(port))),
        field(
            format!("{indent}  Serial").as_str(),
            Some(format!(
                "baud={} ({}) | data={} | stop={} | parity={}",
                option_u32(port.baud_rate),
                option_u32(port.bps),
                cnet_data_bits(port),
                cnet_stop_bits(port),
                cnet_parity(port),
            )),
        ),
        field(
            format!("{indent}  Timeout").as_str(),
            Some(format!(
                "rx={} | char={} | inter-char={}",
                option_u32(port.rx_timeout),
                option_u32(port.char_timeout),
                option_u32(port.inter_char_timeout),
            )),
        ),
        field(
            format!("{indent}  Driver Type").as_str(),
            port.driver_type.map(|value| value.to_string()),
        ),
        field(
            format!("{indent}  DI").as_str(),
            Some(cnet_device_range(
                port.di_device,
                port.di_address.as_deref(),
                port.di_device_type,
                port.di_data_type,
                port.di_size,
                port.di_addr,
            )),
        ),
        field(
            format!("{indent}  DO").as_str(),
            Some(cnet_device_range(
                port.do_device,
                port.do_address.as_deref(),
                port.do_device_type,
                port.do_data_type,
                port.do_size,
                port.do_addr,
            )),
        ),
        field(
            format!("{indent}  AI").as_str(),
            Some(cnet_device_range(
                port.ai_device,
                port.ai_address.as_deref(),
                port.ai_device_type,
                port.ai_data_type,
                port.ai_size,
                port.ai_addr,
            )),
        ),
        field(
            format!("{indent}  AO").as_str(),
            Some(cnet_device_range(
                port.ao_device,
                port.ao_address.as_deref(),
                port.ao_device_type,
                port.ao_data_type,
                port.ao_size,
                port.ao_addr,
            )),
        ),
        field(
            format!("{indent}  Terminating Resistor").as_str(),
            port.terminating_resister.map(|value| value.to_string()),
        ),
        field(
            format!("{indent}  Repeater").as_str(),
            port.repeater.map(|value| value.to_string()),
        ),
    ]
}

fn cnet_device_range(
    device: Option<char>,
    address: Option<&str>,
    device_type: Option<u32>,
    data_type: Option<u32>,
    size: Option<u32>,
    addr: Option<u32>,
) -> String {
    format!(
        "address={} | device={} ({}) | data={} | size={} | addr={}",
        address.unwrap_or("<none>"),
        device
            .map(|value| value.to_string())
            .unwrap_or_else(|| "<none>".to_owned()),
        option_u32(device_type),
        option_u32(data_type),
        option_u32(size),
        option_u32(addr),
    )
}

fn cnet_data_bits(port: &CnetPortConfigSummary) -> String {
    port.data_bits
        .map(|value| format!("{} ({})", value.label(), option_u32(port.data_bit)))
        .unwrap_or_else(|| option_u32(port.data_bit))
}

fn cnet_mode(port: &CnetPortConfigSummary) -> String {
    port.mode_kind
        .map(|value| format!("{} ({})", value.label(), option_u32(port.mode)))
        .unwrap_or_else(|| option_u32(port.mode))
}

fn cnet_stop_bits(port: &CnetPortConfigSummary) -> String {
    port.stop_bits
        .map(|value| format!("{} ({})", value.label(), option_u32(port.stop_bit)))
        .unwrap_or_else(|| option_u32(port.stop_bit))
}

fn cnet_parity(port: &CnetPortConfigSummary) -> String {
    port.parity_mode
        .map(|value| format!("{} ({})", value.label(), option_u32(port.parity)))
        .unwrap_or_else(|| option_u32(port.parity))
}

fn fenet_config_lines(config: &FenetConfigInfoSummary) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "FEnet base={} slot={} type={}",
                option_u32(config.base),
                option_u32(config.slot),
                option_u32(config.type_code),
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field(
            "  Station",
            config.station_no.map(|value| value.to_string()),
        ),
        field("  SubType", config.sub_type.map(|value| value.to_string())),
        field("  IP Address", Some(ipv4(config.ip_address.as_ref()))),
        field("  Subnet", Some(ipv4(config.subnet.as_ref()))),
        field("  Gateway", Some(ipv4(config.gateway.as_ref()))),
        field("  DNS", Some(ipv4(config.dns.as_ref()))),
        field("  Secondary IP", Some(ipv4(config.ip_address2.as_ref()))),
        field("  Secondary Subnet", Some(ipv4(config.subnet2.as_ref()))),
        field("  Secondary Gateway", Some(ipv4(config.gateway2.as_ref()))),
        field("  Secondary DNS", Some(ipv4(config.dns2.as_ref()))),
        field("  DHCP", config.dhcp.map(|value| value.to_string())),
        field(
            "  Driver Type",
            config.driver_type.map(|value| value.to_string()),
        ),
        field(
            "  Rcv Wait",
            config.rcv_wait_time.map(|value| value.to_string()),
        ),
        field(
            "  Client Wait",
            config.client_wait_time.map(|value| value.to_string()),
        ),
        field(
            "  Glofa Sockets",
            config.glofa_socket_count.map(|value| value.to_string()),
        ),
    ]
}

fn ipv4(value: Option<&Ipv4Summary>) -> String {
    value
        .map(|value| value.address.clone())
        .unwrap_or_else(|| "<none>".to_owned())
}

fn data_details(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.push(heading("Options"));
    if let Some(options) = app.doc.project_options() {
        lines.push(field("Entries", Some(options.entries.len().to_string())));
        for entry in &options.entries {
            lines.push(field(
                format!("  {}", entry.key).as_str(),
                Some(entry.value.clone()),
            ));
        }
    } else {
        lines.push(Line::from("  <none>"));
    }

    if let Some(trend) = app.doc.trend_monitoring() {
        lines.push(Line::from(""));
        lines.push(heading("Trend Monitoring"));
        if let Some(trace) = trend.trace_configuration.as_ref() {
            lines.extend(section_lines(trace));
        }
        if let Some(graph) = trend.graph_configuration.as_ref() {
            lines.extend(section_lines(graph));
        }
        if let Some(window) = trend.window_configuration.as_ref() {
            lines.extend(section_lines(window));
        }
    }

    let xgpd = app.doc.xgpd_config_infos();
    if !xgpd.is_empty() {
        lines.push(Line::from(""));
        lines.push(heading("XGPD Config"));
        for info in xgpd {
            lines.push(Line::from(format!(
                "  DNET station={:?} type={:?} base={:?} slot={:?} sub={:?}",
                info.station_no, info.type_code, info.base, info.slot, info.sub_type
            )));
        }
    }

    let properties = app.doc.properties();
    if !properties.is_empty() {
        lines.push(Line::from(""));
        lines.push(heading("Properties"));
        for property in properties {
            lines.push(field("Value", property.value.as_deref().map(truncate_long)));
        }
    }

    lines.push(Line::from(""));
    lines.push(heading("Selected Payload"));
    match app.decoded_payloads.get(app.data_selected) {
        Some(Ok(payload)) => {
            lines.push(field("Path", Some(payload.path.clone())));
            lines.push(field("Compressed", Some(payload.compressed.to_string())));
            lines.push(field("Encoded", Some(payload.encoded_len.to_string())));
            lines.push(field("Raw", Some(payload.raw_len.to_string())));
            lines.push(field("Decoded", Some(payload.decoded_len.to_string())));
            lines.push(field(
                "Attributes",
                Some(summarize_attributes(&payload.attributes)),
            ));
        }
        Some(Err(error)) => {
            lines.push(Line::from(vec![
                Span::styled("Decode error: ", Style::default().fg(Color::Red)),
                Span::raw(error.to_string()),
            ]));
        }
        None => lines.push(Line::from("  <none>")),
    }

    lines
}

fn section_lines(section: &XmlSectionSummary) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            format!("  {}", section.name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field("    Children", Some(section.child_count.to_string())),
        field(
            "    Attributes",
            Some(summarize_attributes(&section.attributes)),
        ),
    ]
}

fn attribute_lines(prefix: &str, attributes: &[XmlAttribute]) -> Vec<Line<'static>> {
    attributes
        .iter()
        .map(|attribute| {
            field(
                format!("{prefix}{}", attribute.name).as_str(),
                Some(attribute.value.clone()),
            )
        })
        .collect()
}

fn grouped_attribute_lines(prefix: &str, attributes: &[XmlAttribute]) -> Vec<Line<'static>> {
    let mut groups: Vec<GroupedAttributes<'_>> = Vec::new();

    for attribute in attributes {
        if let Some((base, index)) = split_indexed_attribute(attribute.name.as_str()) {
            if let Some(group) = groups.iter_mut().find(|group| group.name == base) {
                group.values.push((index, attribute.value.as_str()));
            } else {
                groups.push(GroupedAttributes {
                    name: base,
                    values: vec![(index, attribute.value.as_str())],
                    scalar: None,
                });
            }
        } else {
            groups.push(GroupedAttributes {
                name: attribute.name.as_str(),
                values: Vec::new(),
                scalar: Some(attribute.value.as_str()),
            });
        }
    }

    let mut lines = Vec::new();
    for group in &mut groups {
        if let Some(value) = group.scalar {
            lines.push(field(
                format!("{prefix}{}", group.name).as_str(),
                Some(value.to_owned()),
            ));
        } else {
            group.values.sort_by_key(|(index, _)| *index);
            lines.push(field(
                format!("{prefix}{}", group.name).as_str(),
                Some(format!("{} values", group.values.len())),
            ));
            for (index, value) in &group.values {
                lines.push(field(
                    format!("{prefix}  [{index}]").as_str(),
                    Some((*value).to_owned()),
                ));
            }
        }
    }

    lines
}

struct GroupedAttributes<'a> {
    name: &'a str,
    values: Vec<(usize, &'a str)>,
    scalar: Option<&'a str>,
}

fn split_indexed_attribute(name: &str) -> Option<(&str, usize)> {
    let (base, suffix) = name.rsplit_once('_')?;
    if base.is_empty() || suffix.is_empty() {
        return None;
    }
    let index = suffix.parse().ok()?;
    Some((base, index))
}

fn variable_details(variable: Option<&VariableSummary>) -> Vec<Line<'static>> {
    let Some(variable) = variable else {
        return vec![Line::from("No variables found.")];
    };

    vec![
        field("Name", variable.name.clone()),
        field("Format", variable.format_version.clone()),
        field("Address", variable.address.clone()),
        field("Area", variable.address_area.clone()),
        field(
            "Number",
            variable.address_number.map(|number| number.to_string()),
        ),
        field("Data Type", variable.data_type.clone()),
        field("Source Ref", variable.source_ref.clone()),
        field("Range", variable.range.clone()),
        field(
            "Description",
            variable.description.as_deref().map(truncate_long),
        ),
    ]
}

fn network_details(app: &App, network: Option<&NetworkSummary>) -> Vec<Line<'static>> {
    let Some(network) = network else {
        return vec![Line::from("No networks found.")];
    };

    let fenet_configs = app.doc.fenet_config_infos();
    let cnet_configs = app.doc.cnet_config_infos();
    let mut lines = vec![
        field("Name", network.name.clone()),
        field("Type", network.type_name.clone()),
        field("Network Type", network.network_type.clone()),
        Line::from(""),
        Line::from(Span::styled(
            "Modules",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    if network.modules.is_empty() {
        lines.push(Line::from("  <none>"));
    } else {
        for (index, module) in network.modules.iter().enumerate() {
            lines.extend(network_module_lines(
                index + 1,
                module,
                &cnet_configs,
                &fenet_configs,
            ));
        }
    }

    lines
}

fn heading(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        label.to_owned(),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ))
}

fn network_module_lines(
    index: usize,
    module: &NetworkModuleSummary,
    cnet_configs: &[CnetConfigInfoSummary],
    fenet_configs: &[FenetConfigInfoSummary],
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            format!(
                "  {index}. {}",
                module.config_name.as_deref().unwrap_or("<unnamed>")
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        field("    Name", module.name.as_deref().map(truncate_long)),
        field("    Base", module.base.map(|value| value.to_string())),
        field("    Slot", module.slot.map(|value| value.to_string())),
        field("    Id", module.id.map(|value| value.to_string())),
        field(
            "    Option Type",
            module.option_type.map(|value| value.to_string()),
        ),
    ];

    let matching_cnet_configs = cnet_configs
        .iter()
        .filter(|config| module.id.is_some() && module.id == config.type_code)
        .collect::<Vec<_>>();
    for (config_index, config) in matching_cnet_configs.iter().enumerate() {
        lines.push(field(
            "    Cnet Config",
            Some(format!(
                "{}{}",
                option_u32(config.type_code),
                if matching_cnet_configs.len() > 1 {
                    format!(" ({}/{})", config_index + 1, matching_cnet_configs.len())
                } else {
                    String::new()
                }
            )),
        ));
        lines.push(field(
            "      Station",
            config.station_no.map(|value| value.to_string()),
        ));
        lines.push(field(
            "      SubType",
            config.sub_type.map(|value| value.to_string()),
        ));
        for (port_index, port) in config.ports.iter().enumerate() {
            lines.extend(cnet_port_lines(port_index + 1, port, "      "));
        }
    }

    let matching_configs = fenet_configs
        .iter()
        .filter(|config| module.id.is_some() && module.id == config.type_code)
        .collect::<Vec<_>>();

    for (config_index, config) in matching_configs.iter().enumerate() {
        lines.push(field(
            "    FEnet Config",
            Some(format!(
                "{}{}",
                option_u32(config.type_code),
                if matching_configs.len() > 1 {
                    format!(" ({}/{})", config_index + 1, matching_configs.len())
                } else {
                    String::new()
                }
            )),
        ));
        lines.push(field(
            "      IP Address",
            Some(ipv4(config.ip_address.as_ref())),
        ));
        lines.push(field("      Subnet", Some(ipv4(config.subnet.as_ref()))));
        lines.push(field("      Gateway", Some(ipv4(config.gateway.as_ref()))));
        lines.push(field("      DNS", Some(ipv4(config.dns.as_ref()))));
    }

    lines
}

fn push_xml_tree(element: &XmlElement, depth: usize, lines: &mut Vec<Line<'static>>) {
    let indent = "  ".repeat(depth);
    let text = element.text.trim();
    let text = (!text.is_empty()).then(|| truncate_long(text));
    let attrs = summarize_attributes(&element.attributes);
    let suffix = match (attrs.is_empty(), text) {
        (true, None) => String::new(),
        (false, None) => format!(" {attrs}"),
        (true, Some(text)) => format!(" text=\"{text}\""),
        (false, Some(text)) => format!(" {attrs} text=\"{text}\""),
    };

    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            element.name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(suffix),
    ]));

    for child in &element.children {
        push_xml_tree(child, depth + 1, lines);
    }
}

fn summarize_attributes(attributes: &[xgwx::XmlAttribute]) -> String {
    let parts = attributes
        .iter()
        .map(|attribute| format!("{}=\"{}\"", attribute.name, attribute.value))
        .collect::<Vec<_>>();

    if parts.is_empty() {
        String::new()
    } else {
        format!("[{}]", parts.join(" "))
    }
}

fn truncate_long(value: &str) -> String {
    value.to_owned()
}

fn field(label: &str, value: Option<String>) -> Line<'static> {
    let value = value.unwrap_or_else(|| "<none>".to_owned());
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}
