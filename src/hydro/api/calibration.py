import asyncio
from typing import Any, get_args

import polars as pl
from hydro_rs.metrics import calculate_kge, calculate_nse, calculate_rmse
from starlette.routing import BaseRoute, WebSocketRoute
from starlette.websockets import WebSocket, WebSocketDisconnect

from hydro.data import (
    ClimateModel,
    SnowModel,
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
                "snow": [*get_args(SnowModel), None],
                "climate": [
                    m for m in get_args(ClimateModel) if m != "day_median"
                ],
                "objectives": get_args(calibration.Objective),
                "transformations": get_args(calibration.Transformation),
                "algorithms": {
                    algorithm: calibration.get_algorithm_params(algorithm)
                    for algorithm in get_args(calibration.Algorithm)
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
    params = await calibration.calibrate(
        calib_data, metadata, "day_median", None, "rmse", "sce", {}
    )
    _predictions = model.simulate(params, calib_data, metadata)
    predictions = calib_data.select("date").with_columns(
        pl.Series("discharge", _predictions)
    )
    observations = calib_data.select("date", "discharge")
    results = {
        "rmse": calculate_rmse(
            observations["discharge"].to_numpy(),
            predictions["discharge"].to_numpy(),
        ),
        "nse": calculate_nse(
            observations["discharge"].to_numpy(),
            predictions["discharge"].to_numpy(),
        ),
        "kge": calculate_kge(
            observations["discharge"].to_numpy(),
            predictions["discharge"].to_numpy(),
        ),
    }

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
            "snow_model",
            "algorithm",
            "objective",
            "algorithm_params",
        )
    ):
        await _send(
            ws,
            "error",
            "`station`, `pet_model`, `n_valid_years`, `climate_model`, `snow_model`, `algorithm`, `objective` and `algorithm_params` must be provided.",
        )
        return

    # station is in the format `<name> (<id>)`
    id = msg_data["station"].split()[-1][1:-1]
    calib_data, _ = await read_datasets(
        id, msg_data["pet_model"], msg_data["n_valid_years"]
    )
    metadata = await hydro.read_metadata(id)

    params = calibration.calibrate(
        calib_data,
        metadata,
        msg_data["climate_model"],
        msg_data["snow_model"],
        msg_data["objective"],
        msg_data["algorithm"],
        msg_data["algorithm_params"],
    )

    async def callback(
        done: bool, predictions: pl.DataFrame, results: dict[str, float]
    ) -> None:
        print(results)
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

    params = await calibration.calibrate(
        calib_data,
        metadata,
        msg_data["climate_model"],
        msg_data["snow_model"],
        msg_data["objective"],
        msg_data["algorithm"],
        msg_data["algorithm_params"],
        callback=callback,
        stop_event=stop_event,
    )


###########
# private #
###########


async def _send(ws: WebSocket, event: str, data: Any) -> None:
    await ws.send_json({"type": event, "data": convert_for_json(data)})
