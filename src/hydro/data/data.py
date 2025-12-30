from typing import Literal

import polars as pl
from hydro_rs import pet

from hydro.utils import paths

from .hydro import Metadata

#########
# types #
#########

PetModels = Literal["oudin"]
SnowModels = Literal["cemaneige"]
ClimateModels = Literal["gr4j", "bucket"]

##########
# public #
##########


def create_datasets(
    station: str,
    hydro_data: pl.DataFrame,
    weather_data: pl.DataFrame,
    precipitation_data: pl.DataFrame,
    hydro_metadata: Metadata,
    pet_model: PetModels,
    n_valid_years: int,
) -> tuple[pl.DataFrame, pl.DataFrame]:
    data = _read_joined_data(
        station, hydro_data, weather_data, precipitation_data
    )
    data = _add_pet_data(station, data, hydro_metadata, pet_model)
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


def _add_pet_data(
    station: str,
    data: pl.DataFrame,
    hydro_metadata: Metadata,
    pet_model: PetModels,
) -> pl.DataFrame:
    last_date = data[-1, "date"]
    path = (
        paths.data_dir
        / "transformed"
        / "features"
        / f"{station}_{last_date}_{pet_model}.ipc"
    )
    if path.exists():
        pet_data = pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        match (pet_model):
            case "oudin":
                temperature = data["temperature"].to_numpy()
                day_of_year = (
                    data["date"].dt.ordinal_day().cast(pl.Float64).to_numpy()
                )
                pet_data_ = pet.oudin.simulate(
                    temperature, day_of_year, hydro_metadata.lat
                )
                pet_data = pl.DataFrame(
                    {"date": data["date"], "pet": pet_data_}
                )
                pet_data.write_ipc(path)
            case _:
                raise NotImplementedError(
                    f"The pet model {pet_model} isn't implemented."
                )
    return data.join(pet_data, on="date").sort("date")
