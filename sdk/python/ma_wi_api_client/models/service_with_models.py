from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.service import Service
    from ..models.service_model_info import ServiceModelInfo


T = TypeVar("T", bound="ServiceWithModels")


@_attrs_define
class ServiceWithModels:
    """
    Attributes:
        service (Service):
        models (list['ServiceModelInfo']):
    """

    service: "Service"
    models: list["ServiceModelInfo"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        service = self.service.to_dict()

        models = []
        for models_item_data in self.models:
            models_item = models_item_data.to_dict()
            models.append(models_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "service": service,
                "models": models,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.service import Service
        from ..models.service_model_info import ServiceModelInfo

        d = dict(src_dict)
        service = Service.from_dict(d.pop("service"))

        models = []
        _models = d.pop("models")
        for models_item_data in _models:
            models_item = ServiceModelInfo.from_dict(models_item_data)

            models.append(models_item)

        service_with_models = cls(
            service=service,
            models=models,
        )

        service_with_models.additional_properties = d
        return service_with_models

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
