//! Enumerate installed hardware and network modules with resolved settings.
//! Run: `cargo run --example modules -- project.xgwx` (no optional features).
use std::error::Error;
use xgwx::{ConfigurationMatch, Module, NetworkConfiguration, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: modules <file.xgwx>")?;
    let doc = XgwxDocument::from_path(path)?;
    for base in doc.bases() {
        println!(
            "Base #{}, number {:?}, PLC {:?}, slots {:?}",
            base.index(),
            base.number(),
            base.configuration_index(),
            base.slot_count()
        );
        for module in base.modules() {
            println!("  {module:#?}");
        }
    }
    println!(
        "Cnet configuration records: {}",
        doc.network().cnet_config_infos().len()
    );
    println!(
        "FEnet configuration records: {}",
        doc.network().fenet_config_infos().len()
    );
    for module in doc.modules() {
        match module {
            Module::Io(io) => println!("I/O {:?}: {:#?}", io.kind, io.base),
            Module::Other(hardware) => println!("Other hardware: {hardware:#?}"),
            Module::Network(network) => {
                println!("Network: {:#?}", network.metadata);
                println!("Installed hardware: {:#?}", network.base);
                if !network.hardware_candidates.is_empty() {
                    println!(
                        "Unresolved hardware candidates: {:#?}",
                        network.hardware_candidates
                    );
                }
                match network.configuration {
                    ConfigurationMatch::Matched(NetworkConfiguration::Fenet(config)) => {
                        println!("FEnet settings: {config:#?}");
                    }
                    ConfigurationMatch::Matched(NetworkConfiguration::Cnet(config)) => {
                        println!("Cnet settings, including ports: {config:#?}");
                    }
                    ConfigurationMatch::Matched(NetworkConfiguration::Other(config)) => {
                        println!("Other protocol settings: {config:#?}");
                    }
                    ConfigurationMatch::Missing => println!("No matching settings"),
                    ConfigurationMatch::Ambiguous(candidates) => {
                        println!("Ambiguous configuration association: {candidates:#?}");
                    }
                }
            }
        }
    }
    Ok(())
}
