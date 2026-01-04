from datetime import date
from typing import Any

import polars as pl
from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import hydro
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
        case "stations":
            await _handle_stations_message(ws, msg.get("data", {}))
        case "station":
            await _handle_station_message(ws, msg.get("data", None))
        case _:
            await _send(ws, "error", f"Unknown message type {msg['type']}.")


async def _handle_stations_message(
    ws: WebSocket, msg_data: dict[str, Any]
) -> None:
    stations = hydro.read_stations()

    station = msg_data.get("station", "").lower()
    if station != "":
        stations = stations.filter(
            pl.col("station").str.to_lowercase().str.contains(station)
        )

    n_years = int(msg_data.get("n_years", "10"))
    stations = stations.filter(
        (date.today().year - pl.col("start")) >= n_years
    )

    await _send(ws, "stations", convert_for_json(stations))


async def _handle_station_message(ws: WebSocket, msg_data: str | None) -> None:
    if msg_data is None:
        await _send(ws, "error", "No station was provided.")
        return

    # station is in the format `<name> (<id>)`
    id = msg_data.split()[-1][1:-1]

    try:
        metadata = await hydro.read_metadata(id)
        await _send(ws, "station", metadata._asdict())
    except ValueError as exc:
        await _send(ws, "error", str(exc))


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": convert_for_json(data)})
