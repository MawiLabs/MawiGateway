from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateService")


@_attrs_define
class CreateService:
    """
    Attributes:
        name (str):
        service_type (str):
        guardrails (list[str]):
        description (Union[Unset, str]):
        strategy (Union[Unset, str]):
        planner_model_id (Union[Unset, str]):
        system_prompt (Union[Unset, str]):
        max_iterations (Union[Unset, int]):
    """

    name: str
    service_type: str
    guardrails: list[str]
    description: Union[Unset, str] = UNSET
    strategy: Union[Unset, str] = UNSET
    planner_model_id: Union[Unset, str] = UNSET
    system_prompt: Union[Unset, str] = UNSET
    max_iterations: Union[Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        service_type = self.service_type

        guardrails = self.guardrails

        description = self.description

        strategy = self.strategy

        planner_model_id = self.planner_model_id

        system_prompt = self.system_prompt

        max_iterations = self.max_iterations

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "service_type": service_type,
                "guardrails": guardrails,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description
        if strategy is not UNSET:
            field_dict["strategy"] = strategy
        if planner_model_id is not UNSET:
            field_dict["planner_model_id"] = planner_model_id
        if system_prompt is not UNSET:
            field_dict["system_prompt"] = system_prompt
        if max_iterations is not UNSET:
            field_dict["max_iterations"] = max_iterations

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name")

        service_type = d.pop("service_type")

        guardrails = cast(list[str], d.pop("guardrails"))

        description = d.pop("description", UNSET)

        strategy = d.pop("strategy", UNSET)

        planner_model_id = d.pop("planner_model_id", UNSET)

        system_prompt = d.pop("system_prompt", UNSET)

        max_iterations = d.pop("max_iterations", UNSET)

        create_service = cls(
            name=name,
            service_type=service_type,
            guardrails=guardrails,
            description=description,
            strategy=strategy,
            planner_model_id=planner_model_id,
            system_prompt=system_prompt,
            max_iterations=max_iterations,
        )

        create_service.additional_properties = d
        return create_service

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
