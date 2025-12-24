import base64

import httpx
from starlette.requests import Request
from starlette.responses import FileResponse, Response
from starlette.routing import BaseRoute, Route

from hydro.utils import paths

from .utils import with_path_params

##########
# public #
##########


def get_routes() -> list[BaseRoute]:
    return [
        Route(
            "/tile/{z}/{x}/{y}.png",
            endpoint=_get_map_tile,
            methods=["GET"],
        ),
    ]


##########
# routes #
##########


@with_path_params(args=["x", "y", "z"])
async def _get_map_tile(_: Request, x: int, y: int, z: int) -> Response:
    path = paths.data_dir / "map" / f"tile_{z}_{x}_{y}.png"
    if path.exists():
        return FileResponse(str(path))
    else:
        # download the tile before sending the cached version
        if await _download_map_tile(x, y, z):
            return FileResponse(str(path))
        # return a black tile if the tile isn't available
        else:
            return Response(
                base64.b64decode(
                    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
                ),
                media_type="image/png",
            )


###########
# private #
###########


async def _download_map_tile(x: int, y: int, z: int) -> bool:
    url = "https://{s}.basemaps.cartocdn.com/{style}/{z}/{x}/{y}.png"
    subdomains = ["a", "b", "c", "d"]
    style = "dark_all"
    path = paths.data_dir / "map" / f"tile_{z}_{x}_{y}.png"

    subdomain = subdomains[(int(x) + int(y)) % len(subdomains)]
    url = url.format(s=subdomain, style=style, z=z, x=x, y=y)

    try:
        async with httpx.AsyncClient(timeout=30.0) as client:
            resp = await client.get(url)
            if resp.status_code == 200:
                path.parent.mkdir(exist_ok=True, parents=True)
                path.write_bytes(resp.content)
                return True
            else:
                return False
    except Exception:
        return False
