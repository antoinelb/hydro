import tomllib

from starlette.requests import Request
from starlette.responses import HTMLResponse, PlainTextResponse, Response
from starlette.routing import BaseRoute, Mount, Route
from starlette.staticfiles import StaticFiles


#########
# types #
#########

pyproject_path = utils.paths.root_dir / "pyproject.toml"
static_dir = utils.paths.root_dir / "src" / "static"

##########
# public #
##########


def get_routes() -> list[BaseRoute]:
    return [
        Route("/", endpoint=_index, methods=["GET"]),
        Route("/ping", endpoint=_ping, methods=["GET"]),
        Route("/version", endpoint=_get_version, methods=["GET"]),
        Mount(
            "/static",
            app=StaticFiles(directory=str(static_dir.absolute())),
        ),
        Mount("/map", routes=map.get_routes()),
        Mount("/data", routes=data.get_routes()),
    ]


##########
# routes #
##########


async def _ping(_: Request) -> Response:
    return PlainTextResponse("Pong!")


async def _get_version(_: Request) -> Response:
    with open(pyproject_path, "rb") as f:
        config = tomllib.load(f)
    if "project" in config and "version" in config["project"]:
        return PlainTextResponse(config["project"]["version"])
    else:
        return PlainTextResponse("Unknown version", 500)


async def _index(_: Request) -> Response:
    with open(static_dir / "index.html") as f:
        index = f.read()
    return HTMLResponse(index)
