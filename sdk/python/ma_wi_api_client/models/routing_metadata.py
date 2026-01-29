from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.actual_routing import ActualRouting
    from ..models.requested_routing import RequestedRouting


T = TypeVar("T", bound="RoutingMetadata")


@_attrs_define
class RoutingMetadata:
    """
    Attributes:
        requested_routing (RequestedRouting):
        actual_routing (ActualRouting):
    """

    requested_routing: "RequestedRouting"
    actual_routing: "ActualRouting"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        requested_routing = self.requested_routing.to_dict()

        actual_routing = self.actual_routing.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "requested_routing": requested_routing,
                "actual_routing": actual_routing,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.actual_routing import ActualRouting
        from ..models.requested_routing import RequestedRouting

        d = dict(src_dict)
        requested_routing = RequestedRouting.from_dict(d.pop("requested_routing"))

        actual_routing = ActualRouting.from_dict(d.pop("actual_routing"))

        routing_metadata = cls(
            requested_routing=requested_routing,
            actual_routing=actual_routing,
        )

        routing_metadata.additional_properties = d
        return routing_metadata

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
