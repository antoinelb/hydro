import asyncio
from typing import Any, get_args

import polars as pl
from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import (
    ClimateModels,
    SnowModels,
    calibration,
    climate,
    hydro,
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
            models = {
                "snow": [*get_args(SnowModels), None],
                "climate": [
                    m for m in get_args(ClimateModels) if m != "day_median"
                ],
                "objectives": get_args(calibration.ObjectiveFunctions),
                "transformations": get_args(calibration.Transformations),
                "algorithms": {
                    algorithm: calibration.get_algorithm_params(algorithm)
                    for algorithm in get_args(calibration.Algorithms)
                },
            }
            await _send(ws, "models", models)
        case "observations":
            await _handle_observations_message(ws, msg.get("data", {}))
        case "calibration_start":
            msg_data = msg.get("data", {})
            if "climate_model" not in msg_data:
                await _send(
                    ws,
                    "error",
                    "`climate_model` must be provided.",
                )
                return
            stop_event = asyncio.Event()
            setattr(
                ws.state, f"{msg_data['climate_model']}_stop_event", stop_event
            )
            asyncio.create_task(
                _handle_calibration_start_message(
                    ws, msg.get("data", {}), stop_event
                )
            )
        case "calibration_stop":
            model = msg.get("data", None)
            if model is None:
                await _send(
                    ws,
                    "error",
                    "The model must be provided.",
                )
                return
            if hasattr(ws.state, f"{model}_stop_event"):
                getattr(ws.state, f"{model}_stop_event").set()
        case _:
            await _send(ws, "error", f"Unknown message type {msg['type']}.")


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
            "`station`, `pet_model` and `n_valid_years` must be provided.",
        )
        return

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    calib_data, _ = await read_datasets(
        id, msg_data["pet_model"], msg_data["n_valid_years"]
    )
    metadata = await hydro.read_metadata(id)

    model = climate.get_model("day_median")
    model = await model.calibrate(calib_data, metadata)
    _predictions = model(calib_data)
    predictions = calib_data.select("date").with_columns(
        pl.Series("discharge", _predictions)
    )
    results = climate.evaluate_model(
        calib_data["discharge"].to_numpy(), _predictions
    )

    observations = calib_data.select("date", "discharge")

    await _send(
        ws,
        "observations",
        {
            "observations": observations,
            "day_median": {
                "predictions": predictions,
                "results": results,
            },
        },
    )


async def _handle_calibration_start_message(
    ws: WebSocket, msg_data: dict[str, Any], stop_event: asyncio.Event
) -> None:
    if any(
        key not in msg_data
        for key in (
            "station",
            "pet_model",
            "n_valid_years",
            "climate_model",
            "algorithm",
            "objective",
            "algorithm_params",
        )
    ):
        await _send(
            ws,
            "error",
            "`station`, `pet_model`, `n_valid_years`, `climate_model`, `algorithm`, `objective` and `algorithm_params` must be provided.",
        )
        return

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    calib_data, _ = await read_datasets(
        id, msg_data["pet_model"], msg_data["n_valid_years"]
    )
    metadata = await hydro.read_metadata(id)

    model = climate.get_model(msg_data["climate_model"])

    async def callback(
        done: bool, predictions: pl.DataFrame, results: dict[str, float]
    ) -> None:
        await _send(
            ws,
            "calibration_step",
            {
                "done": done,
                "model": msg_data["climate_model"],
                "predictions": predictions,
                "results": results,
            },
        )

    model = await model.calibrate(
        calib_data,
        metadata,
        algorithm=msg_data["algorithm"],
        objective=msg_data["objective"],
        algorithm_params=msg_data["algorithm_params"],
        callback=callback,
        stop_event=stop_event,
    )


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": convert_for_json(data)})
