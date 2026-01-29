from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="TimeSeriesPoint")


@_attrs_define
class TimeSeriesPoint:
    """
    Attributes:
        timestamp (str):
        request_count (int):
        error_count (int):
        avg_latency_ms (float):
        total_cost_usd (float):
        total_tokens (int):
    """

    timestamp: str
    request_count: int
    error_count: int
    avg_latency_ms: float
    total_cost_usd: float
    total_tokens: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        timestamp = self.timestamp

        request_count = self.request_count

        error_count = self.error_count

        avg_latency_ms = self.avg_latency_ms

        total_cost_usd = self.total_cost_usd

        total_tokens = self.total_tokens

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "timestamp": timestamp,
                "request_count": request_count,
                "error_count": error_count,
                "avg_latency_ms": avg_latency_ms,
                "total_cost_usd": total_cost_usd,
                "total_tokens": total_tokens,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        timestamp = d.pop("timestamp")

        request_count = d.pop("request_count")

        error_count = d.pop("error_count")

        avg_latency_ms = d.pop("avg_latency_ms")

        total_cost_usd = d.pop("total_cost_usd")

        total_tokens = d.pop("total_tokens")

        time_series_point = cls(
            timestamp=timestamp,
            request_count=request_count,
            error_count=error_count,
            avg_latency_ms=avg_latency_ms,
            total_cost_usd=total_cost_usd,
            total_tokens=total_tokens,
        )

        time_series_point.additional_properties = d
        return time_series_point

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
