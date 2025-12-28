use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use std::f64::consts::PI;

pub fn make_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "oudin")?;
    m.add_function(wrap_pyfunction!(simulate, &m)?)?;
    Ok(m)
}

#[pyfunction]
fn simulate<'py>(
    py: Python<'py>,
    temperature: PyReadonlyArray1<'py, f64>,
    day_of_year: PyReadonlyArray1<'py, f64>,
    latitude: f64,
) -> Bound<'py, PyArray1<f64>> {
    let temp = temperature.as_slice().unwrap();
    let doy = day_of_year.as_slice().unwrap();

    let gsc = 0.082; // solar constant (MJ m^-2 min^-1)
    let rho = 1000.; // water density (kg/m^3)
    let n_timesteps: usize = temp.len();
    let lat_rad = PI * latitude / 180.; // latitude in rad

    let mut potential_evapotranspiration: Vec<f64> = vec![];

    for t in 0..n_timesteps {
        let lambda = 2.501 - 0.002361 * temp[t]; // latent heat of vaporization (MJ/kg)
        let doy = doy[t];
        let ds = 0.409 * (2. * PI / 365. * doy - 1.39).sin(); // solar declination (rad)
        let dr = 1. + 0.033 * (doy * 2. * PI / 365.).cos(); // inverse relative distance Earth-Sun
        let omega = (-lat_rad.tan() * ds.tan()).clamp(-1., 1.).acos(); // sunset hour angle (rad)
        let re = 24. * 60. / PI
            * gsc
            * dr
            * (omega * lat_rad.sin() * ds.sin() + lat_rad.cos() * ds.cos() * omega.sin()); // extraterrestrial radiation (MJ m^-2 day^-1)
        potential_evapotranspiration
            .push((re / (lambda * rho) * (temp[t] + 5.) / 100. * 1000.).max(0.));
    }

    PyArray1::from_vec(py, potential_evapotranspiration)
}
