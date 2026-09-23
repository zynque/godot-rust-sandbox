use godot::classes::Engine;
use godot::prelude::*;

use crate::shader_test_common::ComputeHarness;

// ---------------------------------------------------------------------------
// Shared plumbing for tests that run a compute shader over parcel intervals.
//
// The shaders all consume a flat float payload and produce a `ParcelIntervals`
// list, whose output layout is described by `write_intervals()` in
// test_harness.glslinc.
// ---------------------------------------------------------------------------

/// Mirrors MAX_PARCEL_INTERVALS in constants.glslinc.
pub const MAX_PARCEL_INTERVALS: usize = 100;
/// Entry/exit values come back from GPU float math, so they are compared with
/// a tolerance. Parcel indices are integers and must match exactly.
pub const EPSILON: f32 = 1e-4;

/// One (entry, exit, parcel_index) interval.
pub type Interval = (f32, f32, i32);

/// A case for a compute shader that turns a flat float payload into a
/// `ParcelIntervals` list.
pub trait IntervalCase {
    fn label(&self) -> &'static str;
    /// Flat float payload for binding 0, or an error if the case cannot be
    /// represented by the shader.
    fn encode_input(&self) -> Result<Vec<f32>, String>;
    fn expected(&self) -> &'static [Interval];
}

/// Runs every case against `shader_path` and prints a PASS/FAIL/ERROR report.
///
/// Does nothing outside the editor, where the tests are driven from a scene's
/// tool button.
pub fn run_interval_tests<C: IntervalCase>(context: &str, shader_path: &str, cases: &[C]) {
    if !Engine::singleton().is_editor_hint() {
        return;
    }

    let Some(mut harness) = ComputeHarness::new(context, shader_path) else {
        return;
    };

    let mut passed = 0usize;
    let mut failed = 0usize;

    for case in cases {
        match run_case(&mut harness, case) {
            Ok(intervals) if intervals_match(&intervals, case.expected()) => {
                godot_print!("[{}] PASS  '{}'", context, case.label());
                passed += 1;
            }
            Ok(intervals) => {
                godot_print!(
                    "[{}] FAIL  '{}'\n  expected: {:?}\n  got:      {:?}",
                    context,
                    case.label(),
                    case.expected(),
                    intervals
                );
                failed += 1;
            }
            Err(msg) => {
                godot_print!("[{}] ERROR '{}': {}", context, case.label(), msg);
                failed += 1;
            }
        }
    }

    godot_print!(
        "[{}] Results: {}/{} passed.",
        context,
        passed,
        passed + failed
    );
}

/// Declares the tool node for an interval test: a `#[class]` Node whose
/// `run_tests` method drives `run_interval_tests`.
macro_rules! interval_test_node {
    ($name:ident, $context:expr, $shader_path:expr, $cases:expr) => {
        #[derive(GodotClass)]
        #[class(tool, base=Node)]
        pub struct $name {
            base: Base<Node>,
        }

        #[godot_api]
        impl INode for $name {
            fn init(base: Base<Node>) -> Self {
                Self { base }
            }
        }

        #[godot_api]
        impl $name {
            #[func]
            fn run_tests(&mut self) {
                $crate::parcel_test_common::run_interval_tests($context, $shader_path, $cases);
            }
        }
    };
}
pub(crate) use interval_test_node;

fn run_case<C: IntervalCase>(
    harness: &mut ComputeHarness,
    case: &C,
) -> Result<Vec<Interval>, String> {
    let values = harness.dispatch(&case.encode_input()?, interval_output_len())?;
    decode_intervals(&values)
}

/// Floats the shaders write to binding 1: a count followed by entry/exit/index.
fn interval_output_len() -> usize {
    1 + 3 * MAX_PARCEL_INTERVALS
}

fn decode_intervals(values: &[f32]) -> Result<Vec<Interval>, String> {
    let count = values[0] as usize;
    if count > MAX_PARCEL_INTERVALS {
        return Err(format!(
            "shader reported {} intervals, expected at most {}",
            count, MAX_PARCEL_INTERVALS
        ));
    }

    let mut intervals = Vec::with_capacity(count);
    for i in 0..count {
        let base = 1 + 3 * i;
        intervals.push((values[base], values[base + 1], values[base + 2] as i32));
    }

    Ok(intervals)
}

fn intervals_match(actual: &[Interval], expected: &[Interval]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(a, e)| {
            (a.0 - e.0).abs() <= EPSILON && (a.1 - e.1).abs() <= EPSILON && a.2 == e.2
        })
}
