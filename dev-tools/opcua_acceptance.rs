//! Generate a controlled EOPCT candidate and inspect native resaves.
use std::{env, error::Error, fs};
use xgwx::{OpcUaFieldEdit, OpcUaTag, OpcUaTagPatch, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("check") {
        let doc = XgwxDocument::from_path(&args[2])?;
        println!("{:?}", doc.network().opcua_config_infos());
        println!("{:?}", doc.network().opcua_tags(0)?);
        return Ok(());
    }
    let mut doc = xgwx::create_project("XGK-CPUSN", "LD")?;
    doc.base_mut().by_number(0)?.insert_module(0, "XGL-EOPCT")?;
    for (name, data_type, address_area, address_number) in [
        ("PumpReady", "BIT", "M", 170),
        ("TankLevel", "WORD", "D", 456),
        ("ByteValue", "BYTE", "M", 16),
        ("DwordValue", "DWORD", "D", 200),
        ("LwordValue", "LWORD", "D", 300),
        ("Signed8", "SINT", "M", 32),
        ("Unsigned8", "USINT", "M", 40),
        ("Signed16", "INT", "D", 400),
        ("Unsigned16", "UINT", "D", 401),
        ("Signed32", "DINT", "D", 402),
        ("Unsigned32", "UDINT", "D", 404),
        ("Signed64", "LINT", "D", 406),
        ("Unsigned64", "ULINT", "D", 410),
    ] {
        let expected_tags = doc.network().opcua_tags(0)?;
        doc.network_mut().edit_opcua_tag(&OpcUaTagPatch {
            configuration_index: 0,
            expected_tags,
            index: None,
            tag: Some(OpcUaTag {
                name: name.into(),
                data_type: data_type.into(),
                address_area: address_area.into(),
                address_number,
                comment: format!("{name} generated"),
            }),
        })?;
    }
    doc.network_mut()
        .module_at(0, 0)?
        .opcua()?
        .edit_settings(&[
            OpcUaFieldEdit {
                field: "port".into(),
                expected_value: "4840".into(),
                replacement: "4841".into(),
            },
            OpcUaFieldEdit {
                field: "publishingInterval".into(),
                expected_value: "200".into(),
                replacement: "100".into(),
            },
            OpcUaFieldEdit {
                field: "samplingInterval".into(),
                expected_value: "200".into(),
                replacement: "100".into(),
            },
            OpcUaFieldEdit {
                field: "nodeIdType".into(),
                expected_value: "0".into(),
                replacement: "1".into(),
            },
        ])?;
    fs::write(&args[1], doc.to_verified_bytes()?)?;
    Ok(())
}
