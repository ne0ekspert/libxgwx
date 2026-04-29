use std::env;
use std::error::Error;
use std::io;

use libxgwx::{CnetPortConfigSummary, Ipv4Summary, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo run --example inspect -- <file.xgwx>",
        )
    })?;
    let doc = XgwxDocument::from_path(&path)?;
    let project = doc.project_info();

    println!("file: {path}");
    println!(
        "project: {}",
        project.name.as_deref().unwrap_or("<unnamed>")
    );
    println!(
        "version: {}",
        project.file_version.as_deref().unwrap_or("<unknown>")
    );
    println!("configurations: {}", doc.configurations().len());
    println!("networks: {}", doc.networks().len());
    println!("modules: {}", doc.modules().len());
    println!("programs: {}", doc.programs().len());
    println!("variables: {}", doc.variables().map(|items| items.len())?);
    println!("payloads: {}", doc.decoded_payloads().len());

    print_cnet(&doc);
    print_fenet(&doc);
    print_hsc(&doc)?;
    print_position(&doc);
    print_pid(&doc);

    Ok(())
}

fn print_cnet(doc: &XgwxDocument) {
    let cnet = doc.cnet_config_infos();
    println!("cnet modules: {}", cnet.len());
    for config in cnet {
        println!(
            "  type={} subtype={} station={} ports={}",
            option_u32(config.type_code),
            option_u32(config.sub_type),
            option_u32(config.station_no),
            config.ports.len(),
        );
        for (index, port) in config.ports.iter().enumerate() {
            println!("    port {} {}", index + 1, cnet_port(port));
        }
    }
}

fn print_fenet(doc: &XgwxDocument) {
    let fenet = doc.fenet_config_infos();
    println!("fenet modules: {}", fenet.len());
    for config in fenet {
        println!(
            "  type={} subtype={} station={} ip={} subnet={} gateway={} dns={}",
            option_u32(config.type_code),
            option_u32(config.sub_type),
            option_u32(config.station_no),
            ipv4(config.ip_address.as_ref()),
            ipv4(config.subnet.as_ref()),
            ipv4(config.gateway.as_ref()),
            ipv4(config.dns.as_ref()),
        );
    }
}

fn print_hsc(doc: &XgwxDocument) -> Result<(), Box<dyn Error>> {
    let hsc = doc
        .hsc_parameters()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    println!("hsc parameters: {}", hsc.len());
    for parameter in hsc {
        println!(
            "  payload_bytes={} channels={}",
            parameter.payload_bytes.len(),
            parameter.channels.len()
        );
        for channel in parameter.channels {
            println!(
                "    ch{} counter={} pulse={} compare={} ring_max={} min={} max={} unit_ms={} ppr={}",
                channel.channel,
                channel
                    .counter_mode
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "<none>".to_owned()),
                channel
                    .pulse_input_mode
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "<none>".to_owned()),
                channel
                    .compare_output_mode
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "<none>".to_owned()),
                option_i32(channel.ring_counter_max),
                option_i32(channel.compare_output_min),
                option_i32(channel.compare_output_max),
                option_u16(channel.unit_time_ms),
                option_u16(channel.pulses_per_revolution),
            );
        }
    }
    Ok(())
}

fn print_position(doc: &XgwxDocument) {
    let position = doc.position_parameters();
    println!("position parameters: {}", position.len());
    for parameter in position {
        println!(
            "  axis_count={} axes={}",
            option_u32(parameter.axis_count),
            parameter.axes.len()
        );
        for axis in parameter.axes {
            println!(
                "    {} axis steps={} parsed={}",
                axis.axis_name,
                option_u32(axis.step_count),
                axis.steps.len()
            );
        }
    }
}

fn print_pid(doc: &XgwxDocument) {
    let cal = doc.pid_cal_parameters();
    let tune = doc.pid_tune_parameters();
    println!("pid cal parameters: {}", cal.len());
    for parameter in cal {
        println!("  loops={}", parameter.loops.len());
    }
    println!("pid tune parameters: {}", tune.len());
    for parameter in tune {
        println!("  loops={}", parameter.loops.len());
    }
}

fn cnet_port(port: &CnetPortConfigSummary) -> String {
    format!(
        "station={} mode={} baud={} data={} stop={} parity={} di={} do={} ai={} ao={}",
        option_u32(port.station_no),
        port.mode_kind
            .map(|value| value.label())
            .unwrap_or("<unknown>"),
        option_u32(port.baud_rate),
        port.data_bits
            .map(|value| value.label())
            .unwrap_or("<unknown>"),
        port.stop_bits
            .map(|value| value.label())
            .unwrap_or("<unknown>"),
        port.parity_mode
            .map(|value| value.label())
            .unwrap_or("<unknown>"),
        option_str(port.di_address.as_deref()),
        option_str(port.do_address.as_deref()),
        option_str(port.ai_address.as_deref()),
        option_str(port.ao_address.as_deref()),
    )
}

fn ipv4(value: Option<&Ipv4Summary>) -> &str {
    value
        .map(|value| value.address.as_str())
        .unwrap_or("<none>")
}

fn option_str(value: Option<&str>) -> &str {
    value.unwrap_or("<none>")
}

fn option_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<none>".to_owned())
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
