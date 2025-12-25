from datetime import date
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
    stations: pl.DataFrame | None
    current_station: str | None


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
    data = {}
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
        case "stations":
            return await _handle_stations_message(
                ws, msg.get("data", {}), data
            )
        case "station":
            return await _handle_station_message(
                ws, msg.get("data", None), data
            )


async def _handle_stations_message(
    ws: WebSocket, msg_data: dict[str, Any], data: Data
) -> Data:
    stations = hydro.read_stations()
    data = {**data, "stations": stations}

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

    return data


async def _handle_station_message(
    ws: WebSocket, msg_data: str | None, data: Data
) -> Data:
    if msg_data is None:
        await _send(ws, "error", "No station was provided.")
        return data

    station = msg_data

    stations = data.get("stations")
    if stations is None:
        stations = hydro.read_stations()
        data = {**data, "stations": stations}

    stations = stations.filter(pl.col("station") == station)

    if stations.shape[0] == 0:
        await _send(ws, "error", f"The station {station} doesn't exist.")
        return data
    else:
        station = stations[0, "station"]
        await _send(
            ws,
            "station",
            {
                "station": stations[0, "station"],
                "lat": stations[0, "lat"],
                "lon": stations[0, "lon"],
            },
        )
        return {
            **data,
            "current_station": station,
        }


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": data})
