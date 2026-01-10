use ndarray::{array, Array1, Array2, ArrayView1, Axis};
use numpy::{PyArray1, PyArray2, PyReadonlyArray1, ToPyArray};
use pyo3::prelude::*;

use crate::model::{Data, Error, Metadata, PyData, PyMetadata};

pub fn init() -> (Array1<f64>, Array2<f64>) {
    // corresponds to x1, x2, x3, x4
    let bounds =
        array![[10.0, 1500.0], [-5.0, 3.0], [10.0, 400.0], [0.8, 10.0]];
    let default_values = bounds.sum_axis(Axis(1)) / 2.0;
    (default_values, bounds)
}

pub fn simulate(
    params: ArrayView1<f64>,
    data: Data,
    metadata: &Metadata,
) -> Result<Array1<f64>, Error> {
    let [x1, x2, x3, x4]: [f64; 4] = params
        .as_slice()
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error::ParamsMismatch(4, params.len()))?;

    let precipitation = data.precipitation;
    let pet = data.pet;
    let area = metadata.area * 1000.0 * 1000.0;

    let mut discharge: Vec<f64> = vec![];

    let mut production_store = x1 / 2.;
    let mut routing_store = x3 / 2.;
    let mut routing_precipitation: f64 = 0.0;
    let mut discharge_: f64 = 0.0;

    let unit_hydrographs = create_unit_hydrographs(x4);
    let mut hydrographs = (
        vec![0.0; unit_hydrographs.0.len()],
        vec![0.0; unit_hydrographs.1.len()],
    );

    for t in 0..precipitation.len() {
        update_production(
            &mut production_store,
            &mut routing_precipitation,
            precipitation[t],
            pet[t],
            x1,
        );
        update_routing(
            &mut routing_store,
            &mut hydrographs,
            &mut discharge_,
            &unit_hydrographs,
            routing_precipitation,
            x2,
            x3,
        );
        // discharge_ = discharge_ * 1000.0 * area / (3600.0 * 24.0); // mm/day to m^3/s
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
            (i / x4).powf(2.5)
        }
    };

    let s2 = |i: f64| -> f64 {
        if i == 0. {
            0.
        } else if i >= 2. * x4 {
            1.
        } else if i < x4 {
            0.5 * (i / x4).powf(2.5)
        } else {
            1. - 0.5 * (2. - i / x4).powf(2.5)
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
    store: &mut f64,
    routing_precipitation: &mut f64,
    precipitation: f64,
    pet: f64,
    x1: f64,
) {
    let mut store_precipitation: f64 = 0.0;
    let mut net_precipitation: f64 = 0.0;
    if precipitation > pet {
        net_precipitation = precipitation - pet;
        // only calculate terms once
        let tmp_term_1 = *store / x1;
        let tmp_term_2 = (net_precipitation / x1).tanh();

        store_precipitation = x1 * (1. - tmp_term_1 * tmp_term_1) * tmp_term_2
            / (1. + tmp_term_1 * tmp_term_2);
        *store += store_precipitation;
    } else if precipitation < pet {
        let net_pet = pet - precipitation;
        // only calculate terms once
        let tmp_term_1 = *store / x1;
        let tmp_term_2 = (net_pet / x1).tanh();
        let evapotranspiration = *store * (2. - tmp_term_1) * tmp_term_2
            / (1. + (1. - tmp_term_1) * tmp_term_2);
        *store -= evapotranspiration;
    }

    let mut percolation = 0.0;
    if x1 / *store > 1e-3 {
        percolation =
            *store * (1. - (1. + (4. / 9. * *store / x1).powi(4)).powf(-0.25));
        *store -= percolation;
    }

    *routing_precipitation =
        net_precipitation - store_precipitation + percolation;
}

fn update_routing(
    store: &mut f64,
    hydrographs: &mut (Vec<f64>, Vec<f64>),
    total_flow: &mut f64,
    unit_hydrographs: &(Vec<f64>, Vec<f64>),
    routing_precipitation: f64,
    x2: f64,
    x3: f64,
) {
    update_hydrographs(routing_precipitation, hydrographs, unit_hydrographs);

    let q9 = hydrographs.0[0];
    let q1 = hydrographs.1[0];

    let groundwater_exchange = x2 * (*store / x3).powf(3.5);

    *store = (*store + q9 + groundwater_exchange).max(1e-3 * x3);

    let routed_flow = *store * (1. - (1. + (*store / x3).powi(4)).powf(-0.25));
    *store -= routed_flow;

    let direct_flow = (q1 + groundwater_exchange).max(0.);

    *total_flow = routed_flow + direct_flow;
}

fn update_hydrographs(
    routing_precipitation: f64,
    hydrographs: &mut (Vec<f64>, Vec<f64>),
    unit_hydrographs: &(Vec<f64>, Vec<f64>),
) {
    let n1 = hydrographs.0.len();
    for i in 0..n1 - 1 {
        hydrographs.0[i] = hydrographs.0[i + 1]
            + 0.9 * routing_precipitation * unit_hydrographs.0[i];
    }
    hydrographs.0[n1 - 1] = 0.0;

    let n2 = hydrographs.1.len();
    for i in 0..n2 - 1 {
        hydrographs.1[i] = hydrographs.1[i + 1]
            + 0.1 * routing_precipitation * unit_hydrographs.1[i];
    }
    hydrographs.1[n2 - 1] = 0.0;
}

#[pyfunction]
#[pyo3(name = "init")]
pub fn py_init<'py>(
    py: Python<'py>,
) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray2<f64>>) {
    let (default_values, bounds) = init();
    (default_values.to_pyarray(py), bounds.to_pyarray(py))
}

#[pyfunction]
#[pyo3(name = "simulate")]
pub fn py_simulate<'py>(
    py: Python<'py>,
    params: PyReadonlyArray1<f64>,
    data: PyData,
    metadata: PyMetadata,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let simulation =
        simulate(params.as_array(), data.as_data()?, &metadata.as_metadata())?;
    Ok(simulation.to_pyarray(py))
}

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "gr4j")?;
    m.add_function(wrap_pyfunction!(py_init, &m)?)?;
    m.add_function(wrap_pyfunction!(py_simulate, &m)?)?;
    Ok(m)
}
