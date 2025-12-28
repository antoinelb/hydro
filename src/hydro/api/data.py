from typing import Any, TypedDict

import polars as pl
from hydro_rs import pet, snow
from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import hydro, precipitation, weather
from hydro.logging import logger

from .utils import convert_for_json

#########
# types #
#########


class Data(TypedDict):
    pet_model: str | None
    snow_model: str | None
    validation_years: int | None
    hydro_data: pl.DataFrame | None
    weather_data: pl.DataFrame | None
    precipitation_data: pl.DataFrame | None
    pet_data: pl.DataFrame | None


##########
# public #
##########


def get_routes() -> list[BaseRoute]:
    return [
        WebSocketRoute("/", endpoint=_websocket_handler),
    ]


##########
# routes #
##########


async def _websocket_handler(ws: WebSocket) -> None:
    await ws.accept()
    data = {
        "pet_model": None,
        "snow_model": None,
        "validation_years": None,
        "hydro_data": None,
        "weather_data": None,
        "precipitation_data": None,
        "pet_data": None,
    }
    try:
        while True:
            msg = await ws.receive_json()
            data = await _handle_message(ws, msg, data)
    except WebSocketDisconnect:
        pass


async def _handle_message(
    ws: WebSocket, msg: dict[str, Any], data: Data
) -> Data:
    logger.info(f"Websocket {msg.get('type')} message")
    match msg.get("type"):
        case "models":
            return await _handle_models_message(ws, data)
        case "model":
            return await _handle_model_message(ws, msg.get("data", {}), data)
        case "validation_years":
            return await _handle_validation_years_message(
                ws, msg.get("data", None), data
            )
        case "hydro_data":
            return await _handle_hydro_data_message(
                ws, msg.get("data", {}), data
            )
        case "weather_data":
            return await _handle_weather_data_message(
                ws, msg.get("data", {}), data
            )
        case "precipitation_data":
            return await _handle_precipitation_data_message(
                ws, msg.get("data", {}), data
            )


async def _handle_models_message(ws: WebSocket, data: Data) -> Data:
    models = {"pet": pet.Models, "snow": ["cemaneige", None]}

    await _send(ws, "models", convert_for_json(models))

    return data


async def _handle_model_message(
    ws: WebSocket, msg_data: dict[str, Any], data: Data
) -> Data:
    if "type" not in msg_data or "val" not in msg_data:
        await _send(ws, "error", "The type and val must be provided.")
        return data

    match msg_data["type"]:
        case "pet":
            if msg_data["val"] in pet.Models:
                await _send(ws, "model", msg_data)
                return {**data, "pet_model": msg_data["val"]}
        case "snow":
            if msg_data["val"] == "" or msg_data["val"] is None:
                await _send(ws, "model", None)
                return {**data, "snow_model": None}
            if msg_data["val"] in snow.Models:
                await _send(ws, "model", msg_data)
                return {**data, "snow_model": msg_data["val"]}
        case _:
            await _send(ws, "error", "The only valid types are pet and snow.")
            return data


async def _handle_validation_years_message(
    ws: WebSocket, msg_data: int | None, data: Data
) -> Data:
    if msg_data is None:
        await _send(
            ws, "error", "The number of validation years must be an integer."
        )
        return data

    await _send(ws, "validation_years", msg_data)

    return {**data, "validation_years": msg_data}


async def _handle_hydro_data_message(
    ws: WebSocket, msg_data: dict[str, Any], data: Data
) -> Data:
    if "station" not in msg_data:
        await _send(ws, "error", "The station must be provided.")
        return data

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    hydro_data = await hydro.read_data(
        id, refresh=msg_data.get("refresh", False)
    )
    data = {**data, "hydro_data": hydro_data}
    hydro_data = _prepare_data_to_show(hydro_data.drop("lat", "lon"))

    await _send(ws, "hydro_data", convert_for_json(hydro_data))

    return data


async def _handle_weather_data_message(
    ws: WebSocket, msg_data: dict[str, Any], data: Data
) -> Data:
    if data["hydro_data"] is None:
        await _send(ws, "error", "The hydro data must have been read.")
        return data

    weather_data = await weather.read_closest_data(
        data["hydro_data"], refresh=msg_data.get("refresh", False)
    )
    data = {**data, "weather_data": weather_data}
    weather_data = _prepare_data_to_show(weather_data.drop("precipitation"))

    await _send(ws, "weather_data", convert_for_json(weather_data))

    return data


async def _handle_precipitation_data_message(
    ws: WebSocket, msg_data: dict[str, Any], data: Data
) -> Data:
    if data["hydro_data"] is None:
        await _send(ws, "error", "The hydro data must have been read.")
        return data

    precipitation_data = await precipitation.read_data(data["hydro_data"])
    data = {**data, "precipitation_data": precipitation_data}
    precipitation_data = _prepare_data_to_show(precipitation_data)

    await _send(ws, "precipitation_data", convert_for_json(precipitation_data))

    return data


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": data})


def _prepare_data_to_show(data: pl.DataFrame) -> pl.DataFrame:
    last_date = data[-1, "date"]
    return (
        data.filter(
            (pl.col("date").dt.month() != 2) | (pl.col("date").dt.day() != 29)
        )
        .with_columns(
            pl.when(pl.col("date").dt.replace(year=last_date.year) > last_date)
            .then(pl.col("date").dt.replace(year=last_date.year - 1))
            .otherwise(pl.col("date").dt.replace(year=last_date.year))
            .alias("date")
        )
        .group_by("date")
        .agg(
            pl.all().median().name.map(lambda name: f"{name}_median"),
            pl.all().max().name.map(lambda name: f"{name}_max"),
            pl.all().min().name.map(lambda name: f"{name}_min"),
        )
        .join(data, on="date", how="inner")
        .sort("date")
    )
