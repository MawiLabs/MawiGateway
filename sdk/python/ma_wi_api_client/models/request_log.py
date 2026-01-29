from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="RequestLog")


@_attrs_define
class RequestLog:
    """
    Attributes:
        id (str):
        service_name (str):
        model_id (str):
        provider_type (str):
        latency_ms (int):
        status (str):
        created_at (str):
        failover_count (int):
        tokens_prompt (Union[Unset, int]):
        tokens_completion (Union[Unset, int]):
        tokens_total (Union[Unset, int]):
        cost_usd (Union[Unset, float]):
        error_message (Union[Unset, str]):
    """

    id: str
    service_name: str
    model_id: str
    provider_type: str
    latency_ms: int
    status: str
    created_at: str
    failover_count: int
    tokens_prompt: Union[Unset, int] = UNSET
    tokens_completion: Union[Unset, int] = UNSET
    tokens_total: Union[Unset, int] = UNSET
    cost_usd: Union[Unset, float] = UNSET
    error_message: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        service_name = self.service_name

        model_id = self.model_id

        provider_type = self.provider_type

        latency_ms = self.latency_ms

        status = self.status

        created_at = self.created_at

        failover_count = self.failover_count

        tokens_prompt = self.tokens_prompt

        tokens_completion = self.tokens_completion

        tokens_total = self.tokens_total

        cost_usd = self.cost_usd

        error_message = self.error_message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "service_name": service_name,
                "model_id": model_id,
                "provider_type": provider_type,
                "latency_ms": latency_ms,
                "status": status,
                "created_at": created_at,
                "failover_count": failover_count,
            }
        )
        if tokens_prompt is not UNSET:
            field_dict["tokens_prompt"] = tokens_prompt
        if tokens_completion is not UNSET:
            field_dict["tokens_completion"] = tokens_completion
        if tokens_total is not UNSET:
            field_dict["tokens_total"] = tokens_total
        if cost_usd is not UNSET:
            field_dict["cost_usd"] = cost_usd
        if error_message is not UNSET:
            field_dict["error_message"] = error_message

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        service_name = d.pop("service_name")

        model_id = d.pop("model_id")

        provider_type = d.pop("provider_type")

        latency_ms = d.pop("latency_ms")

        status = d.pop("status")

        created_at = d.pop("created_at")

        failover_count = d.pop("failover_count")

        tokens_prompt = d.pop("tokens_prompt", UNSET)

        tokens_completion = d.pop("tokens_completion", UNSET)

        tokens_total = d.pop("tokens_total", UNSET)

        cost_usd = d.pop("cost_usd", UNSET)

        error_message = d.pop("error_message", UNSET)

        request_log = cls(
            id=id,
            service_name=service_name,
            model_id=model_id,
            provider_type=provider_type,
            latency_ms=latency_ms,
            status=status,
            created_at=created_at,
            failover_count=failover_count,
            tokens_prompt=tokens_prompt,
            tokens_completion=tokens_completion,
            tokens_total=tokens_total,
            cost_usd=cost_usd,
            error_message=error_message,
        )

        request_log.additional_properties = d
        return request_log

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
