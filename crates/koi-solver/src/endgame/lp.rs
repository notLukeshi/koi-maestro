//! Dense two-phase simplex for small standard-form linear programs.
//!
//! Minimizes `c . z` subject to `A z = b`, `z >= 0`, and `b >= 0`.
//! Phase 1 uses an implicit artificial identity basis to find a basic feasible
//! solution; phase 2 reuses the same dense tableau for the true objective.
//! Artificial columns are never materialized: once an artificial leaves the
//! basis it never needs to re-enter, so retaining only its basis index is
//! sufficient. This keeps both phases at `O(m n)` memory rather than
//! `O(m (n + m))` for `m` constraints and `n` variables.
//!
//! The implementation uses Bland's rule throughout: the lowest-index decisive
//! negative reduced cost enters, and exact minimum-ratio ties in the leaving
//! row break to the smallest basic variable index. Bland's rule guarantees
//! termination even on degenerate programs where most-negative reduced-cost
//! rules can cycle (Beale's example); an iteration guard and an explicit
//! numerical-failure path additionally protect against floating-point drift.
//! Each pivot is `O(m n)`; as with every simplex implementation, the number
//! of pivots is exponential in the worst case despite excellent behavior on
//! the small dense sequence-form programs this solver targets.

/// Why a standard-form program could not be solved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpError {
    /// Phase 1 could not drive the artificial variables to zero.
    Infeasible,
    /// The objective decreases without bound along a verified unbounded ray.
    Unbounded,
    /// Dimensions, signs, or floating-point inputs violate the API contract.
    InvalidInput,
    /// Floating-point arithmetic could not maintain the simplex invariants.
    NumericalFailure,
}

impl std::fmt::Display for LpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infeasible => f.write_str("the program has no feasible point"),
            Self::Unbounded => f.write_str("the objective is unbounded below"),
            Self::InvalidInput => f.write_str("the program is not valid standard-form input"),
            Self::NumericalFailure => f.write_str("floating-point simplex invariants were lost"),
        }
    }
}

impl std::error::Error for LpError {}

// The input rows are equilibrated before simplex iterations, so these mixed
// absolute/relative checks are scale-aware rather than raw coefficient cuts.
const FEASIBILITY_ABS_TOLERANCE: f64 = 256.0 * f64::EPSILON;
const FEASIBILITY_REL_TOLERANCE: f64 = 1e-9;
const DUAL_FEASIBILITY_TOLERANCE: f64 = 1e-9;
const ROUND_OFF_MULTIPLIER: f64 = 64.0;
const PIVOT_ABS_TOLERANCE: f64 = 256.0 * f64::EPSILON;
const PIVOT_REL_TOLERANCE: f64 = 128.0 * f64::EPSILON;
const ARTIFICIAL_PIVOT_TOLERANCE: f64 = 1e-12;
const MIN_PIVOT_LIMIT: usize = 100_000;
const PIVOTS_PER_TABLEAU_CELL: usize = 256;
const OBJECTIVE_REFRESH_PASSES: usize = 3;

/// Minimizes `c . z` over `A z = b` with `z >= 0` and `b >= 0`, returning the
/// optimal objective and primal solution.
///
/// Inputs must be finite, `a.len() == b.len() * c.len()`, with `a` laid out in
/// row-major order, and every entry of `b` must be non-negative.
pub fn minimize(c: &[f64], a: &[f64], b: &[f64]) -> Result<(f64, Vec<f64>), LpError> {
    let variables = c.len();
    let (rows, row_scales) = validate_and_prepare_rows(c, a, b)?;

    // With no active equalities the non-negative orthant is the feasible set:
    // any negative cost gives an immediate improving ray, otherwise zero is
    // optimal. Avoid constructing a degenerate one-row tableau.
    if rows == 0 {
        if c.iter().any(|&cost| cost < 0.0) {
            return Err(LpError::Unbounded);
        }
        return Ok((0.0, zeroed_f64_vec(variables)?));
    }

    // Tableau: active constraints plus one objective row. The artificial
    // identity basis is implicit; `basis[row] >= variables` denotes an
    // artificial basic variable. Rows with `0 = 0` were discarded up front.
    let mut tableau = Tableau::new(rows, variables)?;
    fill_phase_one_tableau(&mut tableau, a, b, &row_scales)?;
    drop(row_scales);

    let mut basis = Vec::new();
    basis.try_reserve_exact(rows).map_err(|_| LpError::NumericalFailure)?;
    let artificial_end = variables.checked_add(rows).ok_or(LpError::InvalidInput)?;
    basis.extend(variables..artificial_end);

    find_phase_one_basis(&mut tableau, &mut basis)?;
    remove_basic_artificials(&mut tableau, &mut basis, variables)?;

    // Rows whose artificial remains basic are redundant. Compact them in
    // place, reusing the phase-1 allocation for phase 2.
    tableau.compact_non_artificial_rows(&mut basis, variables);

    match optimize_phase_two(&mut tableau, &mut basis, c)? {
        SimplexStatus::Optimal => {}
        SimplexStatus::Unbounded { entering } => {
            if verify_unbounded_ray(c, a, b, &basis, &tableau, entering)? {
                return Err(LpError::Unbounded);
            }
            return Err(LpError::NumericalFailure);
        }
    }

    let mut solution = zeroed_f64_vec(variables)?;
    for (row, &basic) in basis.iter().enumerate() {
        debug_assert!(basic < variables);
        let value = tableau.row(row)[variables];
        if !value.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        solution[basic] = value;
    }

    sanitize_and_verify_solution(&mut solution, a, b)?;
    let (objective, _) = compensated_dot(c, &solution)?;
    Ok((objective, solution))
}

/// Contiguous row-major tableau. `columns` excludes the right-hand side;
/// `stride == columns + 1`. Keeping one allocation materially improves cache
/// locality over `Vec<Vec<f64>>` and makes every pivot allocation-free.
struct Tableau {
    constraints: usize,
    columns: usize,
    data: Vec<f64>,
}

impl Tableau {
    fn new(constraints: usize, columns: usize) -> Result<Self, LpError> {
        let stride = columns.checked_add(1).ok_or(LpError::InvalidInput)?;
        let rows = constraints.checked_add(1).ok_or(LpError::InvalidInput)?;
        let len = rows.checked_mul(stride).ok_or(LpError::InvalidInput)?;
        let mut data = Vec::new();
        data.try_reserve_exact(len).map_err(|_| LpError::NumericalFailure)?;
        data.resize(len, 0.0);
        Ok(Self {
            constraints,
            columns,
            data,
        })
    }

    #[inline]
    fn stride(&self) -> usize {
        self.columns + 1
    }

    #[inline]
    fn rhs(&self) -> usize {
        self.columns
    }

    #[inline]
    fn row(&self, row: usize) -> &[f64] {
        let stride = self.stride();
        &self.data[row * stride..(row + 1) * stride]
    }

    #[inline]
    fn row_mut(&mut self, row: usize) -> &mut [f64] {
        let stride = self.stride();
        &mut self.data[row * stride..(row + 1) * stride]
    }

    #[inline]
    fn objective(&self) -> &[f64] {
        self.row(self.constraints)
    }

    #[inline]
    fn objective_mut(&mut self) -> &mut [f64] {
        self.row_mut(self.constraints)
    }

    fn compact_non_artificial_rows(&mut self, basis: &mut Vec<usize>, variables: usize) {
        let stride = self.stride();
        let old_constraints = self.constraints;
        let mut target = 0;

        for source in 0..old_constraints {
            if basis[source] >= variables {
                continue;
            }
            if source != target {
                let start = source * stride;
                self.data.copy_within(start..start + stride, target * stride);
                let basic = basis[source];
                basis[target] = basic;
            }
            target += 1;
        }

        basis.truncate(target);
        self.constraints = target;
        self.data.truncate((target + 1) * stride);
        self.objective_mut().fill(0.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimplexStatus {
    Optimal,
    Unbounded { entering: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimplexPhase {
    Feasibility,
    Optimization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnteringColumn {
    None,
    Column(usize),
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeavingRow {
    None,
    Row(usize),
    Ambiguous,
}

/// Finds a feasible basis, refreshing the auxiliary objective when a terminal
/// status is reached. A fresh phase-1 objective with no negative original
/// reduced cost is sufficient to classify any remaining positive artificial
/// as infeasibility; an apparent fresh unbounded direction is numerical error.
fn find_phase_one_basis(tableau: &mut Tableau, basis: &mut [usize]) -> Result<(), LpError> {
    for _ in 0..OBJECTIVE_REFRESH_PASSES {
        let _ = pivot_to_optimality(tableau, basis, SimplexPhase::Feasibility)?;
        if phase_one_feasible(tableau, basis)? {
            return Ok(());
        }

        rebuild_phase_one_objective(tableau, basis)?;
        if phase_one_feasible(tableau, basis)? {
            return Ok(());
        }
        let entering = match entering_column(tableau, basis)? {
            EnteringColumn::None => return Err(LpError::Infeasible),
            EnteringColumn::Column(column) => column,
            EnteringColumn::Ambiguous => return Err(LpError::NumericalFailure),
        };
        match leaving_row(tableau, entering, basis)? {
            LeavingRow::Row(_) => {}
            LeavingRow::None | LeavingRow::Ambiguous => {
                // Sum of artificials is bounded below by zero, so a freshly
                // reconstructed phase-1 objective cannot have an unbounded
                // ray; an unsafe positive pivot is numerical ambiguity too.
                return Err(LpError::NumericalFailure);
            }
        }
        // A fresh improving pivot exists: loop once more from this objective.
    }

    Err(LpError::NumericalFailure)
}

/// Optimizes phase 2 and certifies terminal reduced costs by rebuilding the
/// true objective from the current basis. This prevents accumulated objective
/// row drift from being mistaken for either optimality or unboundedness.
fn optimize_phase_two(tableau: &mut Tableau, basis: &mut [usize], c: &[f64]) -> Result<SimplexStatus, LpError> {
    for _ in 0..OBJECTIVE_REFRESH_PASSES {
        build_phase_two_objective(tableau, basis, c)?;
        let _ = pivot_to_optimality(tableau, basis, SimplexPhase::Optimization)?;

        build_phase_two_objective(tableau, basis, c)?;
        let entering = match entering_column(tableau, basis)? {
            EnteringColumn::None => return Ok(SimplexStatus::Optimal),
            EnteringColumn::Column(column) => column,
            EnteringColumn::Ambiguous => return Err(LpError::NumericalFailure),
        };
        match leaving_row(tableau, entering, basis)? {
            LeavingRow::None => return Ok(SimplexStatus::Unbounded { entering }),
            LeavingRow::Row(_) => {}
            LeavingRow::Ambiguous => return Err(LpError::NumericalFailure),
        }
        // A fresh improving pivot exists: continue from the rebuilt objective.
    }

    Err(LpError::NumericalFailure)
}

/// Pivots until every non-basic reduced cost is non-negative within dual tolerance.
/// Both phases protect against cycling on sub-epsilon noise. Phase 1 separately
/// stops as soon as a feasible basis exists; phase 2 relies on result/ray
/// verification to turn round-off ambiguity into `NumericalFailure`.
/// Entering variables follow Bland's lowest-index rule among decisive
/// (negative reduced-cost) candidates; the leaving row uses a stable
/// candidate-filtered minimum ratio test with Bland smallest-basic-index
/// tie breaking.
fn pivot_to_optimality(
    tableau: &mut Tableau,
    basis: &mut [usize],
    phase: SimplexPhase,
) -> Result<SimplexStatus, LpError> {
    let pivot_limit = pivot_limit(tableau.constraints, tableau.columns);

    for _ in 0..pivot_limit {
        if phase == SimplexPhase::Feasibility && phase_one_feasible(tableau, basis)? {
            return Ok(SimplexStatus::Optimal);
        }
        let entering = match entering_column(tableau, basis)? {
            EnteringColumn::None | EnteringColumn::Ambiguous => return Ok(SimplexStatus::Optimal),
            EnteringColumn::Column(column) => column,
        };
        let leaving = match leaving_row(tableau, entering, basis)? {
            LeavingRow::None => return Ok(SimplexStatus::Unbounded { entering }),
            LeavingRow::Row(row) => row,
            LeavingRow::Ambiguous => return Err(LpError::NumericalFailure),
        };
        pivot(tableau, basis, leaving, entering)?;
    }

    Err(LpError::NumericalFailure)
}

/// Phase 1 may stop as soon as every still-basic artificial has value zero;
/// optimizing its auxiliary objective any further is unnecessary. This also
/// prevents round-off-sized reduced costs from triggering meaningless pivots
/// after a feasible basis has already been found.
fn phase_one_feasible(tableau: &Tableau, basis: &[usize]) -> Result<bool, LpError> {
    let rhs_column = tableau.rhs();
    let dimension = tableau.constraints + tableau.columns + 1;
    let artificial_noise_floor = 8.0 * f64::EPSILON * (dimension as f64);

    for (row, &basic) in basis.iter().enumerate().take(tableau.constraints) {
        let rhs = tableau.row(row)[rhs_column];
        if !rhs.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        let tolerance = feasibility_tolerance(rhs.abs()).max(round_off_tolerance(1.0, dimension));
        if rhs < -tolerance {
            return Err(LpError::NumericalFailure);
        }
        if basic >= tableau.columns && rhs > artificial_noise_floor {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The leaving row for an entering column: evaluates every mathematically
/// positive coefficient, but pivots only on numerically safe candidates. If an
/// unsafe positive coefficient would win the exact minimum-ratio ordering, the
/// result is ambiguous rather than falsely declaring unboundedness or violating
/// primal feasibility. Ties use the smallest basic variable index (Bland).
fn leaving_row(tableau: &Tableau, entering: usize, basis: &[usize]) -> Result<LeavingRow, LpError> {
    let rhs_column = tableau.rhs();
    let mut largest_positive = 0.0_f64;

    for row in 0..tableau.constraints {
        let coefficient = tableau.row(row)[entering];
        if !coefficient.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        if coefficient > largest_positive {
            largest_positive = coefficient;
        }
    }

    if largest_positive == 0.0 {
        return Ok(LeavingRow::None);
    }

    let dimension = tableau.constraints + tableau.columns + 1;
    let coeff_noise_floor = (dimension as f64) * f64::EPSILON;
    let rhs_noise_floor = 8.0 * f64::EPSILON * (dimension as f64);
    let pivot_threshold = (PIVOT_REL_TOLERANCE * largest_positive).max(PIVOT_ABS_TOLERANCE);
    let mut safe: Option<(f64, usize, usize)> = None;
    let mut unsafe_candidate: Option<(f64, usize, usize)> = None;

    let neg_tolerance = round_off_tolerance(1.0, dimension);

    for (row, &basic) in basis.iter().enumerate().take(tableau.constraints) {
        let values = tableau.row(row);
        let coefficient = values[entering];
        if coefficient <= coeff_noise_floor {
            continue;
        }

        let rhs = values[rhs_column];
        if !rhs.is_finite() {
            return Err(LpError::NumericalFailure);
        }

        let rhs = if rhs.abs() <= rhs_noise_floor {
            0.0
        } else if rhs > 0.0 {
            rhs
        } else if rhs >= -feasibility_tolerance(rhs.abs()).max(neg_tolerance) {
            0.0
        } else {
            return Err(LpError::NumericalFailure);
        };

        let ratio = rhs / coefficient;
        if !ratio.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        let candidate = (ratio, basic, row);
        let target = if coefficient > pivot_threshold {
            &mut safe
        } else {
            &mut unsafe_candidate
        };

        let replace = match *target {
            None => true,
            Some((best_ratio, best_basic, _)) => ratio < best_ratio || (ratio == best_ratio && basis[row] < best_basic),
        };
        if replace {
            *target = Some(candidate);
        }
    }

    match (safe, unsafe_candidate) {
        (None, None) => Ok(LeavingRow::None),
        (None, Some(_)) => Ok(LeavingRow::Ambiguous),
        (Some((_, _, row)), None) => Ok(LeavingRow::Row(row)),
        (Some((safe_ratio, _, safe_row)), Some((unsafe_ratio, _, _))) => {
            if unsafe_ratio < safe_ratio {
                Ok(LeavingRow::Ambiguous)
            } else {
                Ok(LeavingRow::Row(safe_row))
            }
        }
    }
}

/// Allocation-free Gauss-Jordan pivot. Dispatches once per pivot to an AVX2+FMA
/// vector elimination kernel on supported x86_64 targets.
fn pivot(tableau: &mut Tableau, basis: &mut [usize], row: usize, column: usize) -> Result<(), LpError> {
    let stride = tableau.stride();
    let row_start = row * stride;
    let (before, pivot_and_after) = tableau.data.split_at_mut(row_start);
    let (pivot_row, after) = pivot_and_after.split_at_mut(stride);
    let pivot_value = pivot_row[column];
    if !pivot_value.is_finite() || pivot_value == 0.0 {
        return Err(LpError::NumericalFailure);
    }

    if pivot_value == -1.0 {
        for value in pivot_row.iter_mut() {
            *value = -*value;
        }
    } else if pivot_value != 1.0 {
        let reciprocal = pivot_value.recip();
        if !reciprocal.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        for value in pivot_row.iter_mut() {
            *value *= reciprocal;
        }
    }
    pivot_row[column] = 1.0;

    // Forced-scalar escape hatch (P2-D4): `KOI_LP_SCALAR=1` routes every
    // pivot through the non-fused path so FMA-vs-scalar last-bit divergence
    // is measurable on any target, including aarch64 where FMA is otherwise
    // unconditional.
    if scalar_pivots() {
        eliminate_rows_scalar(before, after, pivot_row, stride, column);
        basis[row] = column;
        return Ok(());
    }

    #[cfg(all(not(target_feature = "fma"), not(target_arch = "aarch64"), target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            // Runtime-detected AVX2/FMA dispatch — the only unsafe in the
            // solver; reviewed against the #[target_feature] contract below.
            #[allow(unsafe_code)]
            unsafe {
                eliminate_rows_avx2_fma(before, after, pivot_row, stride, column);
            }
            basis[row] = column;
            return Ok(());
        }
    }

    eliminate_rows_standard(before, after, pivot_row, stride, column);

    basis[row] = column;
    Ok(())
}

/// Reads `KOI_LP_SCALAR` once per process: `1`/`true` forces the non-fused
/// pivot path for cross-platform divergence measurements.
fn scalar_pivots() -> bool {
    static FORCED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FORCED.get_or_init(|| std::env::var_os("KOI_LP_SCALAR").is_some_and(|value| value == "1" || value == "true"))
}

/// The non-fused elimination over the same before/after split — reachable
/// on every target via `scalar_pivots`, and the default on targets with
/// neither compile-time FMA nor NEON.
#[inline]
fn eliminate_rows_scalar(before: &mut [f64], after: &mut [f64], pivot_row: &[f64], stride: usize, column: usize) {
    for other in before.chunks_exact_mut(stride) {
        eliminate_row_fallback(other, pivot_row, column);
    }
    for other in after.chunks_exact_mut(stride) {
        eliminate_row_fallback(other, pivot_row, column);
    }
}

#[cfg(all(not(target_feature = "fma"), not(target_arch = "aarch64"), target_arch = "x86_64"))]
#[target_feature(enable = "avx2,fma")]
#[allow(unsafe_code)] // #[target_feature] fns must be unsafe; call site is runtime-gated
unsafe fn eliminate_rows_avx2_fma(
    before: &mut [f64],
    after: &mut [f64],
    pivot_row: &[f64],
    stride: usize,
    column: usize,
) {
    for other in before.chunks_exact_mut(stride) {
        eliminate_row_fast(other, pivot_row, column);
    }
    for other in after.chunks_exact_mut(stride) {
        eliminate_row_fast(other, pivot_row, column);
    }
}

#[inline]
fn eliminate_rows_standard(before: &mut [f64], after: &mut [f64], pivot_row: &[f64], stride: usize, column: usize) {
    #[cfg(any(target_feature = "fma", target_arch = "aarch64"))]
    {
        for other in before.chunks_exact_mut(stride) {
            eliminate_row_fast(other, pivot_row, column);
        }
        for other in after.chunks_exact_mut(stride) {
            eliminate_row_fast(other, pivot_row, column);
        }
    }

    #[cfg(not(any(target_feature = "fma", target_arch = "aarch64")))]
    {
        for other in before.chunks_exact_mut(stride) {
            eliminate_row_fallback(other, pivot_row, column);
        }
        for other in after.chunks_exact_mut(stride) {
            eliminate_row_fallback(other, pivot_row, column);
        }
    }
}

#[inline(always)]
fn eliminate_row_fast(other: &mut [f64], pivot_row: &[f64], column: usize) {
    let factor = other[column];
    if factor == 0.0 {
        return;
    }
    assert_eq!(other.len(), pivot_row.len());

    if factor == 1.0 {
        for (slot, &pivot) in other.iter_mut().zip(pivot_row) {
            *slot -= pivot;
        }
    } else if factor == -1.0 {
        for (slot, &pivot) in other.iter_mut().zip(pivot_row) {
            *slot += pivot;
        }
    } else {
        eliminate_row_axpy(other, pivot_row, -factor);
    }
    other[column] = 0.0;
}

/// Fused multiply-add AXPY row update: `other -= factor * pivot_row`.
///
/// AArch64 (Armv9 / Grace Blackwell, Cortex-X925 + Cortex-A725) gets an
/// explicit NEON `float64x2` kernel: `vfmaq_f64` is single-cycle hardware FMA
/// and `vld1q`/`vst1q` tolerate the unaligned mid-row windows used here.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
#[allow(unsafe_code)] // NEON intrinsics are unsafe; AArch64 mandates NEON/FP64 — see SAFETY note
fn eliminate_row_axpy(other: &mut [f64], pivot_row: &[f64], neg_factor: f64) {
    use std::arch::aarch64::*;

    // SAFETY: AArch64 mandates NEON/FP64, so no runtime feature check is
    // needed. `vld1q`/`vst1q` accept unaligned addresses, and every vector
    // access stays within `other`/`pivot_row` because the loop bound is
    // `i + 2 <= len` and both slices have identical length.
    unsafe {
        let factor = vdupq_n_f64(neg_factor);
        let mut i = 0;
        while i + 2 <= other.len() {
            let o = vld1q_f64(other.as_ptr().add(i));
            let p = vld1q_f64(pivot_row.as_ptr().add(i));
            vst1q_f64(other.as_mut_ptr().add(i), vfmaq_f64(o, factor, p));
            i += 2;
        }
        while i < other.len() {
            other[i] = neg_factor.mul_add(pivot_row[i], other[i]);
            i += 1;
        }
    }
}

/// Scalar `mul_add` AXPY row update for non-AArch64 targets; on x86_64 the
/// surrounding `#[target_feature(enable = "avx2,fma")]` frame lets LLVM
/// auto-vectorize this loop into `vfmadd213pd` quads.
#[cfg(not(target_arch = "aarch64"))]
#[inline(always)]
fn eliminate_row_axpy(other: &mut [f64], pivot_row: &[f64], neg_factor: f64) {
    for (slot, &pivot) in other.iter_mut().zip(pivot_row) {
        *slot = neg_factor.mul_add(pivot, *slot);
    }
}

#[inline(always)]
fn eliminate_row_fallback(other: &mut [f64], pivot_row: &[f64], column: usize) {
    let factor = other[column];
    if factor == 0.0 {
        return;
    }
    assert_eq!(other.len(), pivot_row.len());

    if factor == 1.0 {
        for (slot, &pivot) in other.iter_mut().zip(pivot_row) {
            *slot -= pivot;
        }
    } else {
        let neg_factor = -factor;
        for (slot, &pivot) in other.iter_mut().zip(pivot_row) {
            *slot += neg_factor * pivot;
        }
    }
    other[column] = 0.0;
}

/// Selects the entering column with Bland's lowest-index rule among decisive
/// negative costs, which guarantees termination on degenerate programs where
/// Dantzig's most-negative rule can cycle (Beale's example).
fn entering_column(tableau: &Tableau, basis: &[usize]) -> Result<EnteringColumn, LpError> {
    let dual_zero_tol = round_off_tolerance(1.0, tableau.columns + 1);
    let mut ambiguous = false;

    for (column, &value) in tableau.objective()[..tableau.columns].iter().enumerate() {
        if !value.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        if value < -DUAL_FEASIBILITY_TOLERANCE {
            debug_assert!(
                !basis.contains(&column),
                "basic variable cannot have negative reduced cost"
            );
            return Ok(EnteringColumn::Column(column));
        }
        if value < -dual_zero_tol {
            debug_assert!(
                !basis.contains(&column),
                "basic variable cannot have negative reduced cost"
            );
            // A noise-level reduced cost alone cannot enter: keep scanning
            // for a decisive column; the caller re-verifies ambiguity with
            // a freshly rebuilt objective row.
            ambiguous = true;
        }
    }

    if ambiguous {
        Ok(EnteringColumn::Ambiguous)
    } else {
        Ok(EnteringColumn::None)
    }
}

/// Reconstructs the phase-1 objective from the current basis. Original basic
/// variables have auxiliary cost zero; still-basic artificials have cost one.
/// Artificial columns remain implicit, so only original reduced costs and the
/// objective right-hand side are required.
fn rebuild_phase_one_objective(tableau: &mut Tableau, basis: &[usize]) -> Result<(), LpError> {
    let columns = tableau.columns;
    let stride = tableau.stride();
    let split = tableau.constraints * stride;
    let (constraints, objective_and_tail) = tableau.data.split_at_mut(split);
    let objective = &mut objective_and_tail[..stride];
    objective.fill(0.0);

    for (row, basic) in constraints.chunks_exact(stride).zip(basis.iter().copied()) {
        if basic < columns {
            continue;
        }
        for (reduced, &value) in objective.iter_mut().zip(row) {
            *reduced -= value;
        }
    }

    for &basic in basis {
        if basic < columns {
            objective[basic] = 0.0;
        }
    }

    if objective.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(LpError::NumericalFailure)
    }
}

/// Checks phase-1 feasibility, pivots zero-valued basic artificials out when a
/// stable original column exists, and leaves truly redundant rows marked by an
/// artificial basis index for in-place compaction.
fn remove_basic_artificials(tableau: &mut Tableau, basis: &mut [usize], variables: usize) -> Result<(), LpError> {
    let rhs_column = tableau.rhs();
    let dimension = tableau.constraints + variables + 1;
    let artificial_noise_floor = 8.0 * f64::EPSILON * (dimension as f64);

    for (row, &basic) in basis.iter().enumerate().take(tableau.constraints) {
        let rhs = tableau.row(row)[rhs_column];
        if !rhs.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        let tolerance = feasibility_tolerance(rhs.abs()).max(round_off_tolerance(1.0, dimension));
        if rhs < -tolerance {
            return Err(LpError::NumericalFailure);
        }
        if basic >= variables && rhs > artificial_noise_floor {
            return Err(LpError::Infeasible);
        }
        if rhs.abs() <= artificial_noise_floor || rhs < 0.0 {
            tableau.row_mut(row)[rhs_column] = 0.0;
        }
    }

    // Avoid heap allocations for typical small problems (<= 256 variables).
    let mut stack_basic = [0u8; 256];
    let mut heap_basic;
    let basic_columns: &mut [u8] = if variables <= stack_basic.len() {
        &mut stack_basic[..variables]
    } else {
        heap_basic = zeroed_u8_vec(variables)?;
        &mut heap_basic[..]
    };

    for &basic in basis.iter() {
        if basic < variables {
            basic_columns[basic] = 1;
        }
    }

    let coeff_noise_floor = round_off_tolerance(1.0, dimension);

    // Snapshot which rows still carry an artificial basic variable: `pivot`
    // re-borrows `basis` mutably, so the loop cannot hold the iterator.
    let artificial_rows: Vec<usize> = basis
        .iter()
        .take(tableau.constraints)
        .copied()
        .enumerate()
        .filter(|&(_, basic)| basic >= variables)
        .map(|(row, _)| row)
        .collect();

    for &row in &artificial_rows {
        let values = tableau.row(row);
        let mut entering = None;
        let mut largest = 0.0_f64;
        for (column, &coefficient) in values[..variables].iter().enumerate() {
            if !coefficient.is_finite() {
                return Err(LpError::NumericalFailure);
            }
            if basic_columns[column] != 0 {
                continue;
            }
            let magnitude = coefficient.abs();
            if magnitude > largest {
                largest = magnitude;
                entering = Some(column);
            }
        }

        if largest <= coeff_noise_floor {
            continue;
        }
        let redundancy_threshold = coeff_noise_floor.max(ARTIFICIAL_PIVOT_TOLERANCE);
        if largest <= redundancy_threshold {
            return Err(LpError::NumericalFailure);
        }

        let entering = entering.expect("positive largest magnitude has a column");
        pivot(tableau, basis, row, entering)?;
        basic_columns[entering] = 1;
    }

    Ok(())
}

fn build_phase_two_objective(tableau: &mut Tableau, basis: &[usize], c: &[f64]) -> Result<(), LpError> {
    let largest_cost = c.iter().fold(0.0_f64, |largest, &cost| largest.max(cost.abs()));
    let scale = if largest_cost == 0.0 { 1.0 } else { largest_cost };
    let columns = tableau.columns;
    let stride = tableau.stride();
    let split = tableau.constraints * stride;
    let (constraints, objective_and_tail) = tableau.data.split_at_mut(split);
    let objective = &mut objective_and_tail[..stride];
    objective.fill(0.0);

    for (reduced, &cost) in objective[..columns].iter_mut().zip(c) {
        let scaled = cost / scale;
        if !scaled.is_finite() || (cost != 0.0 && scaled == 0.0) {
            return Err(LpError::NumericalFailure);
        }
        *reduced = scaled;
    }

    for (row, basic) in constraints.chunks_exact(stride).zip(basis.iter().copied()) {
        let basic_cost = c[basic] / scale;
        if basic_cost == 0.0 {
            continue;
        }
        for (reduced, &value) in objective.iter_mut().zip(row) {
            *reduced -= basic_cost * value;
        }
    }

    for &basic in basis {
        if basic < columns {
            objective[basic] = 0.0;
        }
    }

    if objective.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(LpError::NumericalFailure)
    }
}

/// Validates shape/numeric preconditions, computes each row's equilibration
/// factor once, and counts active constraints. A zero scale marks an exact
/// redundant `0 = 0` row; exact `0 = b`, `b > 0`, is immediately infeasible.
fn validate_and_prepare_rows(c: &[f64], a: &[f64], b: &[f64]) -> Result<(usize, Vec<f64>), LpError> {
    if c.iter().any(|value| !value.is_finite()) {
        return Err(LpError::InvalidInput);
    }

    let variables = c.len();
    let constraints = b.len();
    let expected_len = constraints.checked_mul(variables).ok_or(LpError::InvalidInput)?;
    if a.len() != expected_len {
        return Err(LpError::InvalidInput);
    }

    let mut active = 0usize;
    let mut scales = Vec::new();
    scales
        .try_reserve_exact(constraints)
        .map_err(|_| LpError::NumericalFailure)?;

    if variables == 0 {
        for &rhs in b {
            if !rhs.is_finite() || rhs < 0.0 {
                return Err(LpError::InvalidInput);
            }
            if rhs != 0.0 {
                return Err(LpError::Infeasible);
            }
            scales.push(0.0);
        }
        return Ok((0, scales));
    }

    for (row, &rhs) in a.chunks_exact(variables).zip(b) {
        if !rhs.is_finite() || rhs < 0.0 {
            return Err(LpError::InvalidInput);
        }

        let mut norm = 0.0_f64;
        for &value in row {
            if !value.is_finite() {
                return Err(LpError::InvalidInput);
            }
            norm = norm.max(value.abs());
        }

        if norm == 0.0 {
            if rhs != 0.0 {
                return Err(LpError::Infeasible);
            }
            scales.push(0.0);
            continue;
        }

        let scale = norm.recip();
        if !scale.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        scales.push(scale);
        active = active.checked_add(1).ok_or(LpError::InvalidInput)?;
    }

    Ok((active, scales))
}

/// Fills the row-equilibrated constraints and phase-1 objective together.
/// Phase 1 stores only original columns: the artificial identity basis is
/// represented by `basis`, and nonbasic artificials never re-enter. Positive
/// row scaling preserves the LP while giving every active `A` row infinity
/// norm one for ordinary finite inputs.
fn fill_phase_one_tableau(tableau: &mut Tableau, a: &[f64], b: &[f64], row_scales: &[f64]) -> Result<(), LpError> {
    let columns = tableau.columns;
    let stride = tableau.stride();
    let objective_start = tableau.constraints * stride;
    let (constraints, objective_and_tail) = tableau.data.split_at_mut(objective_start);
    let objective = &mut objective_and_tail[..stride];
    let mut target = 0usize;

    for ((row, &rhs), &scale) in a.chunks_exact(columns).zip(b).zip(row_scales) {
        if scale == 0.0 {
            continue;
        }

        let start = target * stride;
        let target_row = &mut constraints[start..start + stride];
        for (column, (&value, slot)) in row.iter().zip(&mut target_row[..columns]).enumerate() {
            let scaled = value * scale;
            if !scaled.is_finite() || (value != 0.0 && scaled == 0.0) {
                return Err(LpError::NumericalFailure);
            }
            *slot = scaled;
            objective[column] -= scaled;
        }

        let scaled_rhs = rhs * scale;
        if !scaled_rhs.is_finite() || (rhs != 0.0 && scaled_rhs == 0.0) {
            return Err(LpError::NumericalFailure);
        }
        target_row[columns] = scaled_rhs;
        objective[columns] -= scaled_rhs;
        target += 1;
    }

    debug_assert_eq!(target, tableau.constraints);
    if objective.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(LpError::NumericalFailure)
    }
}

/// Verifies a tableau unbounded direction against the original unscaled LP.
/// This converts numerical ambiguity into `NumericalFailure` instead of a
/// false mathematical `Unbounded` result.
fn verify_unbounded_ray(
    c: &[f64],
    a: &[f64],
    b: &[f64],
    basis: &[usize],
    tableau: &Tableau,
    entering: usize,
) -> Result<bool, LpError> {
    let mut point = zeroed_f64_vec(c.len())?;
    for (row, &basic) in basis.iter().enumerate() {
        let value = tableau.row(row)[tableau.rhs()];
        if !value.is_finite() {
            return Ok(false);
        }
        point[basic] = value;
    }
    if sanitize_and_verify_solution(&mut point, a, b).is_err() {
        return Ok(false);
    }
    drop(point);

    let mut ray = zeroed_f64_vec(c.len())?;
    ray[entering] = 1.0;
    for (row, &basic) in basis.iter().enumerate() {
        let direction = -tableau.row(row)[entering];
        if !direction.is_finite() {
            return Ok(false);
        }
        ray[basic] = direction;
    }

    let largest_direction = ray.iter().fold(0.0_f64, |largest, &value| largest.max(value.abs()));
    if largest_direction == 0.0 || !largest_direction.is_finite() {
        return Ok(false);
    }
    for value in &mut ray {
        *value /= largest_direction;
    }

    let nonnegative_tolerance = round_off_tolerance(1.0, ray.len() + 1);
    if ray.iter().any(|&value| value < -nonnegative_tolerance) {
        return Ok(false);
    }
    for value in &mut ray {
        if *value < 0.0 {
            *value = 0.0;
        }
    }

    if !c.is_empty() {
        for row in a.chunks_exact(c.len()) {
            let row_norm = row.iter().fold(0.0_f64, |largest, &value| largest.max(value.abs()));
            if row_norm == 0.0 {
                continue;
            }
            let (residual, activity) = compensated_scaled_dot(row, row_norm.recip(), &ray)?;
            let tolerance = round_off_tolerance(1.0, row.len() + 1).max(feasibility_tolerance(activity));
            if residual.abs() > tolerance {
                return Ok(false);
            }
        }
    }

    let objective_norm = c.iter().fold(0.0_f64, |largest, &value| largest.max(value.abs()));
    if objective_norm == 0.0 {
        return Ok(false);
    }
    let (descent, _) = compensated_scaled_dot(c, objective_norm.recip(), &ray)?;
    Ok(descent < -round_off_tolerance(1.0, c.len() + 1))
}

fn sanitize_and_verify_solution(solution: &mut [f64], a: &[f64], b: &[f64]) -> Result<(), LpError> {
    for value in solution.iter_mut() {
        let nonnegative_tolerance = feasibility_tolerance(value.abs());
        if !value.is_finite() || *value < -nonnegative_tolerance {
            return Err(LpError::NumericalFailure);
        }
        if *value < 0.0 {
            *value = 0.0;
        }
    }

    if solution.is_empty() {
        for &rhs in b {
            if rhs != 0.0 {
                return Err(LpError::NumericalFailure);
            }
        }
        return Ok(());
    }

    for (row, &rhs) in a.chunks_exact(solution.len()).zip(b) {
        let row_norm = row.iter().fold(0.0_f64, |largest, &value| largest.max(value.abs()));
        if row_norm == 0.0 {
            if rhs != 0.0 {
                return Err(LpError::NumericalFailure);
            }
            continue;
        }
        let scale = row_norm.recip();
        let (lhs, activity) = compensated_scaled_dot(row, scale, solution)?;
        let scaled_rhs = rhs * scale;
        if !scaled_rhs.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        let tol = feasibility_tolerance(scaled_rhs.abs() + activity);
        if (lhs - scaled_rhs).abs() > tol {
            return Err(LpError::NumericalFailure);
        }
    }
    Ok(())
}

/// Neumaier-compensated dot product plus an absolute activity estimate used by
/// residual checks. It is intentionally kept off the hot pivot path.
fn compensated_dot(left: &[f64], right: &[f64]) -> Result<(f64, f64), LpError> {
    debug_assert_eq!(left.len(), right.len());
    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    let mut activity = 0.0_f64;

    for (&lhs, &rhs) in left.iter().zip(right) {
        let term = lhs * rhs;
        if !term.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        activity += term.abs();
        if !activity.is_finite() {
            return Err(LpError::NumericalFailure);
        }

        let next = sum + term;
        if !next.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        compensation += if sum.abs() >= term.abs() {
            (sum - next) + term
        } else {
            (term - next) + sum
        };
        sum = next;
    }

    let result = sum + compensation;
    if result.is_finite() {
        Ok((result, activity))
    } else {
        Err(LpError::NumericalFailure)
    }
}

fn compensated_scaled_dot(left: &[f64], left_scale: f64, right: &[f64]) -> Result<(f64, f64), LpError> {
    debug_assert_eq!(left.len(), right.len());
    if !left_scale.is_finite() {
        return Err(LpError::NumericalFailure);
    }

    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    let mut activity = 0.0_f64;
    for (&lhs, &rhs) in left.iter().zip(right) {
        let term = (lhs * left_scale) * rhs;
        if !term.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        activity += term.abs();
        if !activity.is_finite() {
            return Err(LpError::NumericalFailure);
        }

        let next = sum + term;
        if !next.is_finite() {
            return Err(LpError::NumericalFailure);
        }
        compensation += if sum.abs() >= term.abs() {
            (sum - next) + term
        } else {
            (term - next) + sum
        };
        sum = next;
    }

    let result = sum + compensation;
    if result.is_finite() {
        Ok((result, activity))
    } else {
        Err(LpError::NumericalFailure)
    }
}

#[inline]
fn feasibility_tolerance(scale: f64) -> f64 {
    FEASIBILITY_ABS_TOLERANCE + FEASIBILITY_REL_TOLERANCE * scale
}

#[inline]
fn round_off_tolerance(scale: f64, operations: usize) -> f64 {
    ROUND_OFF_MULTIPLIER * f64::EPSILON * (operations.max(1) as f64) * scale.max(1.0)
}

#[inline]
fn pivot_limit(constraints: usize, variables: usize) -> usize {
    constraints
        .saturating_add(1)
        .saturating_mul(variables.saturating_add(1))
        .saturating_mul(PIVOTS_PER_TABLEAU_CELL)
        .max(MIN_PIVOT_LIMIT)
}

fn zeroed_f64_vec(len: usize) -> Result<Vec<f64>, LpError> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).map_err(|_| LpError::NumericalFailure)?;
    values.resize(len, 0.0);
    Ok(values)
}

fn zeroed_u8_vec(len: usize) -> Result<Vec<u8>, LpError> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).map_err(|_| LpError::NumericalFailure)?;
    values.resize(len, 0);
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        let tolerance = 1e-8 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected:.16e}, got {actual:.16e}"
        );
    }

    fn assert_primal(a: &[f64], b: &[f64], solution: &[f64]) {
        assert!(solution.iter().all(|&value| value >= -1e-9));
        let cols = solution.len();
        if cols == 0 {
            for &rhs in b {
                assert_close(0.0, rhs);
            }
            return;
        }
        for (row, &rhs) in a.chunks_exact(cols).zip(b) {
            let lhs: f64 = row.iter().zip(solution).map(|(&x, &y)| x * y).sum();
            assert_close(lhs, rhs);
        }
    }

    fn brute_force_bounded_optimum(c: &[f64], a: &[f64], b: &[f64]) -> f64 {
        let rows = b.len();
        let variables = c.len();
        assert!(rows <= variables && variables < usize::BITS as usize);
        let mut best = f64::INFINITY;

        for mask in 0usize..(1usize << variables) {
            if mask.count_ones() as usize != rows {
                continue;
            }
            let columns: Vec<usize> = (0..variables)
                .filter(|&column| mask & (1usize << column) != 0)
                .collect();
            let Some(values) = solve_basis(a, b, variables, &columns) else {
                continue;
            };
            if values.iter().any(|&value| value < -1e-10) {
                continue;
            }

            let objective: f64 = columns
                .iter()
                .zip(&values)
                .map(|(&column, &value)| c[column] * value.max(0.0))
                .sum();
            best = best.min(objective);
        }

        assert!(best.is_finite(), "constructed test LP has no basic feasible solution");
        best
    }

    fn solve_basis(a: &[f64], b: &[f64], variables: usize, columns: &[usize]) -> Option<Vec<f64>> {
        let rows = b.len();
        let mut matrix = vec![vec![0.0; rows + 1]; rows];
        for (row_idx, row) in a.chunks_exact(variables).enumerate() {
            for (col_idx, &source) in columns.iter().enumerate() {
                matrix[row_idx][col_idx] = row[source];
            }
            matrix[row_idx][rows] = b[row_idx];
        }

        for column in 0..rows {
            let pivot = (column..rows)
                .max_by(|&left, &right| matrix[left][column].abs().total_cmp(&matrix[right][column].abs()))?;
            if matrix[pivot][column].abs() <= 1e-12 {
                return None;
            }
            matrix.swap(column, pivot);

            let reciprocal = matrix[column][column].recip();
            for value in &mut matrix[column][column..=rows] {
                *value *= reciprocal;
            }
            let pivot_row = matrix[column].clone();
            for (index, row_values) in matrix.iter_mut().enumerate().take(rows) {
                if index == column {
                    continue;
                }
                let factor = row_values[column];
                if factor == 0.0 {
                    continue;
                }
                for (slot, &pivot) in row_values[column..=rows].iter_mut().zip(&pivot_row[column..=rows]) {
                    *slot -= factor * pivot;
                }
            }
        }

        Some((0..rows).map(|row| matrix[row][rows]).collect())
    }

    fn draw(state: &mut u64, upper: u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state % upper
    }

    #[test]
    fn solves_homogeneous_constraint_with_large_variables() {
        let a = [1.0, -3.0, 0.0, 3.0];
        let b = [0.0, 100_000.0];
        let (objective, solution) = minimize(&[1.0, 0.0], &a, &b).unwrap();
        assert_close(objective, 100_000.0);
        assert_close(solution[0], 100_000.0);
        assert_close(solution[1], 100_000.0 / 3.0);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn solves_a_trivial_program() {
        let a = [1.0, 1.0];
        let b = [1.0];
        let (objective, solution) = minimize(&[1.0, 1.0], &a, &b).unwrap();
        assert_close(objective, 1.0);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn solves_a_bounded_choice() {
        let a = [1.0, 1.0, 1.0, 0.0, 1.0, 3.0, 0.0, 1.0];
        let b = [4.0, 6.0];
        let (objective, solution) = minimize(&[-2.0, -3.0, 0.0, 0.0], &a, &b).unwrap();
        assert_close(objective, -9.0);
        assert_close(solution[0], 3.0);
        assert_close(solution[1], 1.0);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn detects_infeasibility() {
        let a = [1.0, 1.0];
        let error = minimize(&[1.0], &a, &[1.0, 2.0]).unwrap_err();
        assert_eq!(error, LpError::Infeasible);
    }

    #[test]
    fn detects_unboundedness() {
        let a = [1.0, -1.0];
        let error = minimize(&[-1.0, 0.0], &a, &[0.0]).unwrap_err();
        assert_eq!(error, LpError::Unbounded);
    }

    #[test]
    fn bland_handles_beales_cycling_example() {
        let a = [
            0.5, -5.5, -2.5, 9.0, 1.0, 0.0, 0.0, 0.5, -1.5, -0.5, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let b = [0.0, 0.0, 1.0];
        let c = [-10.0, 57.0, 9.0, 24.0, 0.0, 0.0, 0.0];
        let (objective, solution) = minimize(&c, &a, &b).unwrap();
        assert_close(objective, -1.0);
        assert_close(solution[0], 1.0);
        assert_close(solution[2], 1.0);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn degenerate_programs_terminate_without_cycling() {
        let a = [
            1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        ];
        let b = [1.0, 1.0, 1.0];
        let (objective, solution) = minimize(&[-1.0, -1.0, 0.0, 0.0, 0.0], &a, &b).unwrap();
        assert_close(objective, -1.0);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn removes_redundant_constraints_and_artificials() {
        let a = [1.0, 1.0, 2.0, 2.0];
        let b = [1.0, 2.0];
        let (objective, solution) = minimize(&[1.0, 0.0], &a, &b).unwrap();
        assert_close(objective, 0.0);
        assert_close(solution[0], 0.0);
        assert_close(solution[1], 1.0);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn drops_zero_equalities_and_rejects_impossible_zero_rows() {
        let a = [1.0, 0.0];
        let b = [2.0, 0.0];
        let (objective, solution) = minimize(&[1.0], &a, &b).unwrap();
        assert_close(objective, 2.0);
        assert_close(solution[0], 2.0);

        let error = minimize(&[0.0], &[0.0], &[1.0]).unwrap_err();
        assert_eq!(error, LpError::Infeasible);
    }

    #[test]
    fn detects_small_scale_infeasibility() {
        let a = [1.0, 1.0];
        let error = minimize(&[0.0], &a, &[0.0, 1e-12]).unwrap_err();
        assert_eq!(error, LpError::Infeasible);
    }

    #[test]
    fn preserves_tiny_positive_basic_values() {
        let a = [1.0];
        let b = [1e-14];
        let (objective, solution) = minimize(&[1.0], &a, &b).unwrap();
        assert!(solution[0] > 0.0);
        assert!((solution[0] - 1e-14).abs() <= 1e-28);
        assert!((objective - 1e-14).abs() <= 1e-28);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn row_equilibration_handles_tiny_coefficients() {
        let a = [1e-12];
        let b = [1.0];
        let (objective, solution) = minimize(&[0.0], &a, &b).unwrap();
        assert_close(objective, 0.0);
        assert!((solution[0] - 1e12).abs() <= 1e4);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn scaled_unbounded_ray_is_verified() {
        let a = [1e-12, -1e-12];
        let b = [0.0];
        let error = minimize(&[-1.0, 0.0], &a, &b).unwrap_err();
        assert_eq!(error, LpError::Unbounded);
    }

    #[test]
    fn handles_no_constraints() {
        let (objective, solution) = minimize(&[1.0, 2.0], &[], &[]).unwrap();
        assert_close(objective, 0.0);
        assert_eq!(solution, vec![0.0, 0.0]);

        let error = minimize(&[-1.0], &[], &[]).unwrap_err();
        assert_eq!(error, LpError::Unbounded);
    }

    #[test]
    fn handles_no_variables() {
        let (objective, solution) = minimize(&[], &[], &[0.0]).unwrap();
        assert_close(objective, 0.0);
        assert!(solution.is_empty());

        let error = minimize(&[], &[], &[1.0]).unwrap_err();
        assert_eq!(error, LpError::Infeasible);
    }

    #[test]
    fn accepts_multiple_optima() {
        let a = [1.0, 1.0];
        let b = [1.0];
        let (objective, solution) = minimize(&[1.0, 1.0], &a, &b).unwrap();
        assert_close(objective, 1.0);
        assert_primal(&a, &b, &solution);
    }

    #[test]
    fn deterministic_small_lps_match_exhaustive_basis_enumeration() {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for _ in 0..128 {
            let ux_i = 1 + draw(&mut state, 6);
            let uy_i = 1 + draw(&mut state, 6);
            let p_i = draw(&mut state, 5);
            let q_i = draw(&mut state, 5);
            let w_i = draw(&mut state, p_i * ux_i + q_i * uy_i + 7);
            let cx = draw(&mut state, 11) as i64 - 5;
            let cy = draw(&mut state, 11) as i64 - 5;

            let a = [
                1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, p_i as f64, q_i as f64, 0.0, 0.0, 1.0,
            ];
            let b = [ux_i as f64, uy_i as f64, w_i as f64];
            let c = [cx as f64, cy as f64, 0.0, 0.0, 0.0];

            let expected = brute_force_bounded_optimum(&c, &a, &b);
            let (objective, solution) = minimize(&c, &a, &b).unwrap();
            assert_close(objective, expected);
            assert_primal(&a, &b, &solution);
        }
    }

    #[test]
    fn tiny_objective_scale_does_not_change_unboundedness() {
        let error = minimize(&[-1e-30], &[], &[]).unwrap_err();
        assert_eq!(error, LpError::Unbounded);
    }

    #[test]
    fn mixed_objective_scale_does_not_hide_unboundedness() {
        let error = minimize(&[1.0, -5e-10], &[], &[]).unwrap_err();
        assert_eq!(error, LpError::Unbounded);
    }

    #[test]
    fn unsafe_positive_pivot_is_never_certified_as_unboundedness() {
        let a = [1e-14, 1.0];
        let result = minimize(&[-1.0, 0.0], &a, &[1.0]);
        assert_eq!(result.unwrap_err(), LpError::NumericalFailure);
    }

    #[test]
    fn ambiguous_phase_one_reduced_cost_is_not_certified_as_infeasibility() {
        let delta = 5e-10;
        let a = [1.0, -1.0, -1.0 + delta, 1.0];
        let b = [0.0, delta];
        let error = minimize(&[0.0, 0.0], &a, &b).unwrap_err();
        assert_eq!(error, LpError::NumericalFailure);
    }

    #[test]
    fn detects_infeasibility_below_the_primal_absolute_tolerance() {
        let a = [1.0, 1.0];
        let error = minimize(&[0.0], &a, &[0.0, 1e-14]).unwrap_err();
        assert_eq!(error, LpError::Infeasible);
    }

    #[test]
    fn catastrophic_cancellation_is_never_certified_as_an_optimum() {
        let a = [
            2.0, -4.0, 4.0, -4.0, -4.0, -1.0, 3.0, -1.0, -1.0, -2.0, -3.0, -1.0, 0.0, 1.0, -3.0,
        ];
        let b = [1.0, 2.0, 0.0];
        let c = [-5.0, -2.0, 0.0, 2.0, -3.0];
        let error = minimize(&c, &a, &b).unwrap_err();
        assert!(matches!(error, LpError::Infeasible | LpError::NumericalFailure));
    }

    #[test]
    fn rejects_invalid_dimensions() {
        assert_eq!(minimize(&[1.0], &[], &[1.0]).unwrap_err(), LpError::InvalidInput);
        assert_eq!(
            minimize(&[1.0, 2.0], &[1.0], &[1.0]).unwrap_err(),
            LpError::InvalidInput
        );
    }

    #[test]
    fn rejects_negative_rhs() {
        assert_eq!(minimize(&[1.0], &[1.0], &[-1.0]).unwrap_err(), LpError::InvalidInput);
    }

    #[test]
    fn rejects_non_finite_input() {
        assert_eq!(
            minimize(&[f64::NAN], &[1.0], &[1.0]).unwrap_err(),
            LpError::InvalidInput
        );
        assert_eq!(
            minimize(&[1.0], &[f64::INFINITY], &[1.0]).unwrap_err(),
            LpError::InvalidInput
        );
        assert_eq!(
            minimize(&[1.0], &[1.0], &[f64::NAN]).unwrap_err(),
            LpError::InvalidInput
        );
    }

    #[test]
    fn returned_objective_matches_the_primal_dot_product() {
        let a = [1.0, 1.0, 1.0, 0.0, 2.0, 1.0, 0.0, 1.0];
        let b = [5.0, 8.0];
        let c = [-3.0, -2.0, 0.0, 0.0];
        let (objective, solution) = minimize(&c, &a, &b).unwrap();
        let direct: f64 = c.iter().zip(&solution).map(|(&x, &y)| x * y).sum();
        assert_close(objective, direct);
        assert_primal(&a, &b, &solution);
    }
}
