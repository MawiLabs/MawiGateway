from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="RequestedRouting")


@_attrs_define
class RequestedRouting:
    """
    Attributes:
        service (str):
        model_override (Union[Unset, str]):
        routing_strategy (Union[Unset, str]):
    """

    service: str
    model_override: Union[Unset, str] = UNSET
    routing_strategy: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        service = self.service

        model_override = self.model_override

        routing_strategy = self.routing_strategy

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "service": service,
            }
        )
        if model_override is not UNSET:
            field_dict["model_override"] = model_override
        if routing_strategy is not UNSET:
            field_dict["routing_strategy"] = routing_strategy

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        service = d.pop("service")

        model_override = d.pop("model_override", UNSET)

        routing_strategy = d.pop("routing_strategy", UNSET)

        requested_routing = cls(
            service=service,
            model_override=model_override,
            routing_strategy=routing_strategy,
        )

        requested_routing.additional_properties = d
        return requested_routing

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
