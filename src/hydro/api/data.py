from typing import Any, TypedDict

import polars as pl
from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import hydro
from hydro.logging import logger

from .utils import convert_for_json

#########
# types #
#########


class Data(TypedDict):
    pass


PetModels = ["oudin"]
SnowModels = [None, "cemaneige"]

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
        case "hydro_data":
            return await _handle_hydro_data_message(
                ws, msg.get("data", {}), data
            )


async def _handle_models_message(ws: WebSocket, data: Data) -> Data:
    models = {"pet": ["oudin"], "snow": ["cemaneige", None]}

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
            if msg_data["val"] in PetModels:
                await _send(ws, "model", msg_data)
                return {**data, "pet_model": msg_data["val"]}
        case "snow":
            if msg_data["val"] == "":
                msg_data["val"] = None
            if msg_data["val"] in SnowModels:
                await _send(ws, "model", msg_data)
                return {**data, "snow_model": msg_data["val"]}
        case _:
            await _send(ws, "error", "The only valid types are pet and snow.")
            return data


async def _handle_hydro_data_message(
    ws: WebSocket, msg_data: dict[str, Any], data: Data
) -> Data:
    if "station" not in msg_data:
        await _send(ws, "error", "The station must be provided.")
        return data

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    hydro_data = await hydro.read_data_async(
        id, refresh=msg_data.get("refresh", False)
    )
    hydro_data = _prepare_data_to_show(hydro_data.drop("lat", "lon"))

    await _send(ws, "hydro_data", convert_for_json(hydro_data))

    return {**data, "hydro_data": hydro_data}


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
