import numpy as np
import numpy.typing as npt
import polars as pl
from hydro_rs.snow import cemaneige

from ..hydro import Metadata
from ..utils import Data


def init() -> tuple[npt.NDArray[np.float64], npt.NDArray[np.float64]]:
    # parameters: ctg, kf, qnbv
    return cemaneige.init()


def simulate(
    params: npt.NDArray[np.float64], data: pl.DataFrame, metadata: Metadata
) -> npt.NDArray[np.float64]:
    _data = Data(
        data["precipitation"].to_numpy(),
        data["temperature"].to_numpy(),
        data["pet"].to_numpy(),
        data["date"].dt.ordinal_day().to_numpy().astype(np.uintp),
    )
    return cemaneige.simulate(params, _data, metadata)
