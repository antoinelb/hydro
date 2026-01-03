import asyncio
from typing import Awaitable, Callable, Literal, Self, assert_never

import hydro_rs
import numpy as np
import numpy.typing as npt
import polars as pl

from .calibration import Algorithms, ObjectiveFunctions, calibrate
from .hydro import Metadata

#########
# types #
#########

ClimateModels = Literal["day_median", "gr4j", "bucket"]

##########
# public #
##########


class ClimateModel:
    """Base class for climate/hydrological models."""

    @property
    def name(self) -> str:
        """Model name used for Rust dispatch."""
        raise NotImplementedError()

    @property
    def param_names(self) -> list[str]:
        """Ordered list of parameter names."""
        raise NotImplementedError()

    @property
    def param_bounds(self) -> list[tuple[float, float]]:
        """Parameter bounds as list of (min, max) tuples."""
        raise NotImplementedError()

    async def calibrate(
        self,
        data: pl.DataFrame,
        metadata: Metadata,
        *,
        algorithm: Algorithms = "sce",
        objective: ObjectiveFunctions = "kge",
        algorithm_params: dict = {},
        callback: (
            Callable[[bool, pl.DataFrame, dict[str, float]], Awaitable[None]]
            | None
        ) = None,
        stop_event: asyncio.Event | None = None,
    ) -> Self:
        """Calibrate the model against observed discharge data."""
        raise NotImplementedError()

    def __call__(self, data: pl.DataFrame) -> npt.NDArray[np.float64]:
        """Run the model with current parameters."""
        raise NotImplementedError()


class DayMedianClimateModel(ClimateModel):
    def __init__(self) -> None:
        self._medians: pl.DataFrame | None = None

    async def calibrate(
        self,
        data: pl.DataFrame,
        metadata: Metadata,
        *,
        calibration_params: None = None,
        callback: None = None,
        stop_event: None = None,
    ) -> Self:
        self._medians = (
            data.select(
                "discharge",
                pl.when(
                    (pl.col("date").dt.month() == 2)
                    & (pl.col("date").dt.day() == 29)
                )
                .then(pl.col("date").dt.replace(day=28))
                .otherwise(pl.col("date"))
                .alias("date"),
            )
            .with_columns(
                pl.col("date").dt.month().alias("month"),
                pl.col("date").dt.day().alias("day"),
            )
            .group_by("month", "day")
            .agg(
                pl.col("discharge").median(),
            )
        )
        if callback is not None:
            raise ValueError()
        return self

    def __call__(self, data: pl.DataFrame) -> npt.NDArray[np.float64]:
        return (
            data.select(
                pl.when(
                    (pl.col("date").dt.month() == 2)
                    & (pl.col("date").dt.day() == 29)
                )
                .then(pl.col("date").dt.replace(day=28))
                .otherwise(pl.col("date"))
                .alias("date"),
            )
            .with_columns(
                pl.col("date").dt.month().alias("month"),
                pl.col("date").dt.day().alias("day"),
            )
            .with_row_index()
            .join(self._medians, on=["month", "day"])
            .sort("index")["discharge"]
            .to_numpy()
        )


class Gr4jClimateModel(ClimateModel):
    def __init__(self) -> None:
        self._params = {
            param: (min_ + max_) / 2
            for param, (min_, max_) in self.possible_params.items()
        }

    async def calibrate(
        self,
        data: pl.DataFrame,
        metadata: Metadata,
        *,
        algorithm: Algorithms = "sce",
        objective: ObjectiveFunctions = "kge",
        algorithm_params: dict,
        callback: (
            Callable[
                [bool, pl.DataFrame, dict[str, float]],
                Awaitable[None],
            ]
            | None
        ) = None,
        stop_event: asyncio.Event | None = None,
    ) -> Self:
        params = await calibrate(
            "gr4j",
            list(self.possible_params.values()),
            data,
            algorithm,
            objective,
            algorithm_params,
            callback=callback,
            stop_event=stop_event,
        )
        self._params = {
            param: value
            for param, value in zip(
                self.possible_params.keys(), params, strict=True
            )
        }
        return self

    def __call__(self, data: pl.DataFrame) -> npt.NDArray[np.float64]:
        precipitation = data["precipitation"].to_numpy()
        pet = data["pet"].to_numpy()
        return hydro_rs.climate.gr4j.simulate(
            np.array(self._params.values()), precipitation, pet
        )

    @property
    def possible_params(self) -> dict[str, tuple[float, float]]:
        return {
            "x1": (10, 1500),
            "x2": (-5, 3),
            "x3": (10, 400),
            "x4": (0.8, 10.0),
        }


def get_model(model: ClimateModels) -> ClimateModel:
    match model:
        case "day_median":
            return DayMedianClimateModel()
        case "gr4j":
            return Gr4jClimateModel()
        case "bucket":
            raise NotImplementedError()
        case _:
            assert_never(model)


def evaluate_model(
    observations: npt.NDArray[np.float64], predictions: npt.NDArray[np.float64]
) -> dict[str, float]:
    if observations.shape != predictions.shape:
        raise ValueError(
            "Shapes of `observations` and `predictions` don't match."
        )
    rmse = hydro_rs.utils.calculate_rmse(observations, predictions)
    nse = hydro_rs.utils.calculate_nse(observations, predictions)
    kge = hydro_rs.utils.calculate_kge(observations, predictions)
    return {"rmse": rmse, "nse": nse, "kge": kge}
