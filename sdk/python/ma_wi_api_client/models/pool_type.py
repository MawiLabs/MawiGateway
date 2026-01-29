from enum import Enum


class PoolType(str, Enum):
    MULTIMODALITY = "MultiModality"
    SINGLEMODALITY = "SingleModality"

    def __str__(self) -> str:
        return str(self.value)
