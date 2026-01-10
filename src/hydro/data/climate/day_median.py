import numpy as np
import numpy.typing as npt
import polars as pl

from ..hydro import Metadata


def init() -> tuple[npt.NDArray[np.float64], npt.NDArray[np.float64]]:
    # corresponds to each day of the year (ignoring 29 february)
    default_values = np.repeat(0.0, 365)
    bounds = np.repeat([0.0, 100_000.0], 365)
    return default_values, bounds


def simulate(
    params: npt.NDArray[np.float64], data: pl.DataFrame, metadata: Metadata
) -> npt.NDArray[np.float64]:
    days = data.select((pl.col("date").dt.ordinal_day() - 1).mod(365))[
        "date"
    ].to_numpy()
    return params[days]


def calibrate(data: pl.DataFrame) -> npt.NDArray[np.float64]:
    return (
        data.select(
            "discharge",
            "date",
        )
        .group_by((pl.col("date").dt.ordinal_day().mod(365) - 1).alias("day"))
        .agg(pl.col("discharge").median())
        .sort("day")["discharge"]
        .to_numpy()
    )
