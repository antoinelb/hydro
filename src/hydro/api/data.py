from typing import Any, get_args

import polars as pl
from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import (
    PetModel,
    hydro,
    precipitation,
    read_datasets,
    weather,
)
from hydro.logging import logger

from .utils import convert_for_json

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
    try:
        while True:
            msg = await ws.receive_json()
            await _handle_message(ws, msg)
    except WebSocketDisconnect:
        pass


async def _handle_message(ws: WebSocket, msg: dict[str, Any]) -> None:
    logger.info(f"Websocket {msg.get('type')} message")
    match msg.get("type"):
        case "models":
            await _handle_models_message(ws)
        case "model":
            await _handle_model_message(ws, msg.get("data", {}))
        case "validation_years":
            await _handle_validation_years_message(ws, msg.get("data", None))
        case "hydro_data":
            await _handle_hydro_data_message(ws, msg.get("data", {}))
        case "weather_data":
            await _handle_weather_data_message(ws, msg.get("data", {}))
        case "precipitation_data":
            await _handle_precipitation_data_message(ws, msg.get("data", {}))
        case "datasets":
            await _handle_datasets_message(ws, msg.get("data", {}))
        case _:
            await _send(ws, "error", f"Unknown message type {msg['type']}.")


async def _handle_models_message(ws: WebSocket) -> None:
    models = {"pet": get_args(PetModel)}
    await _send(ws, "models", convert_for_json(models))


async def _handle_model_message(
    ws: WebSocket, msg_data: dict[str, Any]
) -> None:
    if "type" not in msg_data or "val" not in msg_data:
        await _send(ws, "error", "The type and val must be provided.")
        return

    match msg_data["type"]:
        case "pet":
            if msg_data["val"] in get_args(PetModel):
                await _send(ws, "model", msg_data)
        case _:
            await _send(ws, "error", "The only valid types are pet and snow.")


async def _handle_validation_years_message(
    ws: WebSocket, msg_data: int | None
) -> None:
    if msg_data is None:
        await _send(
            ws, "error", "The number of validation years must be an integer."
        )
        return
    await _send(ws, "validation_years", msg_data)


async def _handle_hydro_data_message(
    ws: WebSocket, msg_data: dict[str, Any]
) -> None:
    if "station" not in msg_data:
        await _send(ws, "error", "The station must be provided.")
        return

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    hydro_data = await hydro.read_data(
        id, refresh=msg_data.get("refresh", False)
    )
    hydro_data = _prepare_data_to_show(hydro_data.drop("lat", "lon"))

    await _send(ws, "hydro_data", convert_for_json(hydro_data))


async def _handle_weather_data_message(
    ws: WebSocket, msg_data: dict[str, Any]
) -> None:
    if "station" not in msg_data:
        await _send(ws, "error", "The station must be provided.")
        return

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    hydro_data = await hydro.read_data(
        id, refresh=msg_data.get("refresh", False)
    )

    weather_data = await weather.read_closest_data(
        hydro_data, refresh=msg_data.get("refresh", False)
    )
    weather_data = _prepare_data_to_show(weather_data.drop("precipitation"))

    await _send(ws, "weather_data", convert_for_json(weather_data))


async def _handle_precipitation_data_message(
    ws: WebSocket, msg_data: dict[str, Any]
) -> None:
    if "station" not in msg_data:
        await _send(ws, "error", "The station must be provided.")
        return

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    hydro_data = await hydro.read_data(
        id, refresh=msg_data.get("refresh", False)
    )

    precipitation_data = await precipitation.read_data(
        hydro_data, refresh=msg_data.get("refresh", False)
    )
    precipitation_data = _prepare_data_to_show(precipitation_data)

    await _send(ws, "precipitation_data", convert_for_json(precipitation_data))


async def _handle_datasets_message(
    ws: WebSocket, msg_data: dict[str, Any]
) -> None:
    if any(
        key not in msg_data
        for key in ("station", "pet_model", "n_valid_years")
    ):
        await _send(
            ws,
            "error",
            "The `station`, `pet_model` and `n_valid_years` must be provided.",
        )
        return

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    calib_data, valid_data = await read_datasets(
        id, msg_data["pet_model"], msg_data["n_valid_years"]
    )

    await _send(
        ws,
        "datasets",
        convert_for_json(
            {"calibration": calib_data, "validation": valid_data}
        ),
    )


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": convert_for_json(data)})


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
