from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="UpdateModel")


@_attrs_define
class UpdateModel:
    """
    Attributes:
        name (Union[Unset, str]):
        provider (Union[Unset, str]):
        modality (Union[Unset, str]):
        description (Union[Unset, str]):
        cost_per_1k_tokens (Union[Unset, float]):
        cost_per_1k_input_tokens (Union[Unset, float]):
        cost_per_1k_output_tokens (Union[Unset, float]):
        tier (Union[Unset, str]):
        api_endpoint (Union[Unset, str]):
        api_version (Union[Unset, str]):
        api_key (Union[Unset, str]):
    """

    name: Union[Unset, str] = UNSET
    provider: Union[Unset, str] = UNSET
    modality: Union[Unset, str] = UNSET
    description: Union[Unset, str] = UNSET
    cost_per_1k_tokens: Union[Unset, float] = UNSET
    cost_per_1k_input_tokens: Union[Unset, float] = UNSET
    cost_per_1k_output_tokens: Union[Unset, float] = UNSET
    tier: Union[Unset, str] = UNSET
    api_endpoint: Union[Unset, str] = UNSET
    api_version: Union[Unset, str] = UNSET
    api_key: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        provider = self.provider

        modality = self.modality

        description = self.description

        cost_per_1k_tokens = self.cost_per_1k_tokens

        cost_per_1k_input_tokens = self.cost_per_1k_input_tokens

        cost_per_1k_output_tokens = self.cost_per_1k_output_tokens

        tier = self.tier

        api_endpoint = self.api_endpoint

        api_version = self.api_version

        api_key = self.api_key

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if name is not UNSET:
            field_dict["name"] = name
        if provider is not UNSET:
            field_dict["provider"] = provider
        if modality is not UNSET:
            field_dict["modality"] = modality
        if description is not UNSET:
            field_dict["description"] = description
        if cost_per_1k_tokens is not UNSET:
            field_dict["cost_per_1k_tokens"] = cost_per_1k_tokens
        if cost_per_1k_input_tokens is not UNSET:
            field_dict["cost_per_1k_input_tokens"] = cost_per_1k_input_tokens
        if cost_per_1k_output_tokens is not UNSET:
            field_dict["cost_per_1k_output_tokens"] = cost_per_1k_output_tokens
        if tier is not UNSET:
            field_dict["tier"] = tier
        if api_endpoint is not UNSET:
            field_dict["api_endpoint"] = api_endpoint
        if api_version is not UNSET:
            field_dict["api_version"] = api_version
        if api_key is not UNSET:
            field_dict["api_key"] = api_key

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name", UNSET)

        provider = d.pop("provider", UNSET)

        modality = d.pop("modality", UNSET)

        description = d.pop("description", UNSET)

        cost_per_1k_tokens = d.pop("cost_per_1k_tokens", UNSET)

        cost_per_1k_input_tokens = d.pop("cost_per_1k_input_tokens", UNSET)

        cost_per_1k_output_tokens = d.pop("cost_per_1k_output_tokens", UNSET)

        tier = d.pop("tier", UNSET)

        api_endpoint = d.pop("api_endpoint", UNSET)

        api_version = d.pop("api_version", UNSET)

        api_key = d.pop("api_key", UNSET)

        update_model = cls(
            name=name,
            provider=provider,
            modality=modality,
            description=description,
            cost_per_1k_tokens=cost_per_1k_tokens,
            cost_per_1k_input_tokens=cost_per_1k_input_tokens,
            cost_per_1k_output_tokens=cost_per_1k_output_tokens,
            tier=tier,
            api_endpoint=api_endpoint,
            api_version=api_version,
            api_key=api_key,
        )

        update_model.additional_properties = d
        return update_model

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
