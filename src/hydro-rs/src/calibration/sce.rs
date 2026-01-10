#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

use std::str::FromStr;

use ndarray::{s, Array1, Array2, ArrayView1, ArrayView2, Axis};
use ndarray_rand::rand_distr::Uniform;
use ndarray_rand::RandomExt;
use numpy::{PyArray1, PyReadonlyArray1, ToPyArray};
use pyo3::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use crate::calibration::utils::{CalibrationParams, Objective};
use crate::climate;
use crate::metrics::{calculate_kge, calculate_nse, calculate_rmse};
use crate::model::{
    compose_init, compose_simulate, Data, Error, Metadata, PyData, PyMetadata,
    SimulateFn,
};
use crate::snow;

struct SceParams {
    pub population: Array2<f64>,
    pub objectives: Array2<f64>,
    pub criteria: Array1<f64>,
    pub n_calls: usize,
    pub n_complexes: usize,
    pub n_per_complex: usize,
    pub n_simplex: usize,
    pub n_evolution_steps: usize,
    pub k_stop: usize,
    pub p_convergence_threshold: f64,
    pub geometric_range_threshold: f64,
    pub max_evaluations: usize,
}

#[pyclass(module = "hydro_rs.calibration.sce", unsendable)]
pub struct Sce {
    calibration_params: CalibrationParams,
    sce_params: SceParams,
}

impl Sce {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        climate_model: &str,
        snow_model: Option<&str>,
        objective: Objective,
        n_complexes: usize,
        k_stop: usize,
        p_convergence_threshold: f64,
        geometric_range_threshold: f64,
        max_evaluations: usize,
        seed: u64,
    ) -> Result<Self, Error> {
        let (simulate, params, bounds): (SimulateFn, _, _) =
            if let Some(snow_model) = snow_model {
                let (snow_init, snow_simulate) = snow::get_model(snow_model)?;
                let (climate_init, climate_simulate) =
                    climate::get_model(climate_model)?;

                let init = compose_init(snow_init, climate_init);
                let (defaults, bounds, n_snow_params) = init();
                let simulate = compose_simulate(
                    snow_simulate,
                    climate_simulate,
                    n_snow_params,
                );
                (simulate, defaults, bounds)
            } else {
                let (init, simulate) = climate::get_model(climate_model)?;
                let (defaults, bounds) = init();
                (Box::new(simulate), defaults, bounds)
            };

        let n_params = params.len();
        let n_per_complex = 2 * n_params + 1;
        let n_simplex = n_params + 1;
        let population_size = n_complexes * n_per_complex;
        let n_evolution_steps = 2 * n_params + 1;

        let lower_bounds: Array1<f64> = bounds.column(0).to_owned();
        let upper_bounds: Array1<f64> = bounds.column(1).to_owned();

        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let population = generate_initial_population(
            population_size,
            &lower_bounds,
            &upper_bounds,
            &mut rng,
        );
        let objectives: Array2<f64> =
            Array2::from_shape_fn((population_size, 3), |(_, j)| {
                if j == 0 {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                }
            });

        let criteria: Array1<f64> = Array1::from_vec(vec![]);
        let params = population.row(0).to_owned();

        let calibration_params = CalibrationParams {
            params,
            simulate,
            lower_bounds,
            upper_bounds,
            objective,
            rng,
            done: false,
        };
        let sce_params = SceParams {
            population,
            objectives,
            criteria,
            n_calls: 0,
            n_complexes,
            n_per_complex,
            n_simplex,
            n_evolution_steps,
            k_stop,
            p_convergence_threshold,
            geometric_range_threshold,
            max_evaluations,
        };

        Ok(Sce {
            calibration_params,
            sce_params,
        })
    }

    pub fn init<'a>(
        &mut self,
        data: Data<'a>,
        metadata: &Metadata<'a>,
        observations: ArrayView1<f64>,
    ) -> Result<(), Error> {
        let objective_idx = match self.calibration_params.objective {
            Objective::Rmse => 0,
            Objective::Nse => 1,
            Objective::Kge => 2,
        };

        let population = generate_initial_population(
            self.sce_params.population.nrows(),
            &self.calibration_params.lower_bounds,
            &self.calibration_params.upper_bounds,
            &mut self.calibration_params.rng,
        );

        let (population, objectives) = evaluate_initial_population(
            &self.calibration_params.simulate,
            data,
            metadata,
            observations,
            population,
            self.calibration_params.objective,
        )?;

        self.sce_params.criteria =
            Array1::from_vec(vec![objectives[[0, objective_idx]]]);
        self.calibration_params.params = population.row(0).to_owned();
        self.sce_params.population = population;
        self.sce_params.objectives = objectives;

        Ok(())
    }

    pub fn step<'a>(
        &mut self,
        data: Data<'a>,
        metadata: &Metadata<'a>,
        observations: ArrayView1<f64>,
    ) -> Result<(bool, Array1<f64>, Array1<f64>, Array1<f64>), Error> {
        if self.calibration_params.done {
            // Recompute simulation for the final result (only happens once when done)
            let best_simulation = (self.calibration_params.simulate)(
                self.calibration_params.params.view(),
                data,
                metadata,
            )?;
            return Ok((
                true,
                self.calibration_params.params.clone(),
                best_simulation,
                self.sce_params.objectives.row(0).to_owned(),
            ));
        }

        let (objective_idx, is_minimization) =
            match self.calibration_params.objective {
                Objective::Rmse => (0, true),
                Objective::Nse => (1, false),
                Objective::Kge => (2, false),
            };

        let (mut complexes, mut complex_objectives) = partition_into_complexes(
            std::mem::take(&mut self.sce_params.population),
            std::mem::take(&mut self.sce_params.objectives),
            self.sce_params.n_complexes,
        );

        let n_calls = evolve_complexes(
            &mut complexes,
            &mut complex_objectives,
            self.calibration_params.lower_bounds.view(),
            self.calibration_params.upper_bounds.view(),
            &self.calibration_params.simulate,
            data,
            metadata,
            observations,
            objective_idx,
            is_minimization,
            self.sce_params.n_calls,
            self.sce_params.n_complexes,
            self.sce_params.n_per_complex,
            self.sce_params.n_simplex,
            self.sce_params.n_evolution_steps,
            &mut self.calibration_params.rng,
        )?;

        let (population, objectives) = merge_complexes(
            complexes,
            complex_objectives,
            objective_idx,
            is_minimization,
        );

        let best_objective = objectives[[0, objective_idx]];

        let gnrng = compute_normalized_geometric_range(
            population.view(),
            self.calibration_params.lower_bounds.view(),
            self.calibration_params.upper_bounds.view(),
        );

        self.sce_params
            .criteria
            .append(Axis(0), Array1::from_elem(1, best_objective).view())
            .unwrap();

        let criteria_change = if self.sce_params.criteria.len()
            >= self.sce_params.k_stop
        {
            let recent = self
                .sce_params
                .criteria
                .slice(s![-(self.sce_params.k_stop as isize)..]);
            let mean_recent = recent.iter().map(|x| x.abs()).sum::<f64>()
                / self.sce_params.k_stop as f64;
            if mean_recent > 0.0 {
                (self.sce_params.criteria[self.sce_params.criteria.len() - 1]
                    - self.sce_params.criteria[self.sce_params.criteria.len()
                        - self.sce_params.k_stop])
                    .abs()
                    * 100.0
                    / mean_recent
            } else {
                f64::INFINITY
            }
        } else {
            f64::INFINITY
        };

        self.calibration_params.done = n_calls > self.sce_params.max_evaluations
            || gnrng < self.sce_params.geometric_range_threshold
            || criteria_change < self.sce_params.p_convergence_threshold;
        self.calibration_params.params = population.row(0).to_owned();
        self.sce_params.n_calls = n_calls;

        // Compute simulation once and return directly (no clone)
        let best_simulation = (self.calibration_params.simulate)(
            self.calibration_params.params.view(),
            data,
            metadata,
        )?;
        let best_objectives = objectives.row(0).to_owned();

        self.sce_params.population = population;
        self.sce_params.objectives = objectives;

        Ok((
            self.calibration_params.done,
            self.calibration_params.params.clone(),
            best_simulation,
            best_objectives,
        ))
    }
}

#[pymethods]
impl Sce {
    #[new]
    pub fn py_new(
        climate_model: &str,
        snow_model: Option<&str>,
        objective: &str,
        n_complexes: usize,
        k_stop: usize,
        p_convergence_threshold: f64,
        geometric_range_threshold: f64,
        max_evaluations: usize,
        seed: u64,
    ) -> PyResult<Self> {
        let objective = Objective::from_str(objective)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Sce::new(
            climate_model,
            snow_model,
            objective,
            n_complexes,
            k_stop,
            p_convergence_threshold,
            geometric_range_threshold,
            max_evaluations,
            seed,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    #[pyo3(name = "init")]
    pub fn py_init(
        &mut self,
        data: PyData<'_>,
        metadata: PyMetadata<'_>,
        observations: PyReadonlyArray1<'_, f64>,
    ) -> PyResult<()> {
        self.init(
            data.as_data().map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?,
            &metadata.as_metadata(),
            observations.as_array(),
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    #[pyo3(name = "step")]
    pub fn py_step<'py>(
        &mut self,
        py: Python<'py>,
        data: PyData<'_>,
        metadata: PyMetadata<'_>,
        observations: PyReadonlyArray1<'_, f64>,
    ) -> PyResult<(
        bool,
        Bound<'py, PyArray1<f64>>,
        Bound<'py, PyArray1<f64>>,
        Bound<'py, PyArray1<f64>>,
    )> {
        let (done, best_params, simulation, objectives) = self
            .step(
                data.as_data().map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(e.to_string())
                })?,
                &metadata.as_metadata(),
                observations.as_array(),
            )
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?;
        Ok((
            done,
            best_params.to_pyarray(py),
            simulation.to_pyarray(py),
            objectives.to_pyarray(py),
        ))
    }
}

fn generate_initial_population(
    population_size: usize,
    lower_bounds: &Array1<f64>,
    upper_bounds: &Array1<f64>,
    rng: &mut ChaCha8Rng,
) -> Array2<f64> {
    let n_params = lower_bounds.len();

    let random_values: Array2<f64> = Array2::random_using(
        (population_size, n_params),
        Uniform::new(0., 1.).unwrap(),
        rng,
    );

    let range = upper_bounds - lower_bounds;
    let mut population = &random_values * &range + lower_bounds;

    let initial_point: Array1<f64> = Array1::from_iter(
        lower_bounds
            .iter()
            .zip(upper_bounds)
            .map(|(l, u)| (l + u) / 2.),
    );

    population.row_mut(0).assign(&initial_point);

    population
}

fn evaluate_initial_population(
    simulate: &SimulateFn,
    data: Data,
    metadata: &Metadata,
    observations: ArrayView1<f64>,
    mut population: Array2<f64>,
    objective: Objective,
) -> Result<(Array2<f64>, Array2<f64>), Error> {
    let n_population = population.nrows();
    let mut objectives = Array2::<f64>::zeros((n_population, 3));

    let results: Vec<Result<Array1<f64>, Error>> = (0..n_population)
        .into_par_iter()
        .map(|i| {
            let params = population.row(i);
            let simulation = simulate(params, data, metadata)?;
            evaluate_simulation(observations, simulation.view())
        })
        .collect();
    for (i, result) in results.into_iter().enumerate() {
        objectives.row_mut(i).assign(&result?);
    }

    let (objective_idx, is_minimization) = match objective {
        Objective::Rmse => (0, true),
        Objective::Nse => (1, false),
        Objective::Kge => (2, false),
    };

    sort_population(
        &mut population,
        &mut objectives,
        objective_idx,
        is_minimization,
    );

    Ok((population, objectives))
}

fn evaluate_simulation(
    observations: ArrayView1<f64>,
    simulations: ArrayView1<f64>,
) -> Result<Array1<f64>, Error> {
    Ok(Array1::from_vec(vec![
        calculate_rmse(observations, simulations)?,
        calculate_nse(observations, simulations)?,
        calculate_kge(observations, simulations)?,
    ]))
}

fn sort_population(
    population: &mut Array2<f64>,
    objectives: &mut Array2<f64>,
    objective_idx: usize,
    is_minimization: bool,
) {
    let mut indices: Vec<usize> = (0..objectives.nrows()).collect();

    if is_minimization {
        indices.sort_by(|&a, &b| {
            objectives[[a, objective_idx]]
                .total_cmp(&objectives[[b, objective_idx]])
        });
    } else {
        indices.sort_by(|&a, &b| {
            objectives[[b, objective_idx]]
                .total_cmp(&objectives[[a, objective_idx]])
        });
    }

    let sorted_population = population.select(Axis(0), &indices);
    let sorted_objectives = objectives.select(Axis(0), &indices);

    *population = sorted_population;
    *objectives = sorted_objectives;
}

fn compute_normalized_geometric_range(
    population: ArrayView2<f64>,
    lower_bounds: ArrayView1<f64>,
    upper_bounds: ArrayView1<f64>,
) -> f64 {
    let bounds = upper_bounds.to_owned() - lower_bounds;
    let maxs = population
        .fold_axis(Axis(0), f64::NEG_INFINITY, |&acc, &x| acc.max(x));
    let mins =
        population.fold_axis(Axis(0), f64::INFINITY, |&acc, &x| acc.min(x));
    let ranges = maxs - mins;
    let normalised_ranges = ranges / bounds;
    normalised_ranges
        .mapv(|x| x.max(1e-10).ln())
        .mean()
        .unwrap_or(0.0)
        .exp()
}

fn partition_into_complexes(
    population: Array2<f64>,
    objectives: Array2<f64>,
    n_complexes: usize,
) -> (Vec<Array2<f64>>, Vec<Array2<f64>>) {
    let n_per_complex = population.nrows() / n_complexes;
    let mut complexes: Vec<Array2<f64>> = vec![];
    let mut complex_objectives: Vec<Array2<f64>> = vec![];

    for igs in 0..n_complexes {
        let k1 = 0..n_per_complex;
        let k2: Vec<usize> = k1.map(|x| x * n_complexes + igs).collect();

        complexes.push(population.select(Axis(0), &k2));
        complex_objectives.push(objectives.select(Axis(0), &k2));
    }

    (complexes, complex_objectives)
}

fn evolve_complexes(
    complexes: &mut [Array2<f64>],
    complex_objectives: &mut [Array2<f64>],
    lower_bounds: ArrayView1<f64>,
    upper_bounds: ArrayView1<f64>,
    simulate: &SimulateFn,
    data: Data,
    metadata: &Metadata,
    observations: ArrayView1<f64>,
    objective_idx: usize,
    is_minimization: bool,
    mut n_calls: usize,
    n_complexes: usize,
    n_per_complex: usize,
    n_simplex: usize,
    n_evolution_steps: usize,
    rng: &mut ChaCha8Rng,
) -> Result<usize, Error> {
    // Sequential evolution (parallel version had convergence issues)
    for igs in 0..n_complexes {
        let cx = &mut complexes[igs];
        let cf = &mut complex_objectives[igs];

        for _ in 0..n_evolution_steps {
            let simplex_indices =
                select_simplex_indices(n_per_complex, n_simplex, rng);
            let mut s = cx.select(Axis(0), &simplex_indices);
            let mut sf = cf.select(Axis(0), &simplex_indices);

            let (snew, fnew, calls_made) = evolve_complex_step(
                s.view(),
                sf.view(),
                lower_bounds,
                upper_bounds,
                simulate,
                data,
                metadata,
                observations,
                objective_idx,
                is_minimization,
                rng,
            )?;
            n_calls += calls_made;

            // replace worst point in simplex
            let last_s_idx = s.nrows() - 1;
            let last_sf_idx = sf.nrows() - 1;
            s.row_mut(last_s_idx).assign(&snew);
            sf.row_mut(last_sf_idx).assign(&fnew);

            // reintegrate simplex into complex
            for (idx, j) in simplex_indices.iter().zip(0..s.nrows()) {
                cx.row_mut(*idx).assign(&s.row(j));
                cf.row_mut(*idx).assign(&sf.row(j));
            }

            sort_population(cx, cf, objective_idx, is_minimization);
        }
    }
    Ok(n_calls)
}

/// Single step of complex evolution (extracted for parallel execution)
fn evolve_complex_step(
    simplex: ArrayView2<f64>,
    simplex_objectives: ArrayView2<f64>,
    lower_bounds: ArrayView1<f64>,
    upper_bounds: ArrayView1<f64>,
    simulate: &SimulateFn,
    data: Data,
    metadata: &Metadata,
    observations: ArrayView1<f64>,
    objective_idx: usize,
    is_minimization: bool,
    rng: &mut ChaCha8Rng,
) -> Result<(Array1<f64>, Array1<f64>, usize), Error> {
    // This is the same logic as evolve_complexes_competitively but returns call count delta
    let alpha = 1.0;
    let beta = 0.5;
    let mut calls = 0;

    let range = &upper_bounds - &lower_bounds;

    let is_worse = |new_val: f64, old_val: f64| -> bool {
        if is_minimization {
            new_val > old_val
        } else {
            new_val < old_val
        }
    };

    // worst point and objective
    let sw = simplex.row(simplex.nrows() - 1);
    let fw = simplex_objectives[[simplex_objectives.nrows() - 1, objective_idx]];

    // centroid excluding worst (all rows except last)
    let ce = simplex
        .slice(s![0..simplex.nrows() - 1, ..])
        .mean_axis(Axis(0))
        .unwrap();

    // reflection
    let mut snew: Array1<f64> = &ce + alpha * (&ce - &sw);

    // check bounds
    let out_of_bounds =
        snew.iter().zip(lower_bounds.iter()).any(|(s, lb)| s < lb)
            || snew.iter().zip(upper_bounds.iter()).any(|(s, ub)| s > ub);

    if out_of_bounds {
        let random_values: Array1<f64> = Array1::random_using(
            snew.len(),
            Uniform::new(0., 1.).unwrap(),
            rng,
        );
        snew = &random_values * &range + lower_bounds;
    }

    // evaluate reflection point
    let simulation = simulate(snew.view(), data, metadata)?;
    let mut fnew = evaluate_simulation(observations, simulation.view())?;
    calls += 1;

    // if reflection failed (worse than worst), try contraction
    if is_worse(fnew[objective_idx], fw) {
        snew = sw.to_owned() + beta * (&ce - &sw);
        let simulation = simulate(snew.view(), data, metadata)?;
        fnew = evaluate_simulation(observations, simulation.view())?;
        calls += 1;

        // if contraction also failed, use random point
        if is_worse(fnew[objective_idx], fw) {
            let random_values: Array1<f64> = Array1::random_using(
                snew.len(),
                Uniform::new(0., 1.).unwrap(),
                rng,
            );
            snew = &random_values * &range + lower_bounds;
            let simulation = simulate(snew.view(), data, metadata)?;
            fnew = evaluate_simulation(observations, simulation.view())?;
            calls += 1;
        }
    }

    Ok((snew, fnew, calls))
}

fn select_simplex_indices(
    n_per_complex: usize,
    n_simplex: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<usize> {
    let mut indices: Vec<usize> = vec![0]; // Always include best point

    for _ in 1..n_simplex {
        let mut lpos = 0;
        // try to find unique index
        for _ in 0..1000 {
            // triangular distribution (biases toward better points)
            lpos = (n_per_complex as f64 + 0.5
                - ((n_per_complex as f64 + 0.5).powi(2)
                    - (n_per_complex * (n_per_complex + 1)) as f64
                        * rng.random::<f64>())
                .sqrt())
            .floor() as usize;
            if !indices.contains(&lpos) {
                break;
            }
        }
        indices.push(lpos);
    }

    indices.sort_by(|a, b| a.partial_cmp(b).unwrap());
    indices
}

fn merge_complexes(
    complexes: Vec<Array2<f64>>,
    complex_objectives: Vec<Array2<f64>>,
    objective_idx: usize,
    is_minimization: bool,
) -> (Array2<f64>, Array2<f64>) {
    let mut population = ndarray::concatenate(
        Axis(0),
        &complexes.iter().map(|x| x.view()).collect::<Vec<_>>(),
    )
    .unwrap();
    let mut objectives = ndarray::concatenate(
        Axis(0),
        &complex_objectives
            .iter()
            .map(|x| x.view())
            .collect::<Vec<_>>(),
    )
    .unwrap();

    sort_population(
        &mut population,
        &mut objectives,
        objective_idx,
        is_minimization,
    );

    (population, objectives)
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "sce")?;
    m.add_class::<Sce>()?;
    Ok(m)
}
