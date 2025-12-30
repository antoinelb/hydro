from typing import Any, get_args

from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import (
    ClimateModels,
    SnowModels,
    calibration,
    read_datasets,
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
        case "observations":
            await _handle_observations_message(ws, msg.get("data", {}))


async def _handle_models_message(ws: WebSocket) -> None:
    models = {
        "snow": [*get_args(SnowModels), None],
        "climate": get_args(ClimateModels),
        "objectives": get_args(calibration.ObjectiveFunctions),
        "transformations": get_args(calibration.Transformations),
        "algorithms": {
            algorithm: calibration.get_algorithm_params(algorithm)
            for algorithm in get_args(calibration.Algorithms)
        },
    }
    await _send(ws, "models", convert_for_json(models))


async def _handle_observations_message(
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
    calib_data, _ = await read_datasets(
        id, msg_data["pet_model"], msg_data["n_valid_years"]
    )

    observations = calib_data.select("date", "discharge")

    await _send(ws, "observations", convert_for_json(observations))


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": data})
