//! Generate and compare representative XGB native parameter edits.
use std::{error::Error, path::Path};
use xgwx::{NativeParameterChange, NativeParameterTarget as T, ParameterKind as K, XgwxDocument};
fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("generate") => {
            let output = Path::new(args.get(2).ok_or("output directory")?);
            if output.join("baseline.xgwx").exists() || output.join("edited.xgwx").exists() {
                return Err("refusing to overwrite an existing acceptance workspace".into());
            }
            std::fs::create_dir_all(output)?;
            let source = std::fs::read("fixtures/XGB_Enet01.xgwx")?;
            let mut doc = XgwxDocument::parse(&source)?;
            // Representative valid values; acceptance does not exercise arbitrary
            // integers across the raw reader domains.
            let cases = [
                (K::Hsc, T::HscChannel { channel: 0 }, "unit_time_ms", 2345),
                (
                    K::Hsc,
                    T::HscChannel { channel: 0 },
                    "pulses_per_revolution",
                    100,
                ),
                (
                    K::PidCalculation,
                    T::PidCalculationLoop { loop_index: 0 },
                    "TargetValue",
                    123,
                ),
                (
                    K::PidCalculation,
                    T::PidCalculationLoop { loop_index: 0 },
                    "ScanTime",
                    200,
                ),
                (
                    K::PidTuning,
                    T::PidTuningLoop { loop_index: 1 },
                    "TargetValue",
                    234,
                ),
                (
                    K::Position,
                    T::PositionStep {
                        axis_index: 0,
                        step_index: 0,
                    },
                    "TargetPosition",
                    12345,
                ),
                (
                    K::Position,
                    T::PositionStep {
                        axis_index: 1,
                        step_index: 29,
                    },
                    "DwellTime",
                    100,
                ),
            ];
            for (kind, target, name, replacement) in cases {
                let record = doc.parameter().of_kind(kind)[0];
                let index = record.index();
                let field = record
                    .native_fields()
                    .into_iter()
                    .find(|f| f.target == target && f.name == name)
                    .ok_or("missing field")?;
                doc.parameter_mut()
                    .get(index)?
                    .edit_native(&[NativeParameterChange {
                        target,
                        name: name.into(),
                        expected: field.value,
                        replacement,
                    }])?;
            }
            std::fs::write(output.join("baseline.xgwx"), source)?;
            std::fs::write(output.join("edited.xgwx"), doc.to_verified_bytes()?)?;
            println!("generated baseline.xgwx and edited.xgwx");
        }
        Some("check") => {
            let a = XgwxDocument::from_path(args.get(2).ok_or("generated path")?)?;
            let b = XgwxDocument::from_path(args.get(3).ok_or("resaved path")?)?;
            for kind in [K::Hsc, K::PidCalculation, K::PidTuning, K::Position] {
                let original = a.parameter().of_kind(kind)[0];
                let resaved = b.parameter().of_kind(kind)[0];
                let fields = original.native_fields();
                let actual = resaved.native_fields();
                assert!(!actual.is_empty(), "unsupported resaved {kind:?}");
                assert_eq!(fields, actual, "native values changed for {kind:?}");
                println!("{kind:?}: {} fields retained", fields.len());
            }
        }
        _ => return Err("generate OUTDIR | check GENERATED RESAVED".into()),
    }
    Ok(())
}
