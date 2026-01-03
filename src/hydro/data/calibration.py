import asyncio
from typing import Awaitable, Callable, Literal, assert_never

import numpy as np
import numpy.typing as npt
import polars as pl
from hydro_rs.calibration.sce import SCE

#########
# types #
#########

ObjectiveFunctions = Literal["rmse", "nse", "kge"]
Transformations = Literal["log", "sqrt", "none"]
Algorithms = Literal["sce"]

##########
# public #
##########


def get_algorithm_params(
    algorithm: Algorithms,
) -> dict[str, dict[str, int | float | None]]:
    match algorithm:
        case "sce":
            return {
                "n_complexes": {
                    "min": 1,
                    "max": None,
                    "default": 25,
                    "step": 1,
                },
                "k_stop": {
                    "min": 1,
                    "max": None,
                    "default": 10,
                    "step": 1,
                },
                "p_convergence_threshold": {
                    "min": 0,
                    "max": 1,
                    "default": 0.1,
                    "step": 0.01,
                },
                "geometric_range_threshold": {
                    "min": 0,
                    "max": None,
                    "default": 0.001,
                    "step": 0.001,
                },
                "max_evaluations": {
                    "min": 1,
                    "max": None,
                    "default": 5000,
                    "step": 1,
                },
            }
        case _:
            assert_never(algorithm)


async def calibrate(
    model: str,
    param_bounds: list[tuple[float, float]],
    data: pl.DataFrame,
    algorithm: Algorithms,
    objective: ObjectiveFunctions,
    algorithm_params: dict,
    *,
    callback: (
        Callable[[bool, pl.DataFrame, dict[str, float]], Awaitable[None]]
        | None
    ),
    stop_event: asyncio.Event | None = None,
) -> npt.NDArray[np.float64]:
    precipitation = data["precipitation"].to_numpy()
    pet = data["pet"].to_numpy()
    observations = data["discharge"].to_numpy()
    seed = 123
    max_iter = 100_000

    match algorithm:
        case "sce":
            calibration = SCE(
                model,
                param_bounds,
                objective,
                seed=seed,
                **{
                    param: algorithm_params[param]
                    for param in get_algorithm_params("sce").keys()
                },
            )
            calibration.init(precipitation, pet, observations)
            for _ in range(max_iter):
                calibration.step(precipitation, pet, observations)
                simulation = data.select("date").with_columns(
                    pl.Series("discharge", calibration.last_simulation)
                )
                results_ = calibration.last_objectives
                results = {
                    "rmse": results_[0],
                    "nse": results_[1],
                    "kge": results_[2],
                }
                if callback is not None:
                    await callback(calibration.done, simulation, results)
                if stop_event is not None and stop_event.is_set():
                    break
                if calibration.done:
                    break

            return np.array(calibration.best_params)

        case _:
            assert_never(algorithm)
