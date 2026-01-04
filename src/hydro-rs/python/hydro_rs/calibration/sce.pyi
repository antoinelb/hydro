from typing import final

import numpy as np
import numpy.typing as npt

from ..model import Data, Metadata

@final
class Sce:
    def __new__(
        cls,
        climate_model: str,
        snow_model: str | None,
        objective: str,
        n_complexes: int,
        k_stop: int,
        p_convergence_threshold: float,
        geometric_range_threshold: float,
        max_evaluations: int,
        seed: int,
    ) -> Sce: ...
    def init(
        self,
        data: Data,
        metadata: Metadata,
        observations: npt.NDArray[np.float64],
    ) -> None: ...
    def step(
        self,
        data: Data,
        metadata: Metadata,
        observations: npt.NDArray[np.float64],
    ) -> tuple[
        bool,
        npt.NDArray[np.float64],
        npt.NDArray[np.float64],
        npt.NDArray[np.float64],
    ]: ...
