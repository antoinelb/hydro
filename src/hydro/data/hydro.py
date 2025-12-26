import asyncio
import re
from typing import Literal, assert_never

import httpx
import polars as pl

from hydro.utils import paths

##########
# public #
##########


def read_stations() -> pl.DataFrame:
    url = "https://www.donneesquebec.ca/recherche/dataset/c31e2bee-a899-46ca-ad84-5798f0f49676/resource/6b2d32ef-80e2-445b-9bd1-97ddc39b5d59/download/stations_hydrometriques.csv"
    path = paths.data_dir / "raw" / "hydro" / "stations.ipc"
    if path.exists():
        data = pl.read_ipc(path)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        data = (
            pl.read_csv(url, schema_overrides={"no": pl.String})
            .select(
                pl.col("no").alias("id"),
                pl.col("nom").alias("name"),
                "type",
                pl.col("latitude").alias("lat"),
                pl.col("longitude").alias("lon"),
                pl.col("debut").cast(pl.Int64, strict=False).alias("start"),
                pl.col("fin").cast(pl.Int64, strict=False).alias("end"),
                pl.col("cours_eau").alias("waterway"),
                pl.col("superficie").alias("area"),
                (pl.col("regime") == "Influencé").alias("influenced"),
                (pl.col("lien_historique") == "www.eau.ec.gc.ca").alias(
                    "federal"
                ),
            )
            .filter(
                pl.col("type").str.contains("Débit")
                & ~pl.col("federal")
                & pl.col("end").is_null()
            )
            .drop("type", "federal")
            .with_columns(
                (pl.col("name") + " (" + pl.col("id") + ")").alias("station")
            )
        )
        data.write_ipc(path)
    return data


def read_data(id: str, *, refresh: bool = False) -> pl.DataFrame:
    path = paths.data_dir / "raw" / "hydro" / "stations" / f"{id}.ipc"

    if path.exists() and not refresh:
        return pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        data = asyncio.run(_fetch_data(id))
        data.write_ipc(path)
        return data


async def read_data_async(id: str, *, refresh: bool = False) -> pl.DataFrame:
    path = paths.data_dir / "raw" / "hydro" / "stations" / f"{id}.ipc"

    if path.exists() and not refresh:
        return pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        data = await _fetch_data(id)
        data.write_ipc(path)
        return data


###########
# private #
###########


async def _fetch_data(id: str) -> pl.DataFrame:
    base_url = "https://www.cehq.gouv.qc.ca/depot/historique_donnees/fichier"
    async with httpx.AsyncClient() as client:
        datasets = await asyncio.gather(
            _fetch_dataset(client, base_url, id, "level"),
            _fetch_dataset(client, base_url, id, "discharge"),
        )
    data = datasets[0].join(datasets[1], on=["date", "lat", "lon"])
    if data["date"].n_unique() != data.shape[0]:
        raise RuntimeError(
            "There was an error joining the level and discharge data."
        )
    return data


async def _fetch_dataset(
    client: httpx.AsyncClient,
    base_url: str,
    id: str,
    type: Literal["level", "discharge"],
) -> pl.DataFrame:
    match type:
        case "level":
            url = f"{base_url}/{id}_N.txt"
        case "discharge":
            url = f"{base_url}/{id}_Q.txt"
        case _:
            assert_never(type)

    resp = await client.get(url)
    resp.raise_for_status()
    resp.encoding = "latin-1"

    _data: list[list[str]] = []
    reading = False
    lat = None
    lon = None

    for line in resp.text.split("\n"):
        if line.startswith("Coordonnées:"):
            match = re.match(
                r"^Coordonnées:\s+\([^)]+\) (-?\d+)° (\d+)' (\d+)\" // (-?\d+)° (\d+)' (\d+)\"$",
                line.strip(),
            )
            if match is None:
                raise RuntimeError(
                    "There was an error reading the coordinates."
                )
            lat = _convert_lat_lon_to_decimal(
                float(match.group(1)),
                float(match.group(2)),
                float(match.group(3)),
            )
            lon = _convert_lat_lon_to_decimal(
                float(match.group(4)),
                float(match.group(5)),
                float(match.group(6)),
            )
        elif line.startswith("Station") and not line.startswith("Station:"):
            reading = True
        elif reading:
            _data.append(line.strip().split())

    if lat is None or lon is None:
        raise RuntimeError("There was an error reading the coordinates.")

    return pl.DataFrame(
        [[line[1], line[2]] for line in _data if len(line) > 2],
        schema={"date": pl.String, type: pl.Float64},
        orient="row",
    ).with_columns(
        pl.col("date").str.strptime(pl.Date, "%Y/%m/%d"),
        pl.lit(lat).alias("lat"),
        pl.lit(lon).alias("lon"),
    )


def _convert_lat_lon_to_decimal(
    hours: float, minutes: float, seconds: float
) -> float:
    return hours + minutes / 60 + seconds / 3600
