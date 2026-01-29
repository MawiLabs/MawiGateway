from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="TopModel")


@_attrs_define
class TopModel:
    """
    Attributes:
        model_id (str):
        model_name (str):
        request_count (int):
        total_cost (float):
    """

    model_id: str
    model_name: str
    request_count: int
    total_cost: float
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        model_id = self.model_id

        model_name = self.model_name

        request_count = self.request_count

        total_cost = self.total_cost

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "model_id": model_id,
                "model_name": model_name,
                "request_count": request_count,
                "total_cost": total_cost,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        model_id = d.pop("model_id")

        model_name = d.pop("model_name")

        request_count = d.pop("request_count")

        total_cost = d.pop("total_cost")

        top_model = cls(
            model_id=model_id,
            model_name=model_name,
            request_count=request_count,
            total_cost=total_cost,
        )

        top_model.additional_properties = d
        return top_model

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
