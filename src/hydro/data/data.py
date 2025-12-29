from typing import Literal

import polars as pl
from hydro_rs import pet, snow

from hydro.utils import paths

##########
# public #
##########


def create_datasets(
    station: str,
    hydro_data: pl.DataFrame,
    weather_data: pl.DataFrame,
    precipitation_data: pl.DataFrame,
    pet_model: Literal[tuple(pet.Models)],  # type: ignore
    snow_model: Literal[tuple(snow.Models)] | None,  # type: ignore
    n_valid_years: int,
) -> tuple[pl.DataFrame, pl.DataFrame]:
    data = _read_joined_data(
        station, hydro_data, weather_data, precipitation_data
    )
    year_threshold = data["date"].dt.year().unique().sort()[-n_valid_years - 1]
    calib_data = data.filter(pl.col("date").dt.year() < year_threshold)
    valid_data = data.filter(pl.col("date").dt.year() >= year_threshold)
    return calib_data, valid_data


###########
# private #
###########


def _read_joined_data(
    station: str,
    hydro_data: pl.DataFrame,
    weather_data: pl.DataFrame,
    precipitation_data: pl.DataFrame,
) -> pl.DataFrame:
    last_date = max(
        hydro_data[-1, "date"],
        weather_data[-1, "date"],
        precipitation_data[-1, "date"],
    )
    path = (
        paths.data_dir
        / "transformed"
        / "joined"
        / f"{station}_{last_date}.ipc"
    )
    if path.exists():
        return pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        data = (
            hydro_data.join(weather_data, on="date")
            .join(precipitation_data, on="date")
            .sort("date")
        )
        data.write_ipc(path)
        return data


def _add_pet(
    station: str,
    data: pl.DataFrame,
    pet_model: Literal[tuple(pet.Models)],  # type: ignore
) -> pl.DataFrame:
    last_date = data[-1, "date"]
    path = (
        paths.data_dir
        / "transformed"
        / "features"
        / f"{station}_{last_date}_{pet_model}.ipc"
    )
    if path.exists():
        pet = pl.read_ipc(path)
        return data.join(pet, on="date").sort("date")
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        match (pet_model):
            case "oudin":
                pass
            case _:
                raise NotImplementedError(
                    f"The pet model {pet_model} isn't implemented."
                )
