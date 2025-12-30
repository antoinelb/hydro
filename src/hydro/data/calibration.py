from typing import Literal, assert_never

#########
# types #
#########

ObjectiveFunctions = Literal["rmse", "mse", "kge"]
Transformations = Literal["log", "sqrt", "none"]
Algorithms = Literal["sce"]

##########
# public #
##########


def get_algorithm_params(
    algorithm: Algorithms,
) -> dict[str, dict[str, int | float | None]]:
    match algorithm:
        case "sce":
            return {
                "ngs": {
                    "min": 1,
                    "max": None,
                    "default": 25,
                    "step": 1,
                },
                "npg": {
                    "min": 1,
                    "max": None,
                    "default": 9,
                    "step": 1,
                },
                "mings": {
                    "min": 1,
                    "max": None,
                    "default": 25,
                    "step": 1,
                },
                "nspl": {
                    "min": 1,
                    "max": None,
                    "default": 9,
                    "step": 1,
                },
                "maxn": {
                    "min": 1,
                    "max": None,
                    "default": 5000,
                    "step": 1,
                },
                "kstop": {
                    "min": 1,
                    "max": None,
                    "default": 10,
                    "step": 1,
                },
                "pcento": {
                    "min": 0,
                    "max": 1,
                    "default": 0.001,
                    "step": 0.001,
                },
                "peps": {
                    "min": 0,
                    "max": None,
                    "default": 0.1,
                    "step": 0.1,
                },
            }
        case _:
            assert_never(algorithm)
