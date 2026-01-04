import numpy as np
import numpy.typing as npt

def calculate_rmse(
    observations: npt.NDArray[np.float64],
    simulations: npt.NDArray[np.float64],
) -> float: ...

def calculate_nse(
    observations: npt.NDArray[np.float64],
    simulations: npt.NDArray[np.float64],
) -> float: ...

def calculate_kge(
    observations: npt.NDArray[np.float64],
    simulations: npt.NDArray[np.float64],
) -> float: ...
