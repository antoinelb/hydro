import numpy as np
import numpy.typing as npt

from ..model import Data, Metadata

def init() -> tuple[npt.NDArray[np.float64], npt.NDArray[np.float64]]: ...
def simulate(
    params: npt.NDArray[np.float64],
    data: Data,
    metadata: Metadata,
) -> npt.NDArray[np.float64]: ...
