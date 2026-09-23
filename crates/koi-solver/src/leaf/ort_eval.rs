//! ONNX Runtime leaf backend (`leaf-ort` feature).
//!
//! `load-dynamic` resolves the runtime at load time rather than link time:
//! set `ORT_DYLIB_PATH` to the `onnxruntime` shared library, or place
//! `onnxruntime.dll`/`libonnxruntime.so` on the loader path. One `Session`
//! per evaluator; sessions are created once per engine and reused — the
//! resolver never reloads a model mid-search.
//!
//! Ported from a sibling research codebase's `leaf/ort_eval.rs` unchanged except the
//! `KOI_LEAF_FIXTURE_DIR` env var and the crate's own feature/dims — the
//! dylib probe ordering is what keeps a bad runtime from aborting the
//! process, so it is copied verbatim.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ndarray::{aview1, Array2};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};

use super::{
    actions::MAX_ACTIONS,
    eval::{LeafEval, LeafEvalError, LeafEvaluator, LeafQuery},
    features::FEATURE_DIM,
};

/// The platform dylib name `ort` falls back to when `ORT_DYLIB_PATH` is
/// unset — mirrors `ort`'s own load-dynamic lookup.
fn default_dylib_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "onnxruntime.dll"
    }
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    {
        "libonnxruntime.so"
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        "libonnxruntime.dylib"
    }
}

/// Validates that `path` is an ONNX Runtime binary exposing an API version
/// `ort` can bind, using the same checks `ort`'s `load-dynamic` init
/// performs. This must run before any `ort` call: `ort` resolves its dylib
/// lazily and *panics* on a missing or incompatible library, the panic
/// poisons a global mutex, and the poisoned lock then aborts the process
/// during shutdown. A clean `Err` here is the difference between safe
/// rollout fallback and a fatal crash.
#[allow(unsafe_code)] // dylib probing is inherently FFI: load → symbol → call
fn probe_dylib(path: &Path) -> Result<(), LeafEvalError> {
    let unavailable = |message: String| LeafEvalError::RuntimeUnavailable(format!("{}: {message}", path.display()));
    let lib = unsafe { libloading::Library::new(path) }.map_err(|err| unavailable(err.to_string()))?;
    let version = {
        let base_getter: libloading::Symbol<unsafe extern "C" fn() -> *const ort::sys::OrtApiBase> =
            unsafe { lib.get(b"OrtGetApiBase") }
                .map_err(|_| unavailable("does not export `OrtGetApiBase`".to_string()))?;
        let base = unsafe { base_getter() };
        if base.is_null() {
            return Err(unavailable("`OrtGetApiBase` returned null".to_string()));
        }
        unsafe { std::ffi::CStr::from_ptr(((*base).GetVersionString)()) }
            .to_string_lossy()
            .into_owned()
    };
    let minor = version
        .split('.')
        .nth(1)
        .and_then(|field| field.parse::<u32>().ok())
        .unwrap_or(0);
    if minor < ort::MINOR_VERSION {
        return Err(unavailable(format!(
            "version '{version}' is older than the required '1.{}.x'",
            ort::MINOR_VERSION
        )));
    }
    Ok(())
}

/// Initializes the ort environment once. `ORT_DYLIB_PATH` (checked first,
/// then the loader path) locates the shared library; the chosen binary is
/// probed for API compatibility before `ort` is allowed to load it, so a
/// bad runtime degrades to `RuntimeUnavailable` instead of a process abort.
fn ensure_environment() -> Result<(), LeafEvalError> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();
    let result = INIT.get_or_init(|| {
        let path = match std::env::var("ORT_DYLIB_PATH") {
            Ok(value) if !value.is_empty() => PathBuf::from(value),
            _ => PathBuf::from(default_dylib_name()),
        };
        if let Err(err) = probe_dylib(&path) {
            return Err(format!("{err}; set ORT_DYLIB_PATH to a compatible onnxruntime dylib"));
        }
        match ort::init_from(path) {
            Ok(builder) => {
                if builder.commit() {
                    Ok(())
                } else {
                    Err("ORT environment commit returned false".to_string())
                }
            }
            Err(err) => Err(err.to_string()),
        }
    });
    result.clone().map_err(LeafEvalError::RuntimeUnavailable)
}

/// A CPU ONNX Runtime session over the exported leaf graph. Must satisfy
/// `Send` (the `LeafEvaluator` bound) because the engine shares one evaluator
/// behind a `Mutex` across resolver calls.
pub struct OrtLeafEvaluator {
    session: Session,
}

impl OrtLeafEvaluator {
    /// `model_path` is the exported `model.onnx`; the graph already
    /// contains scalar normalization, so no sidecar is consulted.
    pub fn new(model_path: impl AsRef<Path>, intra_threads: usize) -> Result<Self, LeafEvalError> {
        ensure_environment()?;
        let model_path = model_path.as_ref();
        let mut builder = Session::builder()
            .map_err(|err| LeafEvalError::RuntimeUnavailable(err.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|err| LeafEvalError::RuntimeUnavailable(err.to_string()))?;
        // CUDA → CPU selection: with the
        // `leaf-ort-cuda` feature the CUDA EP is registered fail-silently —
        // a CPU-only ORT binary or a host without a GPU simply falls back to
        // the CPU provider. Without the feature the session is CPU-only.
        #[cfg(feature = "leaf-ort-cuda")]
        {
            builder = builder
                .with_execution_providers([ort::ep::CUDA::default().build()])
                .map_err(|err| LeafEvalError::RuntimeUnavailable(err.to_string()))?;
        }
        builder = builder
            .with_intra_threads(intra_threads)
            .map_err(|err| LeafEvalError::RuntimeUnavailable(err.to_string()))?;
        builder = builder
            .with_inter_threads(1)
            .map_err(|err| LeafEvalError::RuntimeUnavailable(err.to_string()))?;
        let session = builder
            .commit_from_file(model_path)
            .map_err(|err| LeafEvalError::ModelLoad(format!("{}: {err}", model_path.display())))?;
        // Fail the model contract at load time: a graph that loads but lacks
        // the `value`/`policy` heads must be a ModelLoad error, never a panic
        // inside `run` (the release cdylib aborts on panic).
        for required in ["value", "policy"] {
            if !session.outputs().iter().any(|outlet| outlet.name() == required) {
                return Err(LeafEvalError::ModelLoad(format!(
                    "{}: graph has no `{required}` output",
                    model_path.display()
                )));
            }
        }
        Ok(Self { session })
    }

    fn run(&mut self, batch: &[LeafQuery]) -> Result<Vec<LeafEval>, LeafEvalError> {
        let n = batch.len();
        let mut features = Array2::<f32>::zeros((n, FEATURE_DIM));
        let mut mask = Array2::<f32>::zeros((n, MAX_ACTIONS));
        for (row, query) in batch.iter().enumerate() {
            features.row_mut(row).assign(&aview1(&query.features));
            mask.row_mut(row).assign(&aview1(&query.legal_mask));
        }
        let outputs = self
            .session
            .run(ort::inputs![
                "features" => TensorRef::from_array_view(&features)
                    .map_err(|err| LeafEvalError::Inference(err.to_string()))?,
                "legal_mask" => TensorRef::from_array_view(&mask)
                    .map_err(|err| LeafEvalError::Inference(err.to_string()))?,
            ])
            .map_err(|err| LeafEvalError::Inference(err.to_string()))?;
        // `get` instead of indexing: `SessionOutputs`' `Index` panics on a
        // missing name, which is fatal under the release panic=abort profile.
        let values = outputs
            .get("value")
            .ok_or_else(|| LeafEvalError::Inference("graph has no `value` output".into()))?
            .try_extract_tensor::<f32>()
            .map_err(|err| LeafEvalError::Inference(err.to_string()))?;
        let policies = outputs
            .get("policy")
            .ok_or_else(|| LeafEvalError::Inference("graph has no `policy` output".into()))?
            .try_extract_tensor::<f32>()
            .map_err(|err| LeafEvalError::Inference(err.to_string()))?;
        let (.., value_data) = values;
        let (.., policy_data) = policies;
        if value_data.len() != n || policy_data.len() != n * MAX_ACTIONS {
            return Err(LeafEvalError::Inference(format!(
                "unexpected output shape: value {} policy {}",
                value_data.len(),
                policy_data.len()
            )));
        }
        Ok(batch
            .iter()
            .enumerate()
            .map(|(row, _)| LeafEval {
                ev: value_data[row],
                policy: policy_data[row * MAX_ACTIONS..(row + 1) * MAX_ACTIONS]
                    .try_into()
                    .expect("policy slice is exactly MAX_ACTIONS wide"),
            })
            .collect())
    }
}

impl LeafEvaluator for OrtLeafEvaluator {
    fn evaluate(&mut self, query: &LeafQuery) -> Result<LeafEval, LeafEvalError> {
        self.evaluate_batch(std::slice::from_ref(query))
            .map(|mut results| results.pop().expect("batch of one returns one"))
    }

    fn evaluate_batch(&mut self, queries: &[LeafQuery]) -> Result<Vec<LeafEval>, LeafEvalError> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }
        self.run(queries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        // KOI_LEAF_FIXTURE_DIR points the parity tests at an exported run
        // dir (e.g. runs/leaf-v0) — the committed tests/leaf fixture is the
        // default so CI needs no trained model.
        std::env::var("KOI_LEAF_FIXTURE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/leaf"))
    }

    /// Loads the fixture model; returns `None` (test skips) when no ONNX
    /// Runtime dylib is locatable — the default CI build never ships one.
    fn try_evaluator() -> Option<OrtLeafEvaluator> {
        OrtLeafEvaluator::new(fixture_dir().join("model.onnx"), 2).ok()
    }

    /// Golden parity at the plan's tolerances: value atol 1e-5 / rtol 1e-3,
    /// policy atol 1e-4.
    #[test]
    fn ort_matches_the_exported_golden_outputs() {
        let Some(mut evaluator) = try_evaluator() else {
            eprintln!("skipping ort parity: no onnxruntime dylib (set ORT_DYLIB_PATH)");
            return;
        };
        #[derive(serde::Deserialize)]
        struct Golden {
            features: Vec<Vec<f32>>,
            legal_mask: Vec<Vec<f32>>,
            value: Vec<f32>,
            policy: Vec<Vec<f32>>,
        }
        let golden: Golden = serde_json::from_str(
            &std::fs::read_to_string(fixture_dir().join("golden.json")).expect("golden.json fixture"),
        )
        .expect("golden.json parses");
        let queries: Vec<LeafQuery> = golden
            .features
            .iter()
            .zip(&golden.legal_mask)
            .map(|(f, m)| LeafQuery {
                features: f.clone().try_into().expect("feature width"),
                legal_mask: m.clone().try_into().expect("mask width"),
            })
            .collect();
        let evals = evaluator.evaluate_batch(&queries).expect("inference runs");
        for (row, eval) in evals.iter().enumerate() {
            let want_v = golden.value[row];
            assert!(
                (eval.ev - want_v).abs() <= 1e-5 + 1e-3 * want_v.abs(),
                "row {row}: value {} vs golden {}",
                eval.ev,
                want_v
            );
            for (action, (got, want)) in eval.policy.iter().zip(golden.policy[row].iter()).enumerate() {
                assert!(
                    (got - want).abs() <= 1e-4,
                    "row {row} action {action}: {} vs {}",
                    got,
                    want
                );
            }
        }
    }
}
