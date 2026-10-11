//! Inspect memberships and edit a selected FEnet module without repeating indices.
use xgwx::{NetworkFieldEdit, NetworkModulePatch, NetworkPatch, create_project};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = create_project("XGK-CPUSN", "LD")?;
    doc.base_mut()
        .by_number(0)?
        .insert_module(0, "XGL-EFMT(B)")?;
    for network in 0..doc.network().list().len() {
        let network = doc.network().get(network)?;
        println!("Network {}: {:?}", network.index(), network.summary().name);
        for module in network.modules() {
            println!("  Membership {}: {:?}", module.index(), module.typed());
        }
    }
    let mut network = doc.network_mut().get(0)?;
    network.update(&NetworkPatch {
        name: Some("Factory".into()),
        ..Default::default()
    })?;
    let mut module = network.module(0)?;
    module.update(&NetworkModulePatch {
        alias: Some("Uplink".into()),
        ..Default::default()
    })?;
    let mut fenet = module.fenet()?;
    fenet.edit_field(&NetworkFieldEdit {
        field: "ipAddress".into(),
        expected_value: "192.168.0.100".into(),
        replacement: "10.20.30.40".into(),
    })?;
    println!("Updated IP: {:?}", fenet.summary().ip_address);
    let bytes = doc.to_verified_bytes()?;
    xgwx::XgwxDocument::parse(&bytes)?;
    Ok(())
}
