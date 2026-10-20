//! Generate I/O variables and insert, update, inspect and delete global records.
use xgwx::{NewGlobalVariable, VariablePatch, create_project};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = create_project("XGK-CPUSN", "LD")?;
    doc.base_mut().by_number(0)?.insert_module(0, "XGI-D21A")?;
    doc.variable_mut().generate_io_variables()?;
    for variable in doc.variable().list()? {
        println!(
            "{}: {:?} at {:?}",
            variable.index(),
            variable.name(),
            variable.summary().address
        );
    }
    let captured = doc.variable().get(0)?.summary().clone();
    let mut variable = doc.variable_mut().get_if(0, &captured)?;
    variable.rename("StartInput")?;
    variable.update(&VariablePatch {
        description: Some("Factory start button".into()),
        ..Default::default()
    })?;
    println!("Updated: {:?}", variable.summary());
    let index = doc.variable_mut().insert(&NewGlobalVariable {
        name: "Ready".into(),
        data_type: "BIT".into(),
        address_area: "M".into(),
        address_number: 42,
        description: "Machine ready".into(),
    })?;
    let expected = doc.variable().get(index)?.summary().clone();
    doc.variable_mut().get_if(index, &expected)?.delete()?;
    let bytes = doc.to_verified_bytes()?;
    xgwx::XgwxDocument::parse(&bytes)?;
    Ok(())
}
