use ndarray::Array1;
use rand_chacha::ChaCha8Rng;
use std::str::FromStr;

use crate::model::SimulateFn;

pub struct CalibrationParams {
    pub params: Array1<f64>,
    pub simulate: SimulateFn,
    pub lower_bounds: Array1<f64>,
    pub upper_bounds: Array1<f64>,
    pub objective: Objective,
    pub rng: ChaCha8Rng,
    pub done: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Objective {
    Rmse,
    Nse,
    Kge,
}

impl FromStr for Objective {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rmse" => Ok(Self::Rmse),
            "nse" => Ok(Self::Nse),
            "kge" => Ok(Self::Kge),
            _ => Err(format!(
                "Unknown objective function '{}'. Valid options: nse, kge, rmse",
                s
            )),
        }
    }
}
