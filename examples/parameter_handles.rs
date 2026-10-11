//! Inspect selected parameter sections and their PLC ownership.
use xgwx::{ParameterData, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example parameter_handles -- project.xgwx")?;
    let doc = XgwxDocument::from_path(path)?;
    for record in doc.parameter().list() {
        println!(
            "Parameter {}: {:?}, PLC {:?}",
            record.index(),
            record.type_name(),
            record.configuration_index()
        );
        match record.decode() {
            Ok(ParameterData::Hsc(hsc)) => println!(
                "  {} counter channels, {} raw bytes",
                hsc.channels.len(),
                hsc.payload_bytes.len()
            ),
            Ok(ParameterData::Position(position)) => {
                for axis in position.axes {
                    println!("  Axis {}: {} steps", axis.axis_name, axis.steps.len());
                }
            }
            Ok(ParameterData::PidCalculation(pid)) => {
                println!("  {} PID calculation loops", pid.loops.len())
            }
            Ok(ParameterData::PidTuning(pid)) => println!("  {} PID tuning loops", pid.loops.len()),
            Ok(ParameterData::Other(summary)) => {
                println!("  {} preserved child sections", summary.sections.len())
            }
            Err(error) => println!("  Decode error: {error}"),
        }
    }
    for index in 0..doc.configuration().list().len() {
        let parameters = doc.configuration().get(index)?.parameters();
        println!(
            "PLC {index}: {} parameters, {} PID calculation tables, {} PID tuning tables",
            parameters.list().len(),
            parameters.pid().calculation().len(),
            parameters.pid().tuning().len()
        );
    }
    Ok(())
}
