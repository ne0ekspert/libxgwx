//! Run: cargo run --example hardware_handles --features write
use std::error::Error;
use xgwx::{ModuleFieldEdit, create_project};
fn main() -> Result<(), Box<dyn Error>> {
    let mut doc = create_project("XGK-CPUSN", "LD")?;
    let mut base = doc.base_mut().by_number(0)?;
    base.set_slot_count(4)?;
    base.insert_module(0, "XGI-D24A/B")?;
    base.insert_module(1, "XGL-EFMT(B)")?;
    let mut module = base.module(0)?;
    module.set_input_filter(xgwx::ModuleInputFilter::Ms5)?;
    module.edit_field(&ModuleFieldEdit {
        operation: "comment".into(),
        expected_value: module.summary().comment.unwrap_or_default(),
        replacement: "Digital inputs".into(),
        key: None,
        index: None,
    })?;
    println!("{:?}", module.summary());
    drop(module);
    for base in doc.bases() {
        println!(
            "Base {:?}, configuration {:?}",
            base.number(),
            base.configuration_index()
        );
        for module in base.installed_modules() {
            println!("record {}: {:?}", module.index(), module.typed());
        }
    }
    // A selected editor owns the document borrow and can be stored directly.
    doc.module_mut().at(0, 1)?.delete()?;
    let bytes = doc.to_verified_bytes()?;
    let reopened = xgwx::XgwxDocument::parse(&bytes)?;
    assert_eq!(reopened.module().summaries(), doc.module().summaries());
    Ok(())
}
