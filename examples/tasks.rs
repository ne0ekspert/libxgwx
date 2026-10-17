//! Create task-driven programs and emit workspaces for native XG5000 inspection.
use std::{fs::OpenOptions, io::Write, path::PathBuf};
use xgwx::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: cargo run --features write --example tasks -- NEW_OUTPUT_DIRECTORY")?,
    );
    std::fs::create_dir(&output)?;
    for (prefix, model) in [("XGK", "XGK-CPUSN"), ("XGI", "XGI-CPUUN")] {
        let mut doc = create_project(model, "LD")?;
        // XG5000 Check Program requires at least one scan program. Keep a
        // separate scan program when moving the initial program into a task.
        doc.program_mut().create(&NewProgram {
            name: "ScanLoop".into(),
            language: "LD".into(),
            object_id: "33333333-3333-4333-8333-333333333333".into(),
            symbol_id: "44444444-4444-4444-8444-444444444444".into(),
        })?;
        let settings = TaskSettings {
            name: "FastLoop".into(),
            priority: 2,
            task_number: 0,
            trigger: TaskTrigger::Periodic { interval_ms: 100 },
        };
        doc.task_mut().create(0, &settings)?;
        let original = doc.program().get(0)?.summary();
        doc.task_mut().get(1)?.assign_program(
            0,
            original.object_id.as_deref().unwrap(),
            original.task.as_deref().unwrap(),
        )?;
        // Rename updates the native Program Task attribute in the same transaction.
        doc.task_mut().get(1)?.rename("ControlLoop")?;
        let mut settings = doc.task().get(1)?.settings()?;
        settings.priority = 7;
        settings.trigger = TaskTrigger::Periodic { interval_ms: 250 };
        doc.task_mut().get(1)?.update(&settings)?;
        doc.task_mut().create(
            0,
            &TaskSettings {
                name: "Initialize".into(),
                priority: 2,
                task_number: 0,
                trigger: TaskTrigger::Initialization,
            },
        )?;
        let bits = [
            TaskBitCondition::Rising,
            TaskBitCondition::Falling,
            TaskBitCondition::Changed,
            TaskBitCondition::On,
            TaskBitCondition::Off,
        ];
        let words = [
            TaskWordCondition::Less,
            TaskWordCondition::LessOrEqual,
            TaskWordCondition::Equal,
            TaskWordCondition::GreaterOrEqual,
            TaskWordCondition::Greater,
        ];
        for i in 0..10 {
            let trigger = if i < 5 {
                TaskTrigger::InternalBit {
                    device: if prefix == "XGI" { "%MX0" } else { "M00000" }.into(),
                    condition: bits[i],
                }
            } else {
                TaskTrigger::InternalWord {
                    device: if prefix == "XGI" { "%MW0" } else { "D000000" }.into(),
                    condition: words[i - 5],
                    value: 65535,
                }
            };
            doc.task_mut().create(
                0,
                &TaskSettings {
                    name: format!("Trigger{i}"),
                    priority: 2 + i as u32 % 6,
                    task_number: 64 + i as u32,
                    trigger,
                },
            )?;
            let index = doc.task().list().len() - 1;
            doc.task_mut().get(index)?.create_program(&NewProgram {
                name: format!("Worker{i}"),
                language: "LD".into(),
                object_id: format!("11111111-1111-4111-8111-{i:012x}"),
                symbol_id: format!("22222222-2222-4222-8222-{i:012x}"),
            })?;
        }
        // Delete an empty task without removing any program implicitly.
        doc.task_mut().create(
            0,
            &TaskSettings {
                name: "Temporary".into(),
                task_number: 31,
                ..settings.clone()
            },
        )?;
        let last = doc.task().list().len() - 1;
        doc.task_mut().get(last)?.delete()?;
        // Give the acceptance example real, distinct memory-only rungs rather
        // than empty programs, which native Check Program reports as errors.
        for index in 0..doc.program().list().len() {
            let mut program = doc.program_mut().get(index)?;
            if prefix == "XGI" {
                program
                    .iec_ld()?
                    .insert_linear_rung(0, "%MX0", &format!("%MX{}", index + 1))?;
            } else {
                let mut ladder = program.ladder()?;
                for (column, kind, operand) in [
                    (0, LadderEditKind::NormallyOpen, "M00000".to_owned()),
                    (9, LadderEditKind::Output, format!("M{:05}", index + 1)),
                ] {
                    ladder.edit_cell(&LadderCellEdit {
                        raw_y: 0,
                        column,
                        expected: None,
                        replacement: Some(LadderEditElement { kind, operand }),
                    })?;
                }
            }
        }
        let path = output.join(format!("{prefix}.xgwx"));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .write_all(&doc.to_verified_bytes()?)?;
        println!(
            "{}: {} tasks, {} programs",
            path.display(),
            doc.task().list().len(),
            doc.program().list().len()
        );
    }
    Ok(())
}
