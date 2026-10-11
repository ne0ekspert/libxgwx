//! Inspect project options and configuration ownership, then rename and change CPU.
use xgwx::create_project;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = create_project("XGK-CPUSN", "LD")?;
    println!("Project: {:?}", doc.project().info());
    if let Some(options) = doc.project().options() {
        for entry in options.entries {
            println!("Option {} = {}", entry.key, entry.value);
        }
    }
    for index in 0..doc.configuration().list().len() {
        let configuration = doc.configuration().get(index)?;
        println!(
            "Configuration {}: {:?}, CPU {:?}, {} bases",
            configuration.index(),
            configuration.summary().name,
            configuration.cpu().map(|cpu| cpu.model),
            configuration.bases().len()
        );
    }
    doc.project_mut().rename("Factory & Line 1")?;
    doc.configuration_mut().get(0)?.select_cpu("XGK-CPUUN")?;
    println!(
        "After: {:?}, {:?}",
        doc.project().name(),
        doc.configuration().get(0)?.cpu().map(|cpu| cpu.model)
    );
    let bytes = doc.to_verified_bytes()?;
    xgwx::XgwxDocument::parse(&bytes)?;
    Ok(())
}
