from enum import Enum


class ServiceType(str, Enum):
    AGENTIC = "Agentic"
    POOL = "Pool"

    def __str__(self) -> str:
        return str(self.value)
