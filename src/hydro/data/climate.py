import asyncio
import time
from typing import Awaitable, Callable, Literal, Self, assert_never

import hydro_rs
import numpy as np
import numpy.typing as npt
import polars as pl

from .hydro import Metadata

#########
# types #
#########

ClimateModels = Literal["day_median", "gr4j", "bucket"]

##########
# public #
##########


class ClimateModel:
    async def calibrate(
        self,
        data: pl.DataFrame,
        metadata: Metadata,
        *,
        callback: (
            Callable[[bool, pl.DataFrame, dict[str, float]], Awaitable[None]]
            | None
        ) = None,
        stop_event: asyncio.Event | None = None,
    ) -> Self:
        raise NotImplementedError()

    def __call__(self, data: pl.DataFrame) -> npt.NDArray[np.float64]:
        raise NotImplementedError()


class DayMedianClimateModel(ClimateModel):
    def __init__(self) -> None:
        self._medians: pl.DataFrame | None = None

    async def calibrate(
        self,
        data: pl.DataFrame,
        metadata: Metadata,
        *,
        callback: (
            Callable[[bool, pl.DataFrame, dict[str, float]], Awaitable[None]]
            | None
        ) = None,
        stop_event: asyncio.Event | None = None,
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
            _predictions = self.__call__(data)
            predictions = data.select("date").with_columns(
                pl.Series("discharge", _predictions)
            )
            results = evaluate_model(
                data["discharge"].to_numpy(), _predictions
            )
            await callback(done=True, predictions=predictions, results=results)
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
        self._median = None

    async def calibrate(
        self,
        data: pl.DataFrame,
        metadata: Metadata,
        *,
        callback: (
            Callable[[bool, pl.DataFrame, dict[str, float]], Awaitable[None]]
            | None
        ) = None,
        stop_event: asyncio.Event | None = None,
    ) -> Self:
        self._median = data["discharge"].median()
        for iter in range(10):
            if stop_event is not None and stop_event.is_set():
                break
            await asyncio.sleep(1)
            if callback is not None:
                _predictions = self.__call__(data)
                predictions = data.select("date").with_columns(
                    pl.Series("discharge", _predictions)
                )
                results = evaluate_model(
                    data["discharge"].to_numpy(), _predictions
                )
                await callback(
                    done=False, predictions=predictions, results=results
                )
        return self

    def __call__(self, data: pl.DataFrame) -> npt.NDArray[np.float64]:
        return np.repeat(self._median, data.shape[0])


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
