import asyncio
from datetime import date, timedelta
from typing import cast

import httpx
import polars as pl

from hydro.utils import paths

##########
# public #
##########


async def read_data(
    hydro_data: pl.DataFrame, *, n_concurrent: int = 20, refresh: bool = False
) -> pl.DataFrame:
    date_start = cast(date, hydro_data["date"].min())
    date_end = cast(date, hydro_data["date"].max())
    lat = round(hydro_data[0, "lat"], 2)
    lon = round(hydro_data[0, "lon"], 2)

    path = (
        paths.data_dir
        / "raw"
        / "precipitation"
        / f"{lat:.2f}_{lon:.2f}_{date_start}_{date_end}.ipc"
    )

    if path.exists() and not refresh:
        precipitation_data = pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        precipitation_data = await _fetch_precipitation_data(
            date_start, date_end, lat, lon, n_concurrent=n_concurrent
        )
        precipitation_data.write_ipc(path)

    return hydro_data.select("date").join(precipitation_data, on="date")


###########
# private #
###########


async def _fetch_precipitation_data(
    date_start: date,
    date_end: date,
    lat: float,
    lon: float,
    *,
    n_concurrent: int,
) -> pl.DataFrame:
    base_url = "https://api.weather.gc.ca/collections/weather:rdpa:10km:24f/coverage?f=json"

    # buffer around the point (degrees) - small to get few grid cells
    # RDPA is ~10km resolution, 0.05 deg ≈ 5km, so we get a few cells
    buffer = 0.05
    bbox = ",".join(
        map(str, [lon - buffer, lat - buffer, lon + buffer, lat + buffer])
    )

    base_url = f"{base_url}&bbox={bbox}"

    semaphore = asyncio.Semaphore(n_concurrent)
    async with httpx.AsyncClient() as client:
        _data = await asyncio.gather(
            *[
                _read_daily_precipitation_data(
                    semaphore, client, base_url, date_start + timedelta(days=i)
                )
                for i in range((date_end - date_start).days + 1)
            ]
        )
    return pl.DataFrame([d for d in _data if d])


async def _read_daily_precipitation_data(
    semaphore: asyncio.Semaphore,
    client: httpx.AsyncClient,
    base_url: str,
    date: date,
) -> dict[str, date | float]:
    url = f"{base_url}&datetime={date}T12Z"
    async with semaphore:
        try:
            resp = await client.get(url)
            resp.raise_for_status()
            json = resp.json()
            precipitation = next(
                val
                for val in json["ranges"]["APCP"]["values"]
                if val is not None
            )
            return {
                "date": date,
                "precipitation": precipitation,
            }
        except Exception:
            return {}
