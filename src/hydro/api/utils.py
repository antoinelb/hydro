import json
from datetime import date, datetime, timezone
from typing import Any, Awaitable, Callable

import polars as pl
from starlette.requests import Request
from starlette.responses import JSONResponse as _JSONResponse
from starlette.responses import PlainTextResponse, Response

from hydro.utils.data import NumericType

##########
# public #
##########


async def get_json_params(
    req: Request,
    args: list[str] | None = None,
    opt_args: list[str] | None = None,
) -> dict[str, Any] | Response:
    if args is None:
        args = []
    if opt_args is None:
        opt_args = []
    try:
        data = json.loads(await req.body())
    except ValueError:
        return PlainTextResponse("Wrong data type was sent.", status_code=400)
    try:
        args_ = {arg: data[arg] for arg in args}
        opt_args_ = {arg: data[arg] for arg in opt_args if arg in data}
    except KeyError:
        return PlainTextResponse(
            "There are missing parameters.", status_code=400
        )
    return {**args_, **opt_args_}


async def get_query_string_params(
    req: Request,
    args: list[str] | None = None,
    opt_args: list[str] | None = None,
) -> dict[str, Any] | Response:
    if args is None:
        args = []
    if opt_args is None:
        opt_args = []
    try:
        args_ = {arg: req.query_params[arg] for arg in args}
        opt_args_ = {
            arg: req.query_params[arg]
            for arg in opt_args
            if arg in req.query_params
        }
    except KeyError:
        return PlainTextResponse(
            "There are missing parameters.", status_code=400
        )
    return {**args_, **opt_args_}


async def get_path_params(
    req: Request,
    args: list[str] | None = None,
    opt_args: list[str] | None = None,
) -> dict[str, Any] | Response:
    if args is None:
        args = []
    if opt_args is None:
        opt_args = []
    try:
        args_ = {arg: req.path_params[arg] for arg in args}
        opt_args_ = {
            arg: req.path_params[arg]
            for arg in opt_args
            if arg in req.path_params
        }
    except KeyError:
        return PlainTextResponse(
            "There are missing parameters.", status_code=400
        )
    return {**args_, **opt_args_}


async def get_headers(
    req: Request,
    args: list[str] | None = None,
    opt_args: list[str] | None = None,
) -> dict[str, Any] | Response:
    if args is None:
        args = []
    if opt_args is None:
        opt_args = []
    try:
        args_ = {arg: req.headers[arg] for arg in args}
        opt_args_ = {
            arg: req.headers[arg] for arg in opt_args if arg in req.headers
        }
    except KeyError:
        return PlainTextResponse("There are missing headers.", status_code=400)
    return {**args_, **opt_args_}


def with_json_params(
    args: list[str] | str | None = None,
    opt_args: list[str] | str | None = None,
) -> Callable[[Callable], Callable]:
    def decorator(
        fct: Callable[..., Awaitable[Response]],
    ) -> Callable[..., Awaitable[Response]]:
        async def wrapper(
            req: Request, *args_: Any, **kwargs_: Any
        ) -> Response:
            params = await get_json_params(
                req,
                args=[args] if isinstance(args, str) else args,
                opt_args=[opt_args] if isinstance(opt_args, str) else opt_args,
            )
            if isinstance(params, Response):
                return params
            params = {
                arg.replace("-", "_"): val for arg, val in params.items()
            }
            return await fct(req, *args_, **kwargs_, **params)

        return wrapper

    return decorator


def with_query_string_params(
    args: list[str] | str | None = None,
    opt_args: list[str] | str | None = None,
) -> Callable[[Callable], Callable]:
    def decorator(
        fct: Callable[..., Awaitable[Response]],
    ) -> Callable[..., Awaitable[Response]]:
        async def wrapper(
            req: Request, *args_: Any, **kwargs_: Any
        ) -> Response:
            params = await get_query_string_params(
                req,
                args=[args] if isinstance(args, str) else args,
                opt_args=[opt_args] if isinstance(opt_args, str) else opt_args,
            )
            if isinstance(params, Response):
                return params
            params = {
                arg.replace("-", "_"): val for arg, val in params.items()
            }
            return await fct(req, *args_, **kwargs_, **params)

        return wrapper

    return decorator


def with_path_params(
    args: list[str] | str | None = None,
    opt_args: list[str] | str | None = None,
) -> Callable[[Callable], Callable]:
    def decorator(
        fct: Callable[..., Awaitable[Response]],
    ) -> Callable[..., Awaitable[Response]]:
        async def wrapper(
            req: Request, *args_: Any, **kwargs_: Any
        ) -> Response:
            params = await get_path_params(
                req,
                args=[args] if isinstance(args, str) else args,
                opt_args=[opt_args] if isinstance(opt_args, str) else opt_args,
            )
            if isinstance(params, Response):
                return params
            params = {
                arg.replace("-", "_"): val for arg, val in params.items()
            }
            return await fct(req, *args_, **kwargs_, **params)

        return wrapper

    return decorator


def with_headers(
    args: list[str] | str | None = None,
    opt_args: list[str] | str | None = None,
) -> Callable[[Callable], Callable]:
    def decorator(
        fct: Callable[..., Awaitable[Response]],
    ) -> Callable[..., Awaitable[Response]]:
        async def wrapper(
            req: Request, *args_: Any, **kwargs_: Any
        ) -> Response:
            params = await get_headers(
                req,
                args=[args] if isinstance(args, str) else args,
                opt_args=[opt_args] if isinstance(opt_args, str) else opt_args,
            )
            if isinstance(params, Response):
                return params
            params = {
                arg.replace("-", "_"): val for arg, val in params.items()
            }
            return await fct(req, *args_, **kwargs_, **params)

        return wrapper

    return decorator


def JSONResponse(data: Any, *args: Any, **kwargs: Any) -> _JSONResponse:
    return _JSONResponse(convert_for_json(data), *args, **kwargs)


def convert_for_json(data: Any) -> Any:
    if isinstance(data, dict):
        return {key: convert_for_json(val) for key, val in data.items()}
    elif isinstance(data, (list, tuple)):
        return [convert_for_json(val) for val in data]
    elif isinstance(data, datetime):
        return int(data.replace(tzinfo=timezone.utc).timestamp())
    elif isinstance(data, date):
        return int(
            datetime.combine(data, datetime.min.time())
            .replace(tzinfo=timezone.utc)
            .timestamp()
        )
    elif isinstance(data, pl.DataFrame):
        return [
            convert_for_json(d)
            for d in data.with_columns(
                pl.when(pl.col(NumericType).is_infinite())
                .then(None)
                .otherwise(pl.col(NumericType))
                .name.keep(),
                pl.col(pl.Date).dt.strftime("%Y-%m-%d"),
                pl.col(pl.Datetime).dt.strftime("%Y-%m-%d %H:%M:%S"),
            ).to_dicts()
        ]
    else:
        return data
