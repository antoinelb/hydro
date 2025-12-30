from typing import Literal

import polars as pl
from hydro_rs import pet

from hydro.utils import paths

from . import hydro, precipitation, weather

#########
# types #
#########

PetModels = Literal["oudin"]
SnowModels = Literal["cemaneige"]
ClimateModels = Literal["gr4j", "bucket"]

##########
# public #
##########


async def read_datasets(
    id: str,
    pet_model: PetModels,
    n_valid_years: int,
    *,
    refresh: bool = False,
) -> tuple[pl.DataFrame, pl.DataFrame]:
    data = await _read_joined_data(id, refresh=refresh)
    data = await _add_pet_data(id, data, pet_model, refresh=refresh)
    year_threshold = data["date"].dt.year().unique().sort()[-n_valid_years - 1]
    calib_data = data.filter(pl.col("date").dt.year() < year_threshold)
    valid_data = data.filter(pl.col("date").dt.year() >= year_threshold)
    return calib_data, valid_data


###########
# private #
###########


async def _read_joined_data(id: str, *, refresh: bool = False) -> pl.DataFrame:
    path = paths.data_dir / "transformed" / "joined" / f"{id}.ipc"
    if path.exists() and not refresh:
        return pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        hydro_data = await hydro.read_data(id, refresh=refresh)
        weather_data = await weather.read_closest_data(
            hydro_data, refresh=refresh
        )
        precipitation_data = await precipitation.read_data(
            hydro_data, refresh=refresh
        )
        data = (
            hydro_data.join(weather_data, on="date")
            .join(precipitation_data, on="date")
            .sort("date")
        )
        data = (
            hydro_data.join(weather_data, on="date")
            .join(precipitation_data, on="date")
            .sort("date")
        )
        data.write_ipc(path)
        return data


async def _add_pet_data(
    id: str,
    data: pl.DataFrame,
    pet_model: PetModels,
    *,
    refresh: bool = False,
) -> pl.DataFrame:
    path = (
        paths.data_dir / "transformed" / "features" / f"{id}_{pet_model}.ipc"
    )
    if path.exists() and not refresh:
        pet_data = pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        hydro_metadata = await hydro.read_metadata(id)
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
