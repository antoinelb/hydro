import asyncio
from typing import Any, Awaitable, Callable, Literal, assert_never

import numpy as np
import numpy.typing as npt
import polars as pl
from hydro_rs.calibration.sce import Sce

from .climate import day_median
from .hydro import Metadata
from .utils import Data

#########
# types #
#########

Objective = Literal["rmse", "nse", "kge"]
Transformation = Literal["log", "sqrt", "none"]
Algorithm = Literal["sce"]

##########
# public #
##########


def get_algorithm_params(
    algorithm: Algorithm,
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
            assert_never(algorithm)  # type: ignore


async def calibrate(
    data: pl.DataFrame,
    metadata: Metadata,
    climate_model: str,
    snow_model: str | None,
    objective: Objective,
    algorithm: Algorithm,
    params: dict[str, Any],
    *,
    callback: (
        Callable[[bool, pl.DataFrame, dict[str, float]], Awaitable[None]]
        | None
    ) = None,
    stop_event: asyncio.Event | None = None,
) -> npt.NDArray[np.float64]:
    if climate_model == "day_median":
        return day_median.calibrate(data)
    else:
        seed = 123
        max_iter = 100_000
        _data = Data(
            data["precipitation"].to_numpy(),
            data["temperature"].to_numpy(),
            data["pet"].to_numpy(),
            data["date"].dt.ordinal_day().to_numpy().astype(np.uintp),
        )
        observations = data["discharge"].to_numpy()

        match algorithm:
            case "sce":
                calibration = Sce(
                    climate_model,
                    snow_model,
                    objective,
                    seed=seed,
                    n_complexes=params["n_complexes"],
                    k_stop=params["k_stop"],
                    p_convergence_threshold=params["p_convergence_threshold"],
                    geometric_range_threshold=params[
                        "geometric_range_threshold"
                    ],
                    max_evaluations=params["max_evaluations"],
                )
                calibration.init(_data, metadata, observations)
                for _ in range(max_iter):
                    done, params, _simulation, objectives = calibration.step(
                        _data, metadata, observations
                    )
                    simulation = data.select("date").with_columns(
                        pl.Series("discharge", _simulation)
                    )
                    results = {
                        "rmse": objectives[0],
                        "nse": objectives[1],
                        "kge": objectives[2],
                    }
                    if callback is not None:
                        print(simulation[:10, "discharge"])
                        print(observations[:10])
                        await callback(done, simulation, results)
                    # Yield control to allow I/O processing (e.g., receiving stop message)
                    await asyncio.sleep(0.001)
                    if stop_event is not None and stop_event.is_set():
                        break
                    if done:
                        break

                return np.array(params)

            case _:
                assert_never(algorithm)  # type: ignore
