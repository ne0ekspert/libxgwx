//! Read-only protocol views. These mappings deliberately do not require the
//! library's WASM-only Serde implementations or expose opaque program bytes.

use crate::*;
use serde_json::{Value, json};

pub const SECTIONS: &[&str] = &[
    "overview",
    "user_pous",
    "programs",
    "program",
    "symbols",
    "hardware",
    "networks",
    "networks.opcua_tags.<configuration_index>",
    "sfc",
    "text",
    "vendor_il",
    "text.variables",
    "program.rows",
    "program.records",
    "program.functions",
    "program.references",
    "program.operand_links",
    "program.graph",
    "program.edit_sites",
    "program.strings",
    "program.cells",
    "sfc.variables",
    "sfc.block.<block_index>",
    "catalog.cpu",
    "catalog.modules",
    "catalog.instructions",
    "catalog.comparisons",
    "io_variables",
];

macro_rules! fields {
    ($value:expr; $($field:ident),* $(,)?) => {{
        let value = $value;
        let mut mapped = serde_json::Map::new();
        $(mapped.insert(stringify!($field).to_owned(), json!(&value.$field));)*
        Value::Object(mapped)
    }};
}

fn attrs(attributes: &[XmlAttribute]) -> Value {
    Value::Object(
        attributes
            .iter()
            .map(|a| (a.name.clone(), json!(a.value)))
            .collect(),
    )
}

/// `offset` applies independently to every list in a composite section. The
/// detailed sections allow clients to paginate one view without re-reading all.
fn page<T>(items: &[T], offset: usize, limit: usize, map: impl Fn(&T) -> Value) -> Value {
    let limit = limit.clamp(1, 1000);
    let end = offset.saturating_add(limit).min(items.len());
    let selected = items
        .iter()
        .skip(offset)
        .take(limit)
        .map(map)
        .collect::<Vec<_>>();
    json!({
        "items": selected, "total": items.len(), "offset": offset, "limit": limit,
        "next_offset": if end < items.len() { Some(end) } else { None },
    })
}

fn indexed_page<T>(
    items: &[T],
    offset: usize,
    limit: usize,
    map: impl Fn(usize, &T) -> Value,
) -> Value {
    let indexed = items.iter().enumerate().collect::<Vec<_>>();
    page(&indexed, offset, limit, |(index, item)| map(*index, item))
}

fn optional_page<T>(
    items: Option<Vec<T>>,
    offset: usize,
    limit: usize,
    map: impl Fn(&T) -> Value,
) -> Value {
    match items {
        Some(items) => {
            let mut result = page(&items, offset, limit, map);
            result["status"] = json!("decoded");
            result
        }
        None => json!({"status": "unsupported_layout", "items": [], "total": null,
            "note": "This decoder does not recognize this layout; this does not establish native invalidity."}),
    }
}

fn program_summary(index: usize, program: &ProgramSummary) -> Value {
    let mut result = fields!(program; name, task, version, local_variable, kind, instance_name,
        comment, object_id, find_program, find_var, encryption);
    result["program_index"] = json!(index);
    result["attributes"] = attrs(&program.attributes);
    result
}

fn configuration(value: &ConfigurationSummary) -> Value {
    let mut result =
        fields!(value; name, version, attribute, comment, kind, type_code, guid, write_signature);
    result["attributes"] = attrs(&value.attributes);
    result["cpu"] = cpu_catalog()
        .iter()
        .find(|cpu| Some(cpu.type_code) == value.type_code)
        .map(cpu)
        .unwrap_or(Value::Null);
    result
}

fn cpu(value: &CpuCatalogEntry) -> Value {
    fields!(value; model, family, type_code, max_base, max_slot)
}

#[cfg(feature = "write")]
fn module_catalog(value: &ModuleCatalogEntry) -> Value {
    let mut result =
        fields!(value; model, category, id, sub_type, name, details, slot_span, status);
    result["options"] = json!(
        value
            .options
            .iter()
            .map(|o| {
                let mut result = fields!(o; key, label, section, scope, count);
                result["encoding"] = fields!(&o.encoding; kind, offset, second_offset, stride,
            bits, shift_base, channels_per_word, bank_stride);
                result["values"] = json!(
                    o.values
                        .iter()
                        .map(|v| fields!(v; label, value))
                        .collect::<Vec<_>>()
                );
                result
            })
            .collect::<Vec<_>>()
    );
    result["visible_options"] = json!(
        value
            .visible_options
            .iter()
            .map(|o| fields!(o;
        key, label, section, scope, items_per_parent, count, default_value, choices))
            .collect::<Vec<_>>()
    );
    result
}

fn instruction(value: &LadderInstructionSpec) -> Value {
    let mut result = fields!(value; mnemonic, opcode, operand_count);
    result["operand_rules"] = json!(
        ladder_instruction_operand_rules(value.mnemonic)
            .iter()
            .map(|r| fields!(r; label, data_types, device_areas, allows_constant, manual_page))
            .collect::<Vec<_>>()
    );
    result["cpu_restriction"] = ladder_instruction_cpu_restriction(value.mnemonic)
        .map(|r| fields!(&r; models, manual_page))
        .unwrap_or(Value::Null);
    result
}

fn global_symbol(index: usize, value: &VariableSummary) -> Value {
    let mut result = fields!(value; format_version, name, address_area, address_number,
        data_type, description, address, source_ref, range);
    result["variable_index"] = json!(index);
    result
}

fn local_symbol(index: usize, value: &IecLocalSymbol) -> Value {
    let mut result = fields!(value; record_offset, name, address, type_reference, data_type_code,
        data_type, storage_class, allocation_number, allocation_width, description, is_instance);
    result["symbol_index"] = json!(index);
    result
}

fn symbols(doc: &XgwxDocument, program_index: Option<usize>, offset: usize, limit: usize) -> Value {
    let globals = match doc.variable().variables() {
        Ok(items) => indexed_page(&items, offset, limit, global_symbol),
        Err(error) => json!({"status": "decode_error", "error": error.to_string()}),
    };
    let locals = doc
        .variable()
        .iec_local_symbols()
        .into_iter()
        .enumerate()
        .filter(|(index, _)| program_index.is_none_or(|selected| selected == *index))
        .map(|(index, items)| {
            json!({"program_index": index, "symbols": match items {
                Ok(items) => indexed_page(&items, offset, limit, local_symbol),
                Err(error) => json!({"status": "decode_error", "error": error.to_string()}),
            }})
        })
        .collect::<Vec<_>>();
    json!({"global": globals, "local": page(&locals,
        if program_index.is_some() { 0 } else { offset }, limit, Clone::clone),
        "system_bool_names": IEC_SYSTEM_BOOL_VARIABLES,
        "note": "Local symbol record_offset and identity are revision-specific. SFC declarations are in the sfc section; standalone ST/IL declarations with full snapshots are in text.variables."})
}

fn typed_module(module: &Module) -> Value {
    let (kind, scope, hardware) = match module {
        Module::Io(io) => ("io", io.base.configuration_index, Some(&io.base.hardware)),
        Module::Other(hw) => ("other", hw.configuration_index, Some(&hw.hardware)),
        Module::Network(net) => (
            "network",
            net.configuration_index,
            net.base.as_ref().map(|hw| &hw.hardware),
        ),
    };
    let mut result = json!({"kind":kind, "configuration_index":scope,
        "hardware": hardware.map(|hw| { let mut v = fields!(hw; base, slot, id, sub_type, name, comment, details);
            v["attributes"] = attrs(&hw.attributes); v })});
    if let Module::Io(io) = module {
        result["io_kind"] = json!(format!("{:?}", io.kind));
    }
    if let Module::Network(net) = module {
        let mut metadata = network_module(0, &net.metadata);
        metadata.as_object_mut().unwrap().remove("module_index");
        result["network"] = metadata;
        result["hardware_candidates"] = json!(
            net.hardware_candidates
                .iter()
                .map(|hw| {
                    let mut value =
                        fields!(&hw.hardware; base, slot, id, sub_type, name, comment, details);
                    value["attributes"] = attrs(&hw.hardware.attributes);
                    json!({"configuration_index":hw.configuration_index, "hardware":value})
                })
                .collect::<Vec<_>>()
        );
        result["configuration"] = match &net.configuration {
            ConfigurationMatch::Missing => json!({"status":"missing"}),
            ConfigurationMatch::Matched(c) => {
                json!({"status":"matched", "settings": typed_settings(c)})
            }
            ConfigurationMatch::Ambiguous(cs) => {
                json!({"status":"ambiguous", "candidates":cs.iter().map(typed_settings).collect::<Vec<_>>()})
            }
        };
    }
    result
}
fn typed_settings(config: &NetworkConfiguration) -> Value {
    match config {
        NetworkConfiguration::Cnet(c) => json!({"kind":"cnet", "value": cnet_config(c)}),
        NetworkConfiguration::Fenet(c) => json!({"kind":"fenet", "value": fenet_config(c)}),
        NetworkConfiguration::Other(c) => {
            let mut value = fields!(c; kind, station_no, type_code, base, slot, sub_type);
            value["attributes"] = attrs(&c.attributes);
            json!({"kind":"other", "value":value})
        }
    }
}
fn typed_modules(doc: &XgwxDocument, offset: usize, limit: usize) -> Value {
    page(&doc.modules(), offset, limit, typed_module)
}

fn hardware_bases(doc: &XgwxDocument, offset: usize, limit: usize) -> Value {
    page(&doc.bases(), offset, limit, |base| {
        let summary = base.summary();
        json!({"base":base.number(), "slot_count":base.slot_count(), "base_index":base.index(),
            "configuration_index":base.configuration_index(), "attributes":attrs(&summary.attributes),
            "modules":page(&base.modules(), offset, limit, typed_module)})
    })
}

fn hardware(doc: &XgwxDocument, offset: usize, limit: usize) -> Value {
    let modules = doc.module().summaries();
    let selections = modules
        .iter()
        .enumerate()
        .map(|(module_index, m)| {
            let mut result =
                fields!(m; base, slot, id, sub_type, name, comment, details, input_filter_raw);
            result["module_index"] = json!(module_index);
            result["attributes"] = attrs(&m.attributes);
            result["input_filter"] = m
                .input_filter
                .map(|filter| json!(super::inspection_types::ModuleInputFilter::from(filter)))
                .unwrap_or(Value::Null);
            result["input_filter_label"] = m.input_filter.map(|filter| filter.to_string()).into();
            #[cfg(feature="write")]
            { result["option_values"] = match (m.base, m.slot) {
                (Some(base), Some(slot)) => match doc
                    .module()
                    .at(base, slot)
                    .and_then(|selected_record| selected_record.option_values())
                {
                    Ok(options) => json!({"status": "decoded", "items": options.iter()
                    .map(|o| fields!(o; key, index, value)).collect::<Vec<_>>()}),
                    Err(error) => json!({"status": "unsupported", "error": error.to_string()}),
                },
                _ => json!({"status": "unsupported", "error": "Module has no base/slot identity"}),
            };
            }
            result
        })
        .collect::<Vec<_>>();
    json!({
        "configurations": page(&doc.configuration().list(), offset, limit, configuration),
        "bases": hardware_bases(doc, offset, limit),
        "typed_modules": typed_modules(doc, offset, limit),
        "modules": page(&selections, offset, limit, Clone::clone),
        "cpu_hardware_profile": doc.configuration().get(0).ok().and_then(|plc| plc.hardware_profile()).map(|p| fields!(p;
            cpu_model, variant, builtin_io_base, builtin_io_slot, builtin_io_id,
            builtin_io_sub_types, builtin_network_id, builtin_network_option_type,
            expansion_catalog_supported)),
        "catalog_sections": ["catalog.cpu", "catalog.modules"],
    })
}

fn network_module(index: usize, value: &NetworkModuleSummary) -> Value {
    let mut result = fields!(value; config_name, config_type, type_name, name, base, slot,
        id, channel_type, option_type, alias, description);
    result["module_index"] = json!(index);
    result["attributes"] = attrs(&value.attributes);
    result
}

fn cnet_port(index: usize, port: &CnetPortConfigSummary) -> Value {
    let mut result = fields!(port; station_no, station_no_b, mode, modem, bps, baud_rate,
        data_bit, stop_bit, parity, rx_timeout, char_timeout, inter_char_timeout, driver_type,
        request_delay_time, parity_error_ignore, di_device, di_address, di_device_type,
        di_data_type, di_size, di_addr, do_device, do_address, do_device_type, do_data_type,
        do_size, do_addr, ai_device, ai_address, ai_device_type, ai_data_type, ai_size,
        ai_addr, ao_device, ao_address, ao_device_type, ao_data_type, ao_size, ao_addr,
        terminating_resister, repeater);
    result["port_index"] = json!(index);
    result["attributes"] = attrs(&port.attributes);
    result["mode_label"] = port.mode_kind.map(CnetMode::label).into();
    result["data_bits_label"] = port.data_bits.map(CnetDataBits::label).into();
    result["stop_bits_label"] = port.stop_bits.map(CnetStopBits::label).into();
    result["parity_label"] = port.parity_mode.map(CnetParity::label).into();
    result
}

fn ipv4(value: &Option<Ipv4Summary>) -> Value {
    value
        .as_ref()
        .map(|ip| fields!(ip; octets, address))
        .unwrap_or(Value::Null)
}

fn networks(doc: &XgwxDocument, offset: usize, limit: usize) -> Value {
    json!({
        "networks": indexed_page(&doc.network().list(), offset, limit, |index, network| {
            let mut result = fields!(network; name, type_name, network_type);
            result["network_index"] = json!(index);
            result["attributes"] = attrs(&network.attributes);
            result["modules"] = indexed_page(&network.modules, offset, limit, network_module);
            result
        }),
        "network_modules": indexed_page(&doc.network().module_summaries(), offset, limit, network_module),
        "xgpd_config_infos": indexed_page(&doc.network().config_infos(), offset, limit, |index, value| {
            let mut result = fields!(value; kind, station_no, type_code, base, slot, sub_type);
            result["config_index"] = json!(index);
            result["attributes"] = attrs(&value.attributes);
            result
        }),
        "cnet": indexed_page(&doc.network().cnet_config_infos(), offset, limit, |index, value| {
            let mut result = cnet_config(value);
            result["ports"] = indexed_page(&value.ports, offset, limit, cnet_port);
            result["config_index"] = json!(index);
            result
        }),
        "fenet": indexed_page(&doc.network().fenet_config_infos(), offset, limit, |index, value| {
            let mut result = fenet_config(value);
            result["config_index"] = json!(index);
            result
        }),
        "opcua": indexed_page(&doc.network().opcua_config_infos(), offset, limit, |index,c|json!({"config_index":index,"base":c.base,"slot":c.slot,"endpoint":c.endpoint,"fields":c.fields.iter().map(|f|json!({"field":f.field,"value":f.value})).collect::<Vec<_>>() })),
        "opcua_tags": indexed_page(&doc.configuration().list(),offset,limit,|index,_|match doc.network().opcua_tags(index){Ok(tags)=>json!({"configuration_index":index,"tags":page(&tags,offset,limit,|tag|fields!(tag;name,data_type,address_area,address_number,comment))}),Err(e)=>json!({"configuration_index":index,"error":e.to_string()})}),
        "cnet_baud_rates": CNET_BAUD_RATES,
        "identity_note": "NetworkModule id/config identity is stable; configured base/slot can differ from physical hardware. Attribute maps retain exact expected strings.",
    })
}

fn string(value: &LadderString) -> Value {
    fields!(value; offset, end_offset, value)
}

fn source_strings(program: &LadderProgramData, offset: usize, limit: usize) -> Value {
    let mut records = program.iec_record_frames().unwrap_or_default();
    records.sort_by_key(|record| record.offset);
    page(&program.strings, offset, limit, |value| {
        let mut result = string(value);
        let candidate = records.partition_point(|record| record.offset <= value.offset);
        if let Some(frame) = candidate
            .checked_sub(1)
            .and_then(|index| records.get(index))
            .filter(|frame| value.end_offset <= frame.end)
        {
            result["record"] = positioned_record(program, frame);
        }
        result
    })
}

fn cell(value: &LadderCell) -> Value {
    let mut result = fields!(value; offset, raw_x, raw_y, value, operands);
    result["kind"] = json!(format!("{:?}", value.kind));
    result["contact"] = value
        .contact
        .map(|kind| json!(format!("{kind:?}")))
        .unwrap_or(Value::Null);
    result["coil"] = value
        .coil
        .map(|kind| json!(format!("{kind:?}")))
        .unwrap_or(Value::Null);
    result
}

fn record(value: &IecRecordFrame) -> Value {
    let mut result = fields!(value; group_index, row_index, offset, end);
    let (kind, code) = match value.kind {
        IecRecordKind::LongWire => ("long_wire", None),
        IecRecordKind::ShortWire => ("short_wire", None),
        IecRecordKind::Contact(code) => ("contact", Some(code)),
        IecRecordKind::Coil(code) => ("coil", Some(code)),
        IecRecordKind::Comment => ("comment", None),
        IecRecordKind::FunctionOperand => ("function_operand", None),
        IecRecordKind::BranchStart => ("branch_start", None),
        IecRecordKind::BranchEnd => ("branch_end", None),
        IecRecordKind::LinkReference(code) => ("link_reference", Some(code)),
        IecRecordKind::Label => ("label", None),
        IecRecordKind::FunctionBlock => ("function_block", None),
    };
    result["kind"] = json!(kind);
    result["code"] = json!(code);
    if let Some(code) = code {
        result["element_kind"] = json!(match code {
            6 => "NO",
            7 => "NC",
            8 => "RISING",
            9 => "FALLING",
            10 => "NEGATED_RISING",
            11 => "NEGATED_FALLING",
            14 => "OUTPUT",
            15 => "INVERSE",
            16 => "SET",
            17 => "RESET",
            18 => "RISING",
            19 => "FALLING",
            _ => "unknown",
        });
    }
    result
}

fn positioned_record(program: &LadderProgramData, value: &IecRecordFrame) -> Value {
    let mut result = record(value);
    // Exact IEC frames have already passed the library's captured row grammar.
    // Their shared native coordinate header stores x at byte 5 and y at 6..8.
    result["raw_x"] = program.data.get(value.offset + 5).copied().into();
    result["raw_y"] = json!(u32::from(value.row_index) * 4);
    if matches!(
        value.kind,
        IecRecordKind::Contact(_)
            | IecRecordKind::Coil(_)
            | IecRecordKind::Comment
            | IecRecordKind::FunctionOperand
    ) {
        let string_offset = value.offset + 15;
        if let Some(string) = program.strings.iter().find(|s| s.offset == string_offset) {
            result["operand_string"] = self::string(string);
        }
    }
    result
}

fn pin(value: &IecFunctionPin) -> Value {
    let mut result = fields!(value; raw_type_flags, data_type_mask, data_type, is_array,
        is_control, reference_ordinal, raw_x, row_index);
    result["name"] = string(&value.name);
    result["type_expression"] = value
        .type_expression
        .as_ref()
        .map(string)
        .unwrap_or(Value::Null);
    result["direction"] = json!(match value.direction {
        IecFunctionPinDirection::Input => "input",
        IecFunctionPinDirection::Output => "output",
    });
    result
}

fn function(value: &IecFunctionBlock) -> Value {
    let mut result = fields!(value; group_index, row_index, record_offset, record_end,
        raw_x, opcode_family, opcode, pin_count);
    result["name"] = string(&value.name);
    result["instance"] = value.instance.as_ref().map(string).unwrap_or(Value::Null);
    result["control_input"] = pin(&value.control_input);
    result["control_output"] = pin(&value.control_output);
    result["pins"] = json!(value.pins.iter().map(pin).collect::<Vec<_>>());
    result["field_strings"] = json!(value.field_strings.iter().map(string).collect::<Vec<_>>());
    result
}

fn point(value: &IecCircuitPoint) -> Value {
    fields!(value; group_index, row_index, x)
}

fn graph(value: Option<IecCircuitGraph>, offset: usize, limit: usize) -> Value {
    match value {
        None => json!({"status": "unsupported_layout", "note":
            "Coordinate graph is unavailable: unrecognized records, overlaps, unmatched branches, or invalid pin bindings. Native acceptance remains unverified."}),
        Some(value) => json!({
            "status": if value.open_branch_endpoints.is_empty() { "decoded" } else { "incomplete_circuit" },
            "edges": page(&value.edges, offset, limit, |edge| {
                let mut result = fields!(edge; record_offset, paired_record_offset);
                result["kind"] = json!(format!("{:?}", edge.kind));
                result["start"] = point(&edge.start);
                result["end"] = point(&edge.end);
                result
            }),
            "occupied_areas": page(&value.occupied_areas, offset, limit, |area| {
                let mut result = fields!(area; group_index, start_row_index, end_row_index,
                    start_x, end_x, record_offset);
                result["kind"] = json!(format!("{:?}", area.kind));
                result
            }),
            "function_bindings": page(&value.function_bindings, offset, limit, |binding| {
                let mut result = fields!(binding; group_index, block_record_offset,
                    reference_record_offset, expression_record_offset, ordinal, function_name,
                    pin_name, expression_cell_x, data_type_mask, is_array);
                result["direction"] = json!(match binding.direction {
                    IecFunctionPinDirection::Input => "input", IecFunctionPinDirection::Output => "output",
                });
                result["pin_point"] = point(&binding.pin_point);
                result
            }),
            "power_components": page(&value.power_components, offset, limit, |component| {
                let mut result = fields!(component; group_index, touches_left_rail, touches_right_rail);
                result["edge_indices"] = page(&component.edge_indices, offset, limit, |index| json!(index));
                result["points"] = page(&component.points, offset, limit, point);
                result
            }),
            "open_branch_endpoints": page(&value.open_branch_endpoints, offset, limit, point),
            "strict_graph_available": value.open_branch_endpoints.is_empty(),
            "native_acceptance": "unverified",
        }),
    }
}

#[cfg(feature = "write")]
fn edit_sites(program: &LadderProgramData, offset: usize, limit: usize) -> Value {
    // Every public edit-site decoder is listed explicitly; None is distinguishable
    // from a recognized layout with no matching site.
    macro_rules! sites {
        ($($method:ident => [$($field:ident),*]),* $(,)?) => {
            json!({$(stringify!($method): optional_page(program.$method(), offset, limit,
                |site| fields!(site; $($field),*))),*})
        };
    }
    sites!(
        iec_terminal_function_deletion_sites => [group_index, row_index, block_offset, wire_offset, raw_x, pin_count],
        iec_scalar_chain_deletion_sites => [group_index, row_index, block_offset, raw_x, pin_count],
        iec_wired_comparison_insertion_sites => [group_index, row_index, raw_x],
        iec_terminal_timer_insertion_sites => [group_index, row_index, contact_offset, insertion_offset, raw_x],
        iec_terminal_function_insertion_sites => [group_index, row_index, contact_offset, insertion_offset, raw_x],
        iec_standalone_function_deletion_sites => [group_index, row_index, block_offset, wire_offset, raw_x, pin_count],
        iec_standalone_function_insertion_sites => [group_index, row_index, insertion_offset, raw_x],
        iec_connected_arithmetic_deletion_sites => [group_index, row_index, block_offset, wire_offset, raw_x],
        iec_function_cell_deletion_sites => [group_index, row_index, block_offset, raw_x, pin_count],
        iec_function_cell_insertion_sites => [function_name, group_index, row_index, insertion_offset, reference_offset, raw_x],
        iec_leading_contact_insertion_sites => [group_index, row_index, insertion_offset],
        iec_no_contact_insertion_sites => [group_index, row_index, wire_offset, start_x, end_x],
        iec_short_wire_contact_insertion_sites => [group_index, row_index, wire_offset, raw_x],
        iec_no_contact_deletion_sites => [group_index, row_index, contact_offset, raw_x, contact_code],
        iec_no_contact_cell_deletion_sites => [group_index, row_index, contact_offset, raw_x, contact_code],
        iec_horizontal_wire_repair_sites => [group_index, row_index, insertion_offset, raw_x],
        iec_horizontal_wire_deletion_sites => [group_index, row_index, wire_offset, raw_x],
    )
}

fn program_details(
    program: &LadderProgramData,
    section: &str,
    offset: usize,
    limit: usize,
) -> Value {
    let rows = || {
        optional_page(
            program.iec_row_frames(),
            offset,
            limit,
            |row| fields!(row; group_index, row_index, start, records_start, end, record_count),
        )
    };
    let records = || {
        optional_page(program.iec_record_frames(), offset, limit, |record| {
            positioned_record(program, record)
        })
    };
    let functions = || optional_page(program.iec_function_blocks(), offset, limit, function);
    let references = || {
        optional_page(program.iec_function_references(), offset, limit, |r| {
            fields!(r;
        group_index, row_index, record_offset, target_record_offset, target_row_index,
        raw_x, code, ordinal, is_output, pin_row_index, pin_raw_x, data_type_mask, is_array)
        })
    };
    let links = || {
        optional_page(program.iec_function_operand_links(), offset, limit, |r| {
            let mut value = fields!(r; group_index, row_index, record_offset, target_record_offset,
            ordinal, is_output, pin_row_index, pin_raw_x, data_type_mask, is_array);
            value["operand_string_offset"] = json!(r.record_offset + 15);
            value
        })
    };
    let strings = || source_strings(program, offset, limit);
    let cells = || {
        let cells = program
            .structure
            .rungs
            .iter()
            .flat_map(|r| &r.cells)
            .collect::<Vec<_>>();
        page(&cells, offset, limit, |value| cell(value))
    };
    match section {
        "program.rows" => rows(),
        "program.records" => records(),
        "program.functions" => functions(),
        "program.references" => references(),
        "program.operand_links" => links(),
        "program.graph" => graph(program.iec_circuit_layout(), offset, limit),
        "program.edit_sites" => edit_sites(program, offset, limit),
        "program.strings" => strings(),
        "program.cells" => cells(),
        _ => json!({
            "metadata": fields!(program; program_name, version, project_type, eno_control_option,
                compressed, encoded_len, decoded_len),
            "coordinates": if program.project_type == Some(2) {
                "IEC LD: row_index identifies stored rows, raw_y = row_index * 4, cell raw_x anchors are 1,4,7,...94, branch boundaries are 0,3,6,...96; group_index is revision-specific."
            } else {
                "XGK LD uses its native ten-column grid; preserve decoded raw_x/raw_y values."
            },
            "rungs": page(&program.structure.rungs, offset, limit, |r| json!({"raw_y": r.raw_y, "cell_count": r.cells.len()})),
            "cells": cells(), "strings": strings(),
            "labels": page(&program.labels(), offset, limit, |label| fields!(label; offset, raw_y, name)),
            "elements": page(&program.elements, offset, limit, |element| {
                let mut value = fields!(element; offset, value, operands);
                value["kind"] = json!(format!("{:?}", element.kind));
                value["contact"] = element.contact.map(|k| format!("{k:?}")).into();
                value["coil"] = element.coil.map(|k| format!("{k:?}")).into();
                value
            }),
            "instructions": page(&program.instructions, offset, limit, |i| fields!(i; offset, mnemonic, operands, raw)),
            "vertical_lines": page(&program.structure.vertical_lines, offset, limit, |v| fields!(v; raw_x, raw_y_start, raw_y_end)),
            "branch_groups": page(&program.structure.branch_groups, offset, limit, |v| fields!(v; raw_x, raw_y_start, raw_y_end)),
            "horizontal_lines": page(&program.structure.horizontal_lines, offset, limit, |v| fields!(v; raw_y, raw_x_start, raw_x_end)),
            "rung_comments": page(&program.structure.rung_comments, offset, limit, |c| fields!(c; offset, raw_x, raw_y, text)),
            "output_comments": page(&program.structure.output_comments, offset, limit, |c| fields!(c; offset, raw_x, raw_y, text)),
            "unknown_records": page(&program.structure.unknown_records, offset, limit, |u| {
                let mut value = fields!(u; offset, marker, raw_x, raw_y);
                value["byte_length"] = json!(u.bytes.len());
                value
            }),
            "iec_rows": rows(), "iec_records": records(), "iec_functions": functions(),
            "iec_function_references": references(), "iec_function_operand_links": links(),
            "iec_geometry": program.iec_geometry().map(|g| json!({
                "horizontal": page(&g.horizontal, offset, limit, |v| fields!(v; group_index, row_index, offset, start_x, end_x)),
                "vertical": page(&g.vertical, offset, limit, |v| fields!(v; group_index, start_row_index, end_row_index, start_offset, end_offset, x)),
            })),
            "iec_circuit_layout": graph(program.iec_circuit_layout(), offset, limit),
            "edit_sites": edit_sites(program, offset, limit),
            "native_acceptance": "unverified",
        }),
    }
}

fn text_program(value: &TextProgram, offset: usize, limit: usize) -> Value {
    let mut result = fields!(value; program_index, object_id, language, source, editable, reason, variables_error);
    result["variables"] = page(&value.variables, offset, limit, sfc_variable);
    result
}

fn sfc_row(value: &SfcRow) -> Value {
    json!(super::inspection_types::SfcRow::from(value.clone()))
}

fn sfc_variable(value: &SfcVariable) -> Value {
    json!(super::inspection_types::SfcVariable::from(value.clone()))
}

fn sfc_block(value: &SfcBlock, offset: usize, limit: usize) -> Value {
    json!({
        "block_index": value.block_index, "name": value.name, "main": value.main,
        "language_type": value.language_type, "language": value.language,
        "rows": value.rows, "columns": value.columns,
        "entities": page(&value.entities, offset, limit,
            |entity| json!(super::inspection_types::SfcEntity::from(entity.clone()))),
        "editable_rows": value.editable_rows.as_ref().map(|rows| page(rows, offset, limit, sfc_row)),
        "sequence_editing_supported": value.editable_rows.is_some(),
        "detail_section": format!("sfc.block.{}", value.block_index),
    })
}

fn sfc(doc: &XgwxDocument, program_index: Option<usize>, offset: usize, limit: usize) -> Value {
    let programs = doc
        .program()
        .sfc_programs()
        .into_iter()
        .filter(|p| program_index.is_none_or(|index| index == p.program_index))
        .collect::<Vec<_>>();
    page(
        &programs,
        if program_index.is_some() { 0 } else { offset },
        limit,
        |p| {
            json!({
                "program_index": p.program_index,
                "variables": page(&p.variables, offset, limit, sfc_variable),
                "variables_error": p.variables_error,
                "blocks": page(&p.blocks, offset, limit, |b| sfc_block(b, offset, limit)),
            })
        },
    )
}

/// Inspect supported views of one document. File identity/revision are supplied
/// by the filesystem layer, which reads the original bytes once per request.
pub fn inspect(
    doc: &XgwxDocument,
    section: &str,
    program_index: Option<usize>,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    if let Some(index) = program_index
        && index >= doc.program().programs().len()
    {
        return Err(format!("program_index {index} is absent"));
    }
    Ok(match section {
        "user_pous" => {
            if program_index.is_some() {
                return Err("user_pous uses its own index namespace; omit program_index".into());
            }
            json!({"can_create":doc.user_pou().can_create(), "definitions":page(&doc.user_pou().list(),offset,limit,|p|serde_json::to_value(super::user_pou::UserPou::from(p.clone())).unwrap())})
        }
        "overview" => {
            let project = doc.project().info();
            let mut info = fields!(&project; name, attribute, version, comment, wks_node_count,
                guid, file_version, file_last_write_time);
            info["attributes"] = attrs(&project.attributes);
            json!({
                "project": info,
                "header": fields!(&doc.container().header(); gzip_offset, signature, label, label_following_u32, compressed_size_hint),
                "container": {"xml_length": doc.container().xml().len(), "main_gzip_length": doc.container().main_gzip().len(),
                    "trailer_length": doc.container().trailer().len(), "trailer_gzip_members": doc.container().trailer_gzip_members().len()},
                "counts": {"programs": doc.program().programs().len(), "tasks": doc.program().tasks().len(),
                    "modules": doc.module().summaries().len(), "networks": doc.network().list().len(), "sfc_programs": doc.program().sfc_programs().len(), "user_pous":doc.user_pou().list().len(), "text_programs":doc.program().text_programs().len()},
                "configurations": page(&doc.configuration().list(), offset, limit, configuration),
                "programs": indexed_page(&doc.program().programs(), offset, limit, program_summary),
                "sections": SECTIONS,
                "pagination": "offset/limit apply to each list independently. Collect all pages before constructing expected SFC entity/row/variable lists.",
                "native_acceptance": "unverified",
            })
        }
        "programs" => json!({
            "programs": indexed_page(&doc.program().programs(), offset, limit, program_summary),
            "tasks": indexed_page(&doc.program().tasks(), offset, limit, |index, t| {
                let mut value = fields!(t; name, version, type_code, attribute, kind, priority,
                    task_index, device, device_type, word_value, word_condition, bit_condition);
                value["index"] = json!(index);
                value["attributes"] = attrs(&t.attributes);
                value
            }),
        }),
        "symbols" => symbols(doc, program_index, offset, limit),
        "hardware" => hardware(doc, offset, limit),
        section if section.starts_with("networks.opcua_tags.") => {
            let index = section
                .strip_prefix("networks.opcua_tags.")
                .unwrap()
                .parse::<usize>()
                .map_err(|_| {
                    "OPC UA tag section requires a numeric configuration_index".to_owned()
                })?;
            let tags = doc.network().opcua_tags(index).map_err(|e| e.to_string())?;
            json!({"configuration_index":index,"tags":page(&tags,offset,limit,|tag|fields!(tag;name,data_type,address_area,address_number,comment))})
        }
        "networks" => networks(doc, offset, limit),
        #[cfg(feature = "il")]
        "vendor_il" => {
            let programs = doc
                .program()
                .vendor_il_programs()
                .into_iter()
                .filter(|p| program_index.is_none_or(|i| i == p.program_index))
                .collect::<Vec<_>>();
            if program_index.is_some() && programs.is_empty() {
                return Err("selected program has no XGK IL projection".into());
            }
            page(
                &programs,
                if program_index.is_some() { 0 } else { offset },
                limit,
                |p| fields!(p; program_index,object_id,source,editable,reason),
            )
        }
        "text" => {
            let programs = doc
                .program()
                .text_programs()
                .into_iter()
                .filter(|p| program_index.is_none_or(|i| i == p.program_index))
                .collect::<Vec<_>>();
            if program_index.is_some() && programs.is_empty() {
                return Err("selected program is not standalone ST/IL".into());
            }
            page(
                &programs,
                if program_index.is_some() { 0 } else { offset },
                limit,
                |p| text_program(p, offset, limit),
            )
        }
        "text.variables" => {
            let index = program_index.ok_or("program_index is required for text.variables")?;
            let program = doc
                .program()
                .text_programs()
                .into_iter()
                .find(|p| p.program_index == index)
                .ok_or("selected program is not standalone ST/IL")?;
            text_program(&program, offset, limit)
        }
        "sfc" => sfc(doc, program_index, offset, limit),
        "sfc.variables" => {
            let index = program_index.ok_or("program_index is required for sfc.variables")?;
            let items = doc
                .variable()
                .sfc_variables(index)
                .map_err(|error| error.to_string())?;
            page(&items, offset, limit, sfc_variable)
        }
        section if section.starts_with("sfc.block.") => {
            let index = program_index.ok_or("program_index is required for SFC block details")?;
            let block_index = section
                .strip_prefix("sfc.block.")
                .unwrap()
                .parse::<usize>()
                .map_err(|_| {
                    "SFC block section must be sfc.block.<numeric block_index>".to_owned()
                })?;
            let selected = doc
                .program()
                .sfc_programs()
                .into_iter()
                .find(|p| p.program_index == index)
                .ok_or_else(|| format!("program_index {index} has no SFC chart"))?;
            let block = selected
                .blocks
                .iter()
                .find(|b| b.block_index == block_index)
                .ok_or_else(|| {
                    format!("SFC block_index {block_index} is absent in program {index}")
                })?;
            sfc_block(block, offset, limit)
        }
        "catalog.cpu" => page(cpu_catalog(), offset, limit, cpu),
        #[cfg(feature = "write")]
        "catalog.modules" => page(xgk_module_catalog(), offset, limit, module_catalog),
        "catalog.instructions" => page(ladder_instruction_catalog(), offset, limit, instruction),
        "catalog.comparisons" => page(ladder_comparison_catalog(), offset, limit, instruction),
        #[cfg(feature = "write")]
        "io_variables" => match doc.variable().preview_io_variables() {
            Ok(items) => page(&items, offset, limit, |v| {
                fields!(v; base, slot, model, name,
                address, address_number, data_type, description, source_ref, action, existing_names)
            }),
            Err(error) => json!({"status": "unsupported", "error": error.to_string()}),
        },
        section
            if SECTIONS.contains(&section)
                && (section == "program" || section.starts_with("program.")) =>
        {
            let index =
                program_index.ok_or("program_index is required for program detail sections")?;
            let summary = program_summary(index, &doc.program().programs()[index]);
            if let Some(program) = doc
                .program()
                .text_programs()
                .into_iter()
                .find(|p| p.program_index == index)
            {
                if section != "program" {
                    return Err("ladder detail sections do not apply to ST/IL; inspect text or text.variables".into());
                }
                return Ok(
                    json!({"program":summary,"details":text_program(&program,offset,limit)}),
                );
            }
            match doc.program().ladder_program(index) {
                Some(Ok(program)) => {
                    json!({"program": summary, "details": program_details(&program, section, offset, limit)})
                }
                Some(Err(error)) => {
                    json!({"program": summary, "status": "decode_error", "error": error.to_string(),
                    "note": "SFC programs are inspected using section sfc; unsupported binary layouts remain read-only."})
                }
                None => return Err(format!("program_index {index} is absent")),
            }
        }
        _ => {
            return Err(format!(
                "unknown section {section:?}; supported sections: {}",
                SECTIONS.join(", ")
            ));
        }
    })
}

fn cnet_config(value: &CnetConfigInfoSummary) -> Value {
    let mut result = fields!(value; station_no, type_code, base, slot, sub_type);
    result["attributes"] = attrs(&value.attributes);
    result["ports"] = indexed_page(&value.ports, 0, 1000, cnet_port);
    result
}

fn fenet_config(value: &FenetConfigInfoSummary) -> Value {
    let mut result = fields!(value; station_no, type_code, base, slot, sub_type,
                media, media_b, dhcp, driver_type, rcv_wait_time, client_wait_time, glofa_socket_count);
    result["attributes"] = attrs(&value.attributes);
    for (key, ip) in [
        ("ip_address", &value.ip_address),
        ("subnet", &value.subnet),
        ("gateway", &value.gateway),
        ("dns", &value.dns),
        ("ip_address2", &value.ip_address2),
        ("subnet2", &value.subnet2),
        ("gateway2", &value.gateway2),
        ("dns2", &value.dns2),
    ] {
        result[key] = ipv4(ip);
    }
    result
}

#[cfg(not(feature = "write"))]
fn edit_sites(_: &LadderProgramData, _: usize, _: usize) -> Value {
    json!({"status":"feature_unavailable","feature":"write"})
}
