#![allow(clippy::too_many_arguments)]
use std::str::FromStr;

use ndarray::{s, Array1, Array2, ArrayView1, ArrayView2, Axis};
use ndarray_rand::rand_distr::Uniform;
use ndarray_rand::RandomExt;
use numpy::{PyArray1, PyReadonlyArray1, ToPyArray};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::calibration::utils::{CalibrationError, Objective};
use crate::climate::{get_model, SimulateFn};
use crate::utils::{calculate_kge, calculate_nse, calculate_rmse};

#[gen_stub_pyclass]
#[pyclass]
#[allow(clippy::upper_case_acronyms)]
pub struct SCE {
    model: String,
    lower_bounds: Array1<f64>,
    upper_bounds: Array1<f64>,
    objective: Objective,
    rng: ChaCha8Rng,
    population: Array2<f64>,
    objectives: Array2<f64>,
    criteria: Array1<f64>,
    best_params: Array1<f64>,
    best_simulation: Array1<f64>,
    n_calls: usize,
    done: bool,
    n_complexes: usize,
    n_per_complex: usize,
    n_simplex: usize,
    n_evolution_steps: usize,
    k_stop: usize,
    p_convergence_threshold: f64,
    geometric_range_threshold: f64,
    max_evaluations: usize,
}

impl SCE {
    pub fn new(
        model: &str,
        param_bounds: Vec<(f64, f64)>,
        objective: Objective,
        n_complexes: usize,
        k_stop: usize,
        p_convergence_threshold: f64,
        geometric_range_threshold: f64,
        max_evaluations: usize,
        seed: u64,
    ) -> Result<Self, CalibrationError> {
        let n_params = param_bounds.len();
        let n_per_complex = 2 * n_params + 1;
        let n_simplex = n_params + 1;
        let population_size = n_complexes * n_per_complex;
        let n_evolution_steps = 2 * n_params + 1;

        let lower_bounds: Array1<f64> =
            Array1::from_iter(param_bounds.iter().map(|(min, _)| *min));
        let upper_bounds: Array1<f64> =
            Array1::from_iter(param_bounds.iter().map(|(_, max)| *max));

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

        let criteria: Array1<f64> = Array1::default(0);
        let best_params = population.row(0).to_owned();
        let best_simulation: Array1<f64> = Array1::default(0);

        Ok(SCE {
            model: model.to_string(),
            lower_bounds,
            upper_bounds,
            objective,
            rng,
            population,
            objectives,
            criteria,
            best_params,
            best_simulation,
            n_calls: 0,
            done: false,
            n_complexes,
            n_per_complex,
            n_simplex,
            n_evolution_steps,
            k_stop,
            p_convergence_threshold,
            geometric_range_threshold,
            max_evaluations,
        })
    }

    pub fn init(
        &mut self,
        precipitation: ArrayView1<f64>,
        pet: ArrayView1<f64>,
        observations: ArrayView1<f64>,
    ) -> Result<(), CalibrationError> {
        let model_fn =
            get_model(&self.model).map_err(CalibrationError::InvalidModel)?;
        let objective_idx = match self.objective {
            Objective::Rmse => 0,
            Objective::Nse => 1,
            Objective::Kge => 2,
        };

        let population = generate_initial_population(
            self.population.nrows(),
            &self.lower_bounds,
            &self.upper_bounds,
            &mut self.rng,
        );

        let (population, objectives) = evaluate_initial_population(
            model_fn,
            precipitation,
            pet,
            observations,
            population,
            self.objective,
        )?;

        self.criteria = Array1::from_vec(vec![objectives[[0, objective_idx]]]);
        self.best_params = population.row(0).to_owned();
        self.population = population;
        self.objectives = objectives;
        self.best_simulation =
            model_fn(self.best_params.view(), precipitation, pet)?;

        Ok(())
    }

    pub fn step(
        &mut self,
        precipitation: ArrayView1<f64>,
        pet: ArrayView1<f64>,
        observations: ArrayView1<f64>,
    ) -> Result<(), CalibrationError> {
        if self.done {
            return Ok(());
        }

        let model_fn =
            get_model(&self.model).map_err(CalibrationError::InvalidModel)?;
        let (objective_idx, is_minimization) = match self.objective {
            Objective::Rmse => (0, true),
            Objective::Nse => (1, false),
            Objective::Kge => (2, false),
        };

        // partition into complexes
        let (complexes, complex_objectives) = partition_into_complexes(
            std::mem::take(&mut self.population),
            std::mem::take(&mut self.objectives),
            self.n_complexes,
        );

        // Evolve each complex
        let (complexes, complex_objectives, n_calls) = evolve_complexes(
            complexes,
            complex_objectives,
            self.lower_bounds.view(),
            self.upper_bounds.view(),
            model_fn,
            precipitation,
            pet,
            observations,
            objective_idx,
            is_minimization,
            self.n_calls,
            self.n_complexes,
            self.n_per_complex,
            self.n_simplex,
            self.n_evolution_steps,
            &mut self.rng,
        )?;

        let (population, objectives) = merge_complexes(
            complexes,
            complex_objectives,
            objective_idx,
            is_minimization,
        );

        let best_objective = objectives[[0, objective_idx]];

        // Compute convergence metrics
        let gnrng = compute_normalized_geometric_range(
            population.view(),
            self.lower_bounds.view(),
            self.upper_bounds.view(),
        );

        self.criteria
            .append(Axis(0), Array1::from_elem(1, best_objective).view())
            .unwrap();

        let criteria_change = if self.criteria.len() >= self.k_stop {
            let recent = self.criteria.slice(s![-(self.k_stop as isize)..]);
            let mean_recent = recent.iter().map(|x| x.abs()).sum::<f64>()
                / self.k_stop as f64;
            if mean_recent > 0.0 {
                (self.criteria[self.criteria.len() - 1]
                    - self.criteria[self.criteria.len() - self.k_stop])
                    .abs()
                    * 100.0
                    / mean_recent
            } else {
                f64::INFINITY
            }
        } else {
            f64::INFINITY
        };

        self.done = n_calls > self.max_evaluations
            || gnrng < self.geometric_range_threshold
            || criteria_change < self.p_convergence_threshold;
        self.best_params = population.row(0).to_owned();
        self.n_calls = n_calls;
        self.population = population;
        self.objectives = objectives;
        self.best_simulation =
            model_fn(self.best_params.view(), precipitation, pet)?;

        Ok(())
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
    model: SimulateFn,
    precipitation: ArrayView1<f64>,
    pet: ArrayView1<f64>,
    observations: ArrayView1<f64>,
    population: Array2<f64>,
    objective: Objective,
) -> Result<(Array2<f64>, Array2<f64>), CalibrationError> {
    let n_population = population.nrows();
    let mut objectives = Array2::<f64>::zeros((n_population, 3));

    for i in 0..n_population {
        let params = population.row(i);
        let simulations = model(params, precipitation, pet)?;
        objectives
            .row_mut(i)
            .assign(&evaluate_simulation(observations, simulations.view())?);
    }

    let (objective_idx, is_minimization) = match objective {
        Objective::Rmse => (0, true),
        Objective::Nse => (1, false),
        Objective::Kge => (2, false),
    };

    let (population, objectives) = sort_population(
        population,
        objectives,
        objective_idx,
        is_minimization,
    );

    Ok((population, objectives))
}

fn evaluate_simulation(
    observations: ArrayView1<f64>,
    simulations: ArrayView1<f64>,
) -> Result<Array1<f64>, CalibrationError> {
    Ok(Array1::from_vec(vec![
        calculate_rmse(observations, simulations)?,
        calculate_nse(observations, simulations)?,
        calculate_kge(observations, simulations)?,
    ]))
}

fn sort_population(
    population: Array2<f64>,
    objectives: Array2<f64>,
    objective_idx: usize,
    is_minimization: bool,
) -> (Array2<f64>, Array2<f64>) {
    let mut indices: Vec<usize> = (0..objectives.nrows()).collect();

    if is_minimization {
        indices.sort_by(|&a, &b| {
            objectives[[a, objective_idx]].total_cmp(&objectives[[b, objective_idx]])
        });
    } else {
        indices.sort_by(|&a, &b| {
            objectives[[b, objective_idx]].total_cmp(&objectives[[a, objective_idx]])
        });
    }

    let sorted_population = population.select(Axis(0), &indices);
    let sorted_objectives = objectives.select(Axis(0), &indices);

    (sorted_population, sorted_objectives)
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

#[allow(clippy::type_complexity)]
fn evolve_complexes(
    mut complexes: Vec<Array2<f64>>,
    mut complex_objectives: Vec<Array2<f64>>,
    lower_bounds: ArrayView1<f64>,
    upper_bounds: ArrayView1<f64>,
    model: SimulateFn,
    precipitation: ArrayView1<f64>,
    pet: ArrayView1<f64>,
    observations: ArrayView1<f64>,
    objective_idx: usize,
    is_minimization: bool,
    mut n_calls: usize,
    n_complexes: usize,
    n_per_complex: usize,
    n_simplex: usize,
    n_evolution_steps: usize,
    rng: &mut ChaCha8Rng,
) -> Result<(Vec<Array2<f64>>, Vec<Array2<f64>>, usize), CalibrationError> {
    for igs in 0..n_complexes {
        let mut cx = complexes[igs].clone();
        let mut cf = complex_objectives[igs].clone();

        // evolve complex n_evolution_steps times
        for _ in 0..n_evolution_steps {
            let simplex_indices =
                select_simplex_indices(n_per_complex, n_simplex, rng);
            let mut s = cx.select(Axis(0), &simplex_indices);
            let mut sf = cf.select(Axis(0), &simplex_indices);

            let (snew, fnew, new_n_calls) = evolve_complexes_competitively(
                s.view(),
                sf.view(),
                lower_bounds,
                upper_bounds,
                model,
                precipitation,
                pet,
                observations,
                objective_idx,
                is_minimization,
                n_calls,
                rng,
            )?;
            n_calls = new_n_calls;

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

            // Sort complex
            (cx, cf) = sort_population(cx, cf, objective_idx, is_minimization);
        }

        complexes[igs] = cx;
        complex_objectives[igs] = cf;
    }
    Ok((complexes, complex_objectives, n_calls))
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

fn evolve_complexes_competitively(
    simplex: ArrayView2<f64>,
    simplex_objectives: ArrayView2<f64>,
    lower_bounds: ArrayView1<f64>,
    upper_bounds: ArrayView1<f64>,
    model: SimulateFn,
    precipitation: ArrayView1<f64>,
    pet: ArrayView1<f64>,
    observations: ArrayView1<f64>,
    objective_idx: usize,
    is_minimization: bool,
    n_calls: usize,
    rng: &mut ChaCha8Rng,
) -> Result<(Array1<f64>, Array1<f64>, usize), CalibrationError> {
    let alpha = 1.0;
    let beta = 0.5;

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
    let fw =
        simplex_objectives[[simplex_objectives.nrows() - 1, objective_idx]];

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
    let simulation = model(snew.view(), precipitation, pet)?;
    let mut fnew = evaluate_simulation(observations, simulation.view())?;
    let mut n_calls = n_calls + 1;

    // if reflection failed (worse than worst), try contraction
    if is_worse(fnew[objective_idx], fw) {
        snew = sw.to_owned() + beta * (&ce - &sw);
        let simulation = model(snew.view(), precipitation, pet)?;
        fnew = evaluate_simulation(observations, simulation.view())?;
        n_calls += 1;

        // if contraction also failed, use random point
        if is_worse(fnew[objective_idx], fw) {
            let random_values: Array1<f64> = Array1::random_using(
                snew.len(),
                Uniform::new(0., 1.).unwrap(),
                rng,
            );
            snew = &random_values * &range + lower_bounds;
            let simulation = model(snew.view(), precipitation, pet)?;
            fnew = evaluate_simulation(observations, simulation.view())?;
            n_calls += 1;
        }
    }

    Ok((snew, fnew, n_calls))
}

fn merge_complexes(
    complexes: Vec<Array2<f64>>,
    complex_objectives: Vec<Array2<f64>>,
    objective_idx: usize,
    is_minimization: bool,
) -> (Array2<f64>, Array2<f64>) {
    let population = ndarray::concatenate(
        Axis(0),
        &complexes.iter().map(|x| x.view()).collect::<Vec<_>>(),
    )
    .unwrap();
    let objectives = ndarray::concatenate(
        Axis(0),
        &complex_objectives
            .iter()
            .map(|x| x.view())
            .collect::<Vec<_>>(),
    )
    .unwrap();

    sort_population(population, objectives, objective_idx, is_minimization)
}

#[gen_stub_pymethods]
#[pymethods]
impl SCE {
    #[new]
    pub fn py_new(
        model: &str,
        param_bounds: Vec<(f64, f64)>,
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
        SCE::new(
            model,
            param_bounds,
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
        precipitation: PyReadonlyArray1<'_, f64>,
        pet: PyReadonlyArray1<'_, f64>,
        observations: PyReadonlyArray1<'_, f64>,
    ) -> PyResult<()> {
        self.init(
            precipitation.as_array(),
            pet.as_array(),
            observations.as_array(),
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    #[pyo3(name = "step")]
    pub fn py_step(
        &mut self,
        precipitation: PyReadonlyArray1<'_, f64>,
        pet: PyReadonlyArray1<'_, f64>,
        observations: PyReadonlyArray1<'_, f64>,
    ) -> PyResult<()> {
        self.step(
            precipitation.as_array(),
            pet.as_array(),
            observations.as_array(),
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    #[getter]
    pub fn best_params<'py>(
        &self,
        py: Python<'py>,
    ) -> Bound<'py, PyArray1<f64>> {
        self.best_params.to_pyarray(py)
    }

    #[getter]
    pub fn done(&self) -> bool {
        self.done
    }

    #[getter]
    pub fn last_objectives<'py>(
        &self,
        py: Python<'py>,
    ) -> Bound<'py, PyArray1<f64>> {
        self.objectives.row(0).to_pyarray(py)
    }

    #[getter]
    pub fn last_simulation<'py>(
        &self,
        py: Python<'py>,
    ) -> Bound<'py, PyArray1<f64>> {
        self.best_simulation.to_pyarray(py)
    }
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "sce")?;
    m.add_class::<SCE>()?;
    Ok(m)
}
