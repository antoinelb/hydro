from typing import Any, get_args

from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import ClimateModels, SnowModels, calibration
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


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": data})
