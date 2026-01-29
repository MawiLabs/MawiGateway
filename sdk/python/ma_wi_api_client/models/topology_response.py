from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.model import Model
    from ..models.provider_response import ProviderResponse
    from ..models.service_with_models import ServiceWithModels


T = TypeVar("T", bound="TopologyResponse")


@_attrs_define
class TopologyResponse:
    """
    Attributes:
        providers (list['ProviderResponse']):
        services (list['ServiceWithModels']):
        models (list['Model']):
    """

    providers: list["ProviderResponse"]
    services: list["ServiceWithModels"]
    models: list["Model"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        providers = []
        for providers_item_data in self.providers:
            providers_item = providers_item_data.to_dict()
            providers.append(providers_item)

        services = []
        for services_item_data in self.services:
            services_item = services_item_data.to_dict()
            services.append(services_item)

        models = []
        for models_item_data in self.models:
            models_item = models_item_data.to_dict()
            models.append(models_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "providers": providers,
                "services": services,
                "models": models,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.model import Model
        from ..models.provider_response import ProviderResponse
        from ..models.service_with_models import ServiceWithModels

        d = dict(src_dict)
        providers = []
        _providers = d.pop("providers")
        for providers_item_data in _providers:
            providers_item = ProviderResponse.from_dict(providers_item_data)

            providers.append(providers_item)

        services = []
        _services = d.pop("services")
        for services_item_data in _services:
            services_item = ServiceWithModels.from_dict(services_item_data)

            services.append(services_item)

        models = []
        _models = d.pop("models")
        for models_item_data in _models:
            models_item = Model.from_dict(models_item_data)

            models.append(models_item)

        topology_response = cls(
            providers=providers,
            services=services,
            models=models,
        )

        topology_response.additional_properties = d
        return topology_response

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
