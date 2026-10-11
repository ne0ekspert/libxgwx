//! Inspect link blocks, safety channels and trend sections with explicit PLC ownership.
use xgwx::XgwxDocument;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example services_handles -- project.xgwx")?;
    let doc = XgwxDocument::from_path(path)?;
    for link in doc.high_speed_link().list() {
        println!(
            "Link {}: PLC {:?}, native selector {:?}",
            link.index(),
            link.configuration_index(),
            link.summary().index
        );
        for block in link.blocks() {
            println!("  Block {}: {:?}", block.index(), block.summary());
        }
    }
    for safety in doc.safety().list() {
        println!(
            "Safety {}: PLC {:?}, {} channels",
            safety.index(),
            safety.configuration_index(),
            safety.channels().len()
        );
    }
    for trend in doc.trend().list() {
        println!(
            "Trend {}: PLC {:?}, {:?}",
            trend.index(),
            trend.configuration_index(),
            trend.sections()
        );
    }
    for index in 0..doc.configuration().list().len() {
        let plc = doc.configuration().get(index)?;
        println!(
            "PLC {index}: {} links, {} safety records, {} trend records",
            plc.high_speed_link().list().len(),
            plc.safety().list().len(),
            plc.trend().list().len()
        );
    }
    Ok(())
}
