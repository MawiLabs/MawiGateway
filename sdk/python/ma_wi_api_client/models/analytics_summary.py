from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="AnalyticsSummary")


@_attrs_define
class AnalyticsSummary:
    """
    Attributes:
        total_requests (int):
        successful_requests (int):
        failed_requests (int):
        total_cost_usd (float):
        total_tokens (int):
        avg_latency_ms (float):
        p95_latency_ms (float):
        p99_latency_ms (float):
    """

    total_requests: int
    successful_requests: int
    failed_requests: int
    total_cost_usd: float
    total_tokens: int
    avg_latency_ms: float
    p95_latency_ms: float
    p99_latency_ms: float
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        total_requests = self.total_requests

        successful_requests = self.successful_requests

        failed_requests = self.failed_requests

        total_cost_usd = self.total_cost_usd

        total_tokens = self.total_tokens

        avg_latency_ms = self.avg_latency_ms

        p95_latency_ms = self.p95_latency_ms

        p99_latency_ms = self.p99_latency_ms

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "total_requests": total_requests,
                "successful_requests": successful_requests,
                "failed_requests": failed_requests,
                "total_cost_usd": total_cost_usd,
                "total_tokens": total_tokens,
                "avg_latency_ms": avg_latency_ms,
                "p95_latency_ms": p95_latency_ms,
                "p99_latency_ms": p99_latency_ms,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        total_requests = d.pop("total_requests")

        successful_requests = d.pop("successful_requests")

        failed_requests = d.pop("failed_requests")

        total_cost_usd = d.pop("total_cost_usd")

        total_tokens = d.pop("total_tokens")

        avg_latency_ms = d.pop("avg_latency_ms")

        p95_latency_ms = d.pop("p95_latency_ms")

        p99_latency_ms = d.pop("p99_latency_ms")

        analytics_summary = cls(
            total_requests=total_requests,
            successful_requests=successful_requests,
            failed_requests=failed_requests,
            total_cost_usd=total_cost_usd,
            total_tokens=total_tokens,
            avg_latency_ms=avg_latency_ms,
            p95_latency_ms=p95_latency_ms,
            p99_latency_ms=p99_latency_ms,
        )

        analytics_summary.additional_properties = d
        return analytics_summary

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
