from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="Model")


@_attrs_define
class Model:
    """
    Attributes:
        id (str):
        name (str):
        provider (str):
        modality (str):
        tier (str):
        avg_latency_ms (int):
        avg_ttft_ms (int):
        max_tps (int):
        tier_required (str):
        worker_type (str):
        description (Union[Unset, str]):
        cost_per_1k_tokens (Union[Unset, float]):
        cost_per_1k_input_tokens (Union[Unset, float]):
        cost_per_1k_output_tokens (Union[Unset, float]):
        api_endpoint (Union[Unset, str]):
        api_version (Union[Unset, str]):
        api_key (Union[Unset, str]):
        created_at (Union[Unset, int]):
        created_by (Union[Unset, str]):
        user_id (Union[Unset, str]):
    """

    id: str
    name: str
    provider: str
    modality: str
    tier: str
    avg_latency_ms: int
    avg_ttft_ms: int
    max_tps: int
    tier_required: str
    worker_type: str
    description: Union[Unset, str] = UNSET
    cost_per_1k_tokens: Union[Unset, float] = UNSET
    cost_per_1k_input_tokens: Union[Unset, float] = UNSET
    cost_per_1k_output_tokens: Union[Unset, float] = UNSET
    api_endpoint: Union[Unset, str] = UNSET
    api_version: Union[Unset, str] = UNSET
    api_key: Union[Unset, str] = UNSET
    created_at: Union[Unset, int] = UNSET
    created_by: Union[Unset, str] = UNSET
    user_id: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        id = self.id

        name = self.name

        provider = self.provider

        modality = self.modality

        tier = self.tier

        avg_latency_ms = self.avg_latency_ms

        avg_ttft_ms = self.avg_ttft_ms

        max_tps = self.max_tps

        tier_required = self.tier_required

        worker_type = self.worker_type

        description = self.description

        cost_per_1k_tokens = self.cost_per_1k_tokens

        cost_per_1k_input_tokens = self.cost_per_1k_input_tokens

        cost_per_1k_output_tokens = self.cost_per_1k_output_tokens

        api_endpoint = self.api_endpoint

        api_version = self.api_version

        api_key = self.api_key

        created_at = self.created_at

        created_by = self.created_by

        user_id = self.user_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "name": name,
                "provider": provider,
                "modality": modality,
                "tier": tier,
                "avg_latency_ms": avg_latency_ms,
                "avg_ttft_ms": avg_ttft_ms,
                "max_tps": max_tps,
                "tier_required": tier_required,
                "worker_type": worker_type,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description
        if cost_per_1k_tokens is not UNSET:
            field_dict["cost_per_1k_tokens"] = cost_per_1k_tokens
        if cost_per_1k_input_tokens is not UNSET:
            field_dict["cost_per_1k_input_tokens"] = cost_per_1k_input_tokens
        if cost_per_1k_output_tokens is not UNSET:
            field_dict["cost_per_1k_output_tokens"] = cost_per_1k_output_tokens
        if api_endpoint is not UNSET:
            field_dict["api_endpoint"] = api_endpoint
        if api_version is not UNSET:
            field_dict["api_version"] = api_version
        if api_key is not UNSET:
            field_dict["api_key"] = api_key
        if created_at is not UNSET:
            field_dict["created_at"] = created_at
        if created_by is not UNSET:
            field_dict["created_by"] = created_by
        if user_id is not UNSET:
            field_dict["user_id"] = user_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        id = d.pop("id")

        name = d.pop("name")

        provider = d.pop("provider")

        modality = d.pop("modality")

        tier = d.pop("tier")

        avg_latency_ms = d.pop("avg_latency_ms")

        avg_ttft_ms = d.pop("avg_ttft_ms")

        max_tps = d.pop("max_tps")

        tier_required = d.pop("tier_required")

        worker_type = d.pop("worker_type")

        description = d.pop("description", UNSET)

        cost_per_1k_tokens = d.pop("cost_per_1k_tokens", UNSET)

        cost_per_1k_input_tokens = d.pop("cost_per_1k_input_tokens", UNSET)

        cost_per_1k_output_tokens = d.pop("cost_per_1k_output_tokens", UNSET)

        api_endpoint = d.pop("api_endpoint", UNSET)

        api_version = d.pop("api_version", UNSET)

        api_key = d.pop("api_key", UNSET)

        created_at = d.pop("created_at", UNSET)

        created_by = d.pop("created_by", UNSET)

        user_id = d.pop("user_id", UNSET)

        model = cls(
            id=id,
            name=name,
            provider=provider,
            modality=modality,
            tier=tier,
            avg_latency_ms=avg_latency_ms,
            avg_ttft_ms=avg_ttft_ms,
            max_tps=max_tps,
            tier_required=tier_required,
            worker_type=worker_type,
            description=description,
            cost_per_1k_tokens=cost_per_1k_tokens,
            cost_per_1k_input_tokens=cost_per_1k_input_tokens,
            cost_per_1k_output_tokens=cost_per_1k_output_tokens,
            api_endpoint=api_endpoint,
            api_version=api_version,
            api_key=api_key,
            created_at=created_at,
            created_by=created_by,
            user_id=user_id,
        )

        model.additional_properties = d
        return model

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
