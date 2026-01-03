use crate::climate::utils::ClimateError;
use ndarray::{Array1, ArrayView1};
use numpy::{PyArray1, PyReadonlyArray1, ToPyArray};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn simulate(
    params: ArrayView1<f64>,
    precipitation: ArrayView1<f64>,
    pet: ArrayView1<f64>,
) -> Result<Array1<f64>, ClimateError> {
    let [x1, x2, x3, x4]: [f64; 4] = params
        .as_slice()
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| ClimateError::ParamsMismatch(4, params.len()))?;

    if precipitation.len() != pet.len() {
        return Err(ClimateError::LengthMismatch(
            precipitation.len(),
            pet.len(),
        ));
    }

    let precipitation_ = precipitation.as_slice().unwrap();
    let pet_ = pet.as_slice().unwrap();

    let mut discharge: Vec<f64> = vec![];

    let mut production_store = x1 / 2.;
    let mut routing_store = x3 / 2.;
    let mut routing_precipitation: f64;
    let mut discharge_: f64;

    let unit_hydrographs = create_unit_hydrographs(x4);
    let mut hydrographs = (
        vec![0.0; unit_hydrographs.0.len()],
        vec![0.0; unit_hydrographs.1.len()],
    );

    for t in 0..precipitation_.len() {
        (production_store, routing_precipitation) = update_production(
            production_store,
            precipitation_[t],
            pet_[t],
            x1,
        );
        (routing_store, hydrographs, discharge_) = update_routing(
            routing_store,
            hydrographs,
            &unit_hydrographs,
            routing_precipitation,
            x2,
            x3,
        );
        discharge.push(discharge_);
    }

    Ok(Array1::from_vec(discharge))
}

fn create_unit_hydrographs(x4: f64) -> (Vec<f64>, Vec<f64>) {
    let s1 = |i: f64| -> f64 {
        if i == 0. {
            0.
        } else if i >= x4 {
            1.
        } else {
            (i / x4).powf(1.25)
        }
    };

    let s2 = |i: f64| -> f64 {
        if i == 0. {
            0.
        } else if i >= 2. * x4 {
            1.
        } else if i < x4 {
            0.5 * (i / x4).powf(1.25)
        } else {
            1. - 0.5 * (2. - i / x4).powf(1.25)
        }
    };

    let unit_hydrograph_1 = (1..=x4.ceil() as usize)
        .map(|i| s1(i as f64) - s1(i as f64 - 1.))
        .collect();
    let unit_hydrograph_2 = (1..=(2. * x4).ceil() as usize)
        .map(|i| s2(i as f64) - s2(i as f64 - 1.))
        .collect();

    (unit_hydrograph_1, unit_hydrograph_2)
}

fn update_production(
    store: f64,
    precipitation: f64,
    pet: f64,
    x1: f64,
) -> (f64, f64) {
    let (store, store_precipitation, net_precipitation) =
        if precipitation > pet {
            let net_precipitation = precipitation - pet;
            // only calculate terms once
            let tmp_term_1 = store / x1;
            let tmp_term_2 = (net_precipitation / x1).tanh();

            let store_precipitation =
                x1 * (1. - tmp_term_1 * tmp_term_1) * tmp_term_2
                    / (1. + tmp_term_1 * tmp_term_2);
            (
                store + store_precipitation,
                store_precipitation,
                net_precipitation,
            )
        } else if precipitation < pet {
            let net_pet = pet - precipitation;
            // only calculate terms once
            let tmp_term_1 = store / x1;
            let tmp_term_2 = (net_pet / x1).tanh();
            let evapotranspiration = store * (2. - tmp_term_1) * tmp_term_2
                / (1. + (1. - tmp_term_1) * tmp_term_2);
            (store - evapotranspiration, 0., 0.)
        } else {
            (store, 0., 0.)
        };

    let (store, percolation) = if x1 / store > 1e-3 {
        let percolation =
            store * (1. - (1. + (4. / 21. * store / x1).powi(4)).powf(-0.25));
        (store - percolation, percolation)
    } else {
        (store, 0.)
    };

    let routing_precipitation =
        net_precipitation - store_precipitation + percolation;

    (store, routing_precipitation)
}

fn update_routing(
    store: f64,
    hydrographs: (Vec<f64>, Vec<f64>),
    unit_hydrographs: &(Vec<f64>, Vec<f64>),
    routing_precipitation: f64,
    x2: f64,
    x3: f64,
) -> (f64, (Vec<f64>, Vec<f64>), f64) {
    let hydrographs = update_hydrographs(
        routing_precipitation,
        hydrographs,
        unit_hydrographs,
    );

    let q9 = hydrographs.0[0];
    let q1 = hydrographs.1[0];

    let groundwater_exchange = x2 * (store / x3).powf(3.5);

    let store = (store + q9 + groundwater_exchange).max(1e-3 * x3);

    let routed_flow = store * (1. - (1. + (store / x3).powi(4)).powf(-0.25));
    let store = store - routed_flow;

    let direct_flow = (q1 + groundwater_exchange).max(0.);

    let total_flow = routed_flow + direct_flow;

    (store, hydrographs, total_flow)
}

fn update_hydrographs(
    routing_precipitation: f64,
    hydrographs: (Vec<f64>, Vec<f64>),
    unit_hydrographs: &(Vec<f64>, Vec<f64>),
) -> (Vec<f64>, Vec<f64>) {
    let hydrograph_1 = hydrographs.0[1..] // shift existing water forward in time
        .iter()
        .chain(std::iter::once(&0.0)) // clear last position
        .zip(&unit_hydrographs.0)
        .map(|(h, u)| h + 0.9 * routing_precipitation * u)
        .collect();

    let hydrograph_2 = hydrographs.1[1..] // shift existing water forward in time
        .iter()
        .chain(std::iter::once(&0.0)) // clear last position
        .zip(&unit_hydrographs.1)
        .map(|(h, u)| h + 0.1 * routing_precipitation * u)
        .collect();

    (hydrograph_1, hydrograph_2)
}

#[gen_stub_pyfunction(module = "hydro_rs.climate.gr4j")]
#[pyfunction]
#[pyo3(name = "simulate")]
pub fn py_simulate<'py>(
    py: Python<'py>,
    params: PyReadonlyArray1<'py, f64>,
    precipitation: PyReadonlyArray1<'py, f64>,
    pet: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    Ok(
        simulate(params.as_array(), precipitation.as_array(), pet.as_array())?
            .to_pyarray(py),
    )
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "gr4j")?;
    m.add_function(wrap_pyfunction!(py_simulate, &m)?)?;
    Ok(m)
}
