//! Generate clean IEC block acceptance projects with native instance declarations.
use std::{env, error::Error};
use xgwx::*;
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut d = XgwxDocument::from_path(&args[0])?;
    let empty = XgwxDocument::from_path(&args[1])?;
    let seed = roxmltree::Document::parse(empty.container().xml())?;
    let data = seed
        .descendants()
        .find(|n| n.has_tag_name("ProgramData"))
        .unwrap();
    let xml = d.container().xml().to_string();
    let parsed = roxmltree::Document::parse(&xml)?;
    let target = parsed
        .descendants()
        .find(|n| n.has_tag_name("ProgramData"))
        .unwrap();
    let mut xml = xml.clone();
    xml.replace_range(target.range(), &empty.container().xml()[data.range()]);
    d.container_mut().replace_xml(xml)?;
    d.write_to(&args[2])?;
    for (i, (name, values)) in [
        ("R_TRIG", vec!["Rise"]),
        ("F_TRIG", vec!["Fall"]),
        ("TON", vec!["OnTimer", "T#1s"]),
        ("TOF", vec!["OffTimer", "T#1s"]),
        ("TP", vec!["PulseTimer", "T#1s"]),
        ("RS", vec!["ResetLatch", "FALSE"]),
        ("SR", vec!["SetLatch", "FALSE"]),
        ("CTU_DINT", vec!["UpCounter", "FALSE", "DINT#10"]),
        ("CTD_DINT", vec!["DownCounter", "FALSE", "DINT#10"]),
        (
            "CTUD_DINT",
            vec!["BothCounter", "FALSE", "FALSE", "FALSE", "DINT#10"],
        ),
        ("NE", vec!["INT#1", "INT#2"]),
        ("MOD", vec!["INT#5", "INT#2", "%MW100"]),
        ("AND", vec!["TRUE", "FALSE", "%MX100"]),
        ("OR", vec!["TRUE", "FALSE", "%MX101"]),
        ("XOR", vec!["TRUE", "FALSE", "%MX102"]),
        ("NOT", vec!["TRUE", "%MX103"]),
        ("WORD_TO_UDINT", vec!["%MW200", "%MD100"]),
    ]
    .into_iter()
    .enumerate()
    {
        let row = i as u16 * 10;
        let x = if i % 2 == 0 { 4 } else { 16 };
        if i % 2 != 0 {
            d.program_mut()
                .get(0)?
                .iec_ld()?
                .insert_single_element(row, 1, "contact", "NO", "%MX0")?;
        }
        d.program_mut().get(0)?.iec_ld()?.insert_function(
            row,
            x,
            name,
            &values.into_iter().map(String::from).collect::<Vec<_>>(),
        )?;
    }
    d.write_to(&args[3])?;
    Ok(())
}
