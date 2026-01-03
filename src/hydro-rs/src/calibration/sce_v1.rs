use std::str::FromStr;

use ndarray::{s, Array1, Array2, ArrayView1, ArrayView2, Axis};
use ndarray_rand::rand_distr::Uniform;
use ndarray_rand::RandomExt;
use numpy::{PyArray1, PyReadonlyArray1, ToPyArray};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::calibration::utils::{CalibrationError, Objective};
use crate::climate::{get_model, SimulateFn};
use crate::utils::{calculate_kge, calculate_nse, calculate_rmse};

pub fn run_calibration(
    model: SimulateFn,
    precipitation: ArrayView1<f64>,
    pet: ArrayView1<f64>,
    observations: ArrayView1<f64>,
    possible_params: Vec<(f64, f64)>,
    objective: Objective,
    n_complexes: usize,
    n_per_complex: usize,
    n_simplex: usize,
    n_evolution_steps: usize,
    min_complexes: usize,
    max_evaluations: usize,
    kstop: usize,
    pcento: f64,
    peps: f64,
    seed: u64,
) -> Result<(Array1<f64>, Array1<f64>), CalibrationError> {
    let (
        mut params,
        lower_bounds,
        upper_bounds,
        _objective_idx,
        mut n_calls,
        mut population,
        mut objectives,
        mut criteria,
    ) = init_calibration(
        model,
        precipitation,
        pet,
        observations,
        possible_params,
        objective,
        n_complexes,
        n_per_complex,
        seed,
    )?;

    let max_iter = 1_000_000;
    for _iteration in 0..max_iter {
        let (
            done,
            new_params,
            new_n_calls,
            new_population,
            new_objectives,
            new_criteria,
        ) = run_calibration_step(
            model,
            precipitation,
            pet,
            observations,
            criteria.view(),
            n_calls,
            lower_bounds.view(),
            upper_bounds.view(),
            objective,
            population,
            objectives,
            min_complexes,
            n_evolution_steps,
            max_evaluations,
            kstop,
            pcento,
            peps,
            n_complexes,
            n_per_complex,
            n_simplex,
            seed,
        )?;

        params = new_params;
        n_calls = new_n_calls;
        population = new_population;
        objectives = new_objectives;
        criteria = new_criteria;

        if done {
            break;
        }
    }

    let best_objectives = objectives.row(0).to_owned();
    Ok((params, best_objectives))
}

pub fn init_calibration(
    model: SimulateFn,
    precipitation: ArrayView1<f64>,
    pet: ArrayView1<f64>,
    observations: ArrayView1<f64>,
    possible_params: Vec<(f64, f64)>,
    objective: Objective,
    n_complexes: usize,
    n_per_complex: usize,
    seed: u64,
) -> Result<
    (
        Array1<f64>,
        Array1<f64>,
        Array1<f64>,
        usize,
        usize,
        Array2<f64>,
        Array2<f64>,
        Array1<f64>,
    ),
    CalibrationError,
> {
    let lower_bounds: Array1<f64> =
        Array1::from_iter(possible_params.iter().map(|(min, _)| *min));
    let upper_bounds: Array1<f64> =
        Array1::from_iter(possible_params.iter().map(|(_, max)| *max));

    let initial_point: Array1<f64> = Array1::from_iter(
        lower_bounds
            .iter()
            .zip(&upper_bounds)
            .map(|(l, u)| (l + u) / 2.),
    );

    let population = generate_initial_population(
        n_complexes,
        n_per_complex,
        &lower_bounds,
        &upper_bounds,
        Some(&initial_point),
        seed,
    );

    let (population, objectives) = evaluate_initial_population(
        model,
        precipitation,
        pet,
        observations,
        population,
        objective,
    )?;

    let objective_idx = match objective {
        Objective::Rmse => 0,
        Objective::Nse => 1,
        Objective::Kge => 2,
    };
    let best_params = population.row(0).to_owned();
    let criteria = Array1::from_vec(vec![objectives[[0, objective_idx]]]);

    Ok((
        best_params,
        lower_bounds,
        upper_bounds,
        objective_idx,
        population.nrows(),
        population,
        objectives,
        criteria,
    ))
}

pub fn run_calibration_step(
    model: SimulateFn,
    precipitation: ArrayView1<f64>,
    pet: ArrayView1<f64>,
    observations: ArrayView1<f64>,
    criteria: ArrayView1<f64>,
    n_calls: usize,
    lower_bounds: ArrayView1<f64>,
    upper_bounds: ArrayView1<f64>,
    objective: Objective,
    population: Array2<f64>,
    objectives: Array2<f64>,
    _min_complexes: usize,
    n_evolution_steps: usize,
    max_evaluations: usize,
    kstop: usize,
    pcento: f64,
    peps: f64,
    n_complexes: usize,
    n_per_complex: usize,
    n_simplex: usize,
    seed: u64,
) -> Result<
    (
        bool,
        Array1<f64>,
        usize,
        Array2<f64>,
        Array2<f64>,
        Array1<f64>,
    ),
    CalibrationError,
> {
    let (objective_idx, is_minimization) = match objective {
        Objective::Rmse => (0, true),
        Objective::Nse => (1, false),
        Objective::Kge => (2, false),
    };

    // partition into complexes
    let (complexes, complex_objectives) =
        partition_into_complexes(population, objectives, n_complexes);

    // Evolve each complex
    let (complexes, complex_objectives, n_calls) = evolve_complexes(
        complexes,
        complex_objectives,
        lower_bounds,
        upper_bounds,
        model,
        precipitation,
        pet,
        observations,
        objective_idx,
        is_minimization,
        n_calls,
        n_complexes,
        n_per_complex,
        n_simplex,
        n_evolution_steps,
        seed,
    )?;

    let (population, objectives) = merge_complexes(
        complexes,
        complex_objectives,
        objective_idx,
        is_minimization,
    );

    let best_params = population.row(0).to_owned();
    let best_objective = objectives[[0, objective_idx]];

    // Compute convergence metrics
    let gnrng = compute_normalized_geometric_range(
        population.view(),
        lower_bounds,
        upper_bounds,
    );

    let mut new_criteria = criteria.to_owned();
    new_criteria
        .append(Axis(0), Array1::from_elem(1, best_objective).view())
        .unwrap();

    let criteria_change = if new_criteria.len() >= kstop {
        let recent = new_criteria.slice(s![-(kstop as isize)..]);
        let mean_recent =
            recent.iter().map(|x| x.abs()).sum::<f64>() / kstop as f64;
        if mean_recent > 0.0 {
            (new_criteria[new_criteria.len() - 1]
                - new_criteria[new_criteria.len() - kstop])
                .abs()
                * 100.0
                / mean_recent
        } else {
            f64::INFINITY
        }
    } else {
        f64::INFINITY
    };

    let done =
        n_calls > max_evaluations || gnrng < peps || criteria_change < pcento;

    Ok((
        done,
        best_params,
        n_calls,
        population,
        objectives,
        new_criteria,
    ))
}

fn generate_initial_population(
    n_complexes: usize,
    n_per_complex: usize,
    lower_bounds: &Array1<f64>,
    upper_bounds: &Array1<f64>,
    initial_point: Option<&Array1<f64>>,
    seed: u64,
) -> Array2<f64> {
    let n_population = n_complexes * n_per_complex;
    let n_params = lower_bounds.len();

    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let random_values: Array2<f64> = Array2::random_using(
        (n_population, n_params),
        Uniform::new(0., 1.).unwrap(),
        &mut rng,
    );

    let range = upper_bounds - lower_bounds;
    let mut population = &random_values * &range + lower_bounds;

    if let Some(point) = initial_point {
        population.row_mut(0).assign(point);
    }

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
        let simulations = model(precipitation, pet, params)?;
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
            objectives[[a, objective_idx]]
                .partial_cmp(&objectives[[b, objective_idx]])
                .unwrap()
        });
    } else {
        indices.sort_by(|&a, &b| {
            objectives[[b, objective_idx]]
                .partial_cmp(&objectives[[a, objective_idx]])
                .unwrap()
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
    let bounds = upper_bounds.to_owned() - &lower_bounds;
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
    let n_per_complex = (population.nrows() / n_complexes) as usize;
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
    seed: u64,
) -> Result<(Vec<Array2<f64>>, Vec<Array2<f64>>, usize), CalibrationError> {
    for igs in 0..n_complexes {
        let mut cx = complexes[igs].clone();
        let mut cf = complex_objectives[igs].clone();

        // evolve complex n_evolution_steps times
        for _ in 0..n_evolution_steps {
            let simplex_indices =
                select_simplex_indices(n_per_complex, n_simplex, seed);
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
                seed,
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
    seed: u64,
) -> Vec<usize> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut indices: Vec<usize> = vec![];

    for _ in 1..n_simplex {
        let mut lpos = 0;
        // try to find unique index
        for _ in 0..1000 {
            // triangular distribution
            lpos = (n_per_complex as f64 + 0.5
                - ((n_per_complex as f64 + 0.5).powi(2)
                    - (n_per_complex * (n_per_complex + 1)) as f64
                    + rng.random::<f64>())
                .sqrt())
            .floor() as usize;
            if indices.iter().any(|x| *x == lpos) {
                continue;
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
    seed: u64,
) -> Result<(Array1<f64>, Array1<f64>, usize), CalibrationError> {
    let alpha = 1.0;
    let beta = 0.5;

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
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
            &mut rng,
        );
        snew = &random_values * &range + &lower_bounds;
    }

    // evaluate reflection point
    let simulation = model(precipitation, pet, snew.view())?;
    let mut fnew = evaluate_simulation(observations, simulation.view())?;
    let mut n_calls = n_calls + 1;

    // if reflection failed (worse than worst), try contraction
    if is_worse(fnew[objective_idx], fw) {
        snew = sw.to_owned() + beta * (&ce - &sw);
        let simulation = model(precipitation, pet, snew.view())?;
        fnew = evaluate_simulation(observations, simulation.view())?;
        n_calls += 1;

        // if contraction also failed, use random point
        if is_worse(fnew[objective_idx], fw) {
            let random_values: Array1<f64> = Array1::random_using(
                snew.len(),
                Uniform::new(0., 1.).unwrap(),
                &mut rng,
            );
            snew = &random_values * &range + &lower_bounds;
            let simulation = model(precipitation, pet, snew.view())?;
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

#[pyclass(module = "hydro_rs.calibration.sce")]
pub struct CalibrationState {
    model_name: String,
    precipitation: Array1<f64>,
    pet: Array1<f64>,
    observations: Array1<f64>,
    params: Array1<f64>,
    lower_bounds: Array1<f64>,
    upper_bounds: Array1<f64>,
    objective: Objective,
    n_calls: usize,
    population: Array2<f64>,
    objectives: Array2<f64>,
    criteria: Array1<f64>,
    n_complexes: usize,
    n_per_complex: usize,
    n_simplex: usize,
    n_evolution_steps: usize,
    min_complexes: usize,
    max_evaluations: usize,
    kstop: usize,
    pcento: f64,
    peps: f64,
    seed: u64,
    done: bool,
}

#[pymethods]
impl CalibrationState {
    #[new]
    #[pyo3(signature = (
        model,
        precipitation,
        pet,
        observations,
        possible_params,
        objective = "kge",
        n_complexes = 5,
        n_per_complex = 10,
        n_simplex = 5,
        n_evolution_steps = 5,
        min_complexes = 2,
        max_evaluations = 10000,
        kstop = 10,
        pcento = 0.1,
        peps = 0.001,
        seed = 42
    ))]
    fn new<'py>(
        model: &str,
        precipitation: PyReadonlyArray1<'py, f64>,
        pet: PyReadonlyArray1<'py, f64>,
        observations: PyReadonlyArray1<'py, f64>,
        possible_params: Vec<(f64, f64)>,
        objective: &str,
        n_complexes: usize,
        n_per_complex: usize,
        n_simplex: usize,
        n_evolution_steps: usize,
        min_complexes: usize,
        max_evaluations: usize,
        kstop: usize,
        pcento: f64,
        peps: f64,
        seed: u64,
    ) -> PyResult<Self> {
        let model_fn =
            get_model(model).map_err(|e| CalibrationError::InvalidModel(e))?;
        let objective_ = Objective::from_str(objective)
            .map_err(|e| CalibrationError::InvalidObjective(e))?;

        let precip_array = precipitation.as_array().to_owned();
        let pet_array = pet.as_array().to_owned();
        let obs_array = observations.as_array().to_owned();

        let (params, lower_bounds, upper_bounds, _objective_idx, n_calls, population, objectives, criteria) =
            init_calibration(
                model_fn,
                precip_array.view(),
                pet_array.view(),
                obs_array.view(),
                possible_params,
                objective_,
                n_complexes,
                n_per_complex,
                seed,
            )?;

        Ok(Self {
            model_name: model.to_string(),
            precipitation: precip_array,
            pet: pet_array,
            observations: obs_array,
            params,
            lower_bounds,
            upper_bounds,
            objective: objective_,
            n_calls,
            population,
            objectives,
            criteria,
            n_complexes,
            n_per_complex,
            n_simplex,
            n_evolution_steps,
            min_complexes,
            max_evaluations,
            kstop,
            pcento,
            peps,
            seed,
            done: false,
        })
    }

    fn step(&mut self) -> PyResult<bool> {
        if self.done {
            return Ok(true);
        }

        let model_fn = get_model(&self.model_name)
            .map_err(|e| CalibrationError::InvalidModel(e))?;

        let (done, new_params, new_n_calls, new_population, new_objectives, new_criteria) =
            run_calibration_step(
                model_fn,
                self.precipitation.view(),
                self.pet.view(),
                self.observations.view(),
                self.criteria.view(),
                self.n_calls,
                self.lower_bounds.view(),
                self.upper_bounds.view(),
                self.objective,
                std::mem::take(&mut self.population),
                std::mem::take(&mut self.objectives),
                self.min_complexes,
                self.n_evolution_steps,
                self.max_evaluations,
                self.kstop,
                self.pcento,
                self.peps,
                self.n_complexes,
                self.n_per_complex,
                self.n_simplex,
                self.seed,
            )?;

        self.params = new_params;
        self.n_calls = new_n_calls;
        self.population = new_population;
        self.objectives = new_objectives;
        self.criteria = new_criteria;
        self.done = done;

        Ok(done)
    }

    #[getter]
    fn is_done(&self) -> bool {
        self.done
    }

    #[getter]
    fn best_params<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.params.to_pyarray(py)
    }

    #[getter]
    fn best_objectives<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.objectives.row(0).to_owned().to_pyarray(py)
    }

    #[getter]
    fn n_evaluations(&self) -> usize {
        self.n_calls
    }
}

#[gen_stub_pyfunction(module = "hydro_rs.calibration.sce")]
#[pyfunction]
#[pyo3(name = "run_calibration")]
#[pyo3(signature = (
    model,
    precipitation,
    pet,
    observations,
    possible_params,
    objective = "kge",
    n_complexes = 5,
    n_per_complex = 10,
    n_simplex = 5,
    n_evolution_steps = 5,
    min_complexes = 2,
    max_evaluations = 10000,
    kstop = 10,
    pcento = 0.1,
    peps = 0.001,
    seed = 42
))]
pub fn py_run_calibration<'py>(
    py: Python<'py>,
    model: &str,
    precipitation: PyReadonlyArray1<'py, f64>,
    pet: PyReadonlyArray1<'py, f64>,
    observations: PyReadonlyArray1<'py, f64>,
    possible_params: Vec<(f64, f64)>,
    objective: &str,
    n_complexes: usize,
    n_per_complex: usize,
    n_simplex: usize,
    n_evolution_steps: usize,
    min_complexes: usize,
    max_evaluations: usize,
    kstop: usize,
    pcento: f64,
    peps: f64,
    seed: u64,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let model_fn =
        get_model(model).map_err(|e| CalibrationError::InvalidModel(e))?;
    let objective_ = Objective::from_str(objective)
        .map_err(|e| CalibrationError::InvalidObjective(e))?;

    let (params, objectives) = run_calibration(
        model_fn,
        precipitation.as_array(),
        pet.as_array(),
        observations.as_array(),
        possible_params,
        objective_,
        n_complexes,
        n_per_complex,
        n_simplex,
        n_evolution_steps,
        min_complexes,
        max_evaluations,
        kstop,
        pcento,
        peps,
        seed,
    )?;

    Ok((params.to_pyarray(py), objectives.to_pyarray(py)))
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "sce")?;
    m.add_class::<CalibrationState>()?;
    m.add_function(wrap_pyfunction!(py_run_calibration, &m)?)?;
    Ok(m)
}
