from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.modality import Modality
from ..models.pool_type import PoolType
from ..models.service_type import ServiceType
from ..types import UNSET, Unset

T = TypeVar("T", bound="Service")


@_attrs_define
class Service:
    """
    Attributes:
        name (str):
        service_type (ServiceType):
        strategy (str):
        input_modalities (list[Modality]):
        output_modalities (list[Modality]):
        description (Union[Unset, str]):
        guardrails (Union[Unset, str]):
        created_at (Union[Unset, int]):
        pool_type (Union[Unset, PoolType]):
        planner_model_id (Union[Unset, str]):
        system_prompt (Union[Unset, str]):
        max_iterations (Union[Unset, int]):
        user_id (Union[Unset, str]):
    """

    name: str
    service_type: ServiceType
    strategy: str
    input_modalities: list[Modality]
    output_modalities: list[Modality]
    description: Union[Unset, str] = UNSET
    guardrails: Union[Unset, str] = UNSET
    created_at: Union[Unset, int] = UNSET
    pool_type: Union[Unset, PoolType] = UNSET
    planner_model_id: Union[Unset, str] = UNSET
    system_prompt: Union[Unset, str] = UNSET
    max_iterations: Union[Unset, int] = UNSET
    user_id: Union[Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        service_type = self.service_type.value

        strategy = self.strategy

        input_modalities = []
        for input_modalities_item_data in self.input_modalities:
            input_modalities_item = input_modalities_item_data.value
            input_modalities.append(input_modalities_item)

        output_modalities = []
        for output_modalities_item_data in self.output_modalities:
            output_modalities_item = output_modalities_item_data.value
            output_modalities.append(output_modalities_item)

        description = self.description

        guardrails = self.guardrails

        created_at = self.created_at

        pool_type: Union[Unset, str] = UNSET
        if not isinstance(self.pool_type, Unset):
            pool_type = self.pool_type.value

        planner_model_id = self.planner_model_id

        system_prompt = self.system_prompt

        max_iterations = self.max_iterations

        user_id = self.user_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "service_type": service_type,
                "strategy": strategy,
                "input_modalities": input_modalities,
                "output_modalities": output_modalities,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description
        if guardrails is not UNSET:
            field_dict["guardrails"] = guardrails
        if created_at is not UNSET:
            field_dict["created_at"] = created_at
        if pool_type is not UNSET:
            field_dict["pool_type"] = pool_type
        if planner_model_id is not UNSET:
            field_dict["planner_model_id"] = planner_model_id
        if system_prompt is not UNSET:
            field_dict["system_prompt"] = system_prompt
        if max_iterations is not UNSET:
            field_dict["max_iterations"] = max_iterations
        if user_id is not UNSET:
            field_dict["user_id"] = user_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name")

        service_type = ServiceType(d.pop("service_type"))

        strategy = d.pop("strategy")

        input_modalities = []
        _input_modalities = d.pop("input_modalities")
        for input_modalities_item_data in _input_modalities:
            input_modalities_item = Modality(input_modalities_item_data)

            input_modalities.append(input_modalities_item)

        output_modalities = []
        _output_modalities = d.pop("output_modalities")
        for output_modalities_item_data in _output_modalities:
            output_modalities_item = Modality(output_modalities_item_data)

            output_modalities.append(output_modalities_item)

        description = d.pop("description", UNSET)

        guardrails = d.pop("guardrails", UNSET)

        created_at = d.pop("created_at", UNSET)

        _pool_type = d.pop("pool_type", UNSET)
        pool_type: Union[Unset, PoolType]
        if isinstance(_pool_type, Unset):
            pool_type = UNSET
        else:
            pool_type = PoolType(_pool_type)

        planner_model_id = d.pop("planner_model_id", UNSET)

        system_prompt = d.pop("system_prompt", UNSET)

        max_iterations = d.pop("max_iterations", UNSET)

        user_id = d.pop("user_id", UNSET)

        service = cls(
            name=name,
            service_type=service_type,
            strategy=strategy,
            input_modalities=input_modalities,
            output_modalities=output_modalities,
            description=description,
            guardrails=guardrails,
            created_at=created_at,
            pool_type=pool_type,
            planner_model_id=planner_model_id,
            system_prompt=system_prompt,
            max_iterations=max_iterations,
            user_id=user_id,
        )

        service.additional_properties = d
        return service

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
