from typing import Literal

import polars as pl
from hydro_rs import pet, snow


def create_datasets(
    hydro_data: pl.DataFrame,
    weather_data: pl.DataFrame,
    precipitation_data: pl.DataFrame,
    pet_model: Literal[tuple(pet.Models)],
    snow_model: Literal[tuple(snow.Models)] | None,
    n_valid_years: int,
) -> tuple[pl.DataFrame, pl.DataFrame]:
    data = hydro_data.join(weather_data, on="date").join(
        precipitation_data, on="date"
    )
    year_threshold = data["date"].dt.year().unique().sort()[-n_valid_years - 1]
    calib_data = data.filter(pl.col("date").dt.year() < year_threshold)
    valid_data = data.filter(pl.col("date").dt.year() >= year_threshold)
    return calib_data, valid_data
