import asyncio
import json
import re
import zipfile
from pathlib import Path
from typing import Literal, NamedTuple, assert_never, cast

import geopandas as gpd
import httpx
import numpy as np
import numpy.typing as npt
import polars as pl
import rasterio
import rasterio.mask

from hydro.utils import paths

#########
# types #
#########


class Metadata(NamedTuple):
    id: str
    name: str
    station: str
    lat: float
    lon: float
    area: float
    elevation_layers: npt.NDArray[np.float64]
    median_elevation: float


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


async def read_data(id: str, *, refresh: bool = False) -> pl.DataFrame:
    path = paths.data_dir / "raw" / "hydro" / "stations" / f"{id}.ipc"

    if path.exists() and not refresh:
        return pl.read_ipc(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        data = await _fetch_data(id)
        data.write_ipc(path)
        return data


async def read_metadata(id: str) -> Metadata:
    path = paths.data_dir / "raw" / "hydro" / "stations" / f"{id}.json"
    if path.exists():
        with open(path, "r") as f:
            metadata = json.load(f)
            return Metadata(
                id=metadata["id"],
                name=metadata["name"],
                station=metadata["station"],
                lat=metadata["lat"],
                lon=metadata["lon"],
                area=metadata["area"],
                elevation_layers=np.array(metadata["elevation_layers"]),
                median_elevation=metadata["median_elevation"],
            )
    else:
        stations = read_stations()
        stations = stations.filter(pl.col("id") == id)
        if stations.shape[0] == 0:
            raise ValueError(f"Station with id {id} doesn't exist.")
        watershed_data = await _get_watershed_data(id)
        metadata = Metadata(
            id=id,
            name=stations[0, "name"],
            station=stations[0, "station"],
            lat=stations[0, "lat"],
            lon=stations[0, "lon"],
            area=stations[0, "area"],
            elevation_layers=np.array(watershed_data["elevation_bands"]),
            median_elevation=cast(float, watershed_data["median_elevation"]),
        )
        with open(path, "w") as f:
            json.dump(
                {
                    **metadata._asdict(),
                    "elevation_layers": metadata.elevation_layers.tolist(),
                },
                f,
                indent=2,
            )
        return metadata


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
    area = None

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
        if line.startswith("Bassin versant:"):
            area = float(line.split()[2])
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
        pl.lit(area).alias("area"),
    )


def _convert_lat_lon_to_decimal(
    hours: float, minutes: float, seconds: float
) -> float:
    return hours + minutes / 60 + seconds / 3600


async def _get_watershed_data(id: str) -> dict[str, float | list[float]]:
    watersheds = await _get_watersheds()
    watershed = watersheds[watersheds["Station"] == id]
    if watershed.shape[0] == 0:
        raise ValueError(f"Watershed for station with id {id} doesn't exist.")

    dem_path = await _get_dem(id, watershed)

    return _get_watershed_elevation_bands(watershed, dem_path)


async def _get_watersheds() -> gpd.GeoDataFrame:
    url = "https://www.donneesquebec.ca/recherche/dataset/c31e2bee-a899-46ca-ad84-5798f0f49676/resource/924cce0a-5fcc-47fa-a725-5ec84522090f/download/bassins_versants_stations_ouvertes.zip"

    path = paths.data_dir / "raw" / "hydro" / "watersheds" / "watersheds.shp"
    if path.exists():
        return gpd.read_file(path)
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        zip_path = path.parent / "watersheds.zip"
        async with httpx.AsyncClient() as client:
            resp = await client.get(url)
            resp.raise_for_status()
            zip_path.write_bytes(resp.content)
        with zipfile.ZipFile(zip_path, "r") as f:
            f.extractall(path.parent)
        zip_path.unlink()
        for _path in path.parent.glob("*"):
            _path.rename(_path.parent / f"watersheds{_path.suffix}")
        watersheds = gpd.read_file(path.parent / "watersheds.shp")
        return watersheds


async def _get_dem(
    id: str, watershed: gpd.GeoDataFrame, resolution: float = 25
) -> Path:
    path = paths.data_dir / "raw" / "hydro" / "dem" / f"{id}.tiff"

    if path.exists():
        return path
    else:
        path.parent.mkdir(exist_ok=True, parents=True)
        watershed = watershed.to_crs("EPSG:4326")
        min_lon, min_lat, max_lon, max_lat = watershed.total_bounds
        res_deg = resolution / 111000  # approximate degrees per meter
        url = (
            "https://datacube.services.geo.ca/wrapper/ogc/elevation-hrdem-mosaic"
            "?SERVICE=WCS"
            "&VERSION=1.1.1"
            "&REQUEST=GetCoverage"
            "&IDENTIFIER=dtm"
            "&FORMAT=image/geotiff"
            f"&BOUNDINGBOX={min_lat},{min_lon},{max_lat},{max_lon},urn:ogc:def:crs:EPSG::4326"
            f"&GRIDBASECRS=urn:ogc:def:crs:EPSG::4326"
            f"&GRIDOFFSETS={-res_deg},{res_deg}"
        )
        async with httpx.AsyncClient(timeout=120.0) as client:
            resp = await client.get(url)
            try:
                resp.raise_for_status()
            except Exception:
                print(resp.text)
                raise
            path.write_bytes(resp.content)
        return path


def _get_watershed_elevation_bands(
    watershed: gpd.GeoDataFrame, dem_path: Path, *, n_bands: int = 5
) -> dict[str, float | list[float]]:
    with rasterio.open(dem_path) as src:
        watershed = watershed.to_crs(src.crs)

        out_image, out_transform = rasterio.mask.mask(
            src, watershed.geometry, crop=True
        )
        elevations = out_image[0]

        # Get valid (non-nodata) values
        nodata = src.nodata if src.nodata else -9999
        valid_mask = (elevations != nodata) & np.isfinite(elevations)
        elevations = elevations[valid_mask]
        if len(elevations) == 0:
            raise ValueError("No valid elevation data within watershed.")

        edges = np.linspace(
            np.min(elevations), np.max(elevations), n_bands + 1
        )

        bands = []
        for i in range(n_bands):
            band_min = edges[i]
            band_max = edges[i + 1]

            # Count pixels in this band
            if i == n_bands - 1:
                in_band = (elevations >= band_min) & (elevations <= band_max)
            else:
                in_band = (elevations >= band_min) & (elevations < band_max)

            band_elevs = elevations[in_band]
            median_elev = (
                float(np.median(band_elevs))
                if len(band_elevs) > 0
                else (band_min + band_max) / 2
            )

            bands.append(median_elev)

    return {
        "elevation_bands": bands,
        "median_elevation": float(np.median(np.array(bands))),
    }
