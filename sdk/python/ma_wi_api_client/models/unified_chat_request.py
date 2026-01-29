from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.routing_strategy import RoutingStrategy
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.chat_message import ChatMessage
    from ..models.chat_params import ChatParams
    from ..models.response_format import ResponseFormat


T = TypeVar("T", bound="UnifiedChatRequest")


@_attrs_define
class UnifiedChatRequest:
    """
    Attributes:
        service (str):
        messages (list['ChatMessage']):
        params (Union[Unset, ChatParams]):
        stream (Union[Unset, bool]):
        model (Union[Unset, str]):
        routing_strategy (Union[Unset, RoutingStrategy]): Routing strategies for POOL services
        response_format (Union[Unset, ResponseFormat]):
    """

    service: str
    messages: list["ChatMessage"]
    params: Union[Unset, "ChatParams"] = UNSET
    stream: Union[Unset, bool] = UNSET
    model: Union[Unset, str] = UNSET
    routing_strategy: Union[Unset, RoutingStrategy] = UNSET
    response_format: Union[Unset, "ResponseFormat"] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        service = self.service

        messages = []
        for messages_item_data in self.messages:
            messages_item = messages_item_data.to_dict()
            messages.append(messages_item)

        params: Union[Unset, dict[str, Any]] = UNSET
        if not isinstance(self.params, Unset):
            params = self.params.to_dict()

        stream = self.stream

        model = self.model

        routing_strategy: Union[Unset, str] = UNSET
        if not isinstance(self.routing_strategy, Unset):
            routing_strategy = self.routing_strategy.value

        response_format: Union[Unset, dict[str, Any]] = UNSET
        if not isinstance(self.response_format, Unset):
            response_format = self.response_format.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "service": service,
                "messages": messages,
            }
        )
        if params is not UNSET:
            field_dict["params"] = params
        if stream is not UNSET:
            field_dict["stream"] = stream
        if model is not UNSET:
            field_dict["model"] = model
        if routing_strategy is not UNSET:
            field_dict["routing_strategy"] = routing_strategy
        if response_format is not UNSET:
            field_dict["response_format"] = response_format

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.chat_message import ChatMessage
        from ..models.chat_params import ChatParams
        from ..models.response_format import ResponseFormat

        d = dict(src_dict)
        service = d.pop("service")

        messages = []
        _messages = d.pop("messages")
        for messages_item_data in _messages:
            messages_item = ChatMessage.from_dict(messages_item_data)

            messages.append(messages_item)

        _params = d.pop("params", UNSET)
        params: Union[Unset, ChatParams]
        if isinstance(_params, Unset):
            params = UNSET
        else:
            params = ChatParams.from_dict(_params)

        stream = d.pop("stream", UNSET)

        model = d.pop("model", UNSET)

        _routing_strategy = d.pop("routing_strategy", UNSET)
        routing_strategy: Union[Unset, RoutingStrategy]
        if isinstance(_routing_strategy, Unset):
            routing_strategy = UNSET
        else:
            routing_strategy = RoutingStrategy(_routing_strategy)

        _response_format = d.pop("response_format", UNSET)
        response_format: Union[Unset, ResponseFormat]
        if isinstance(_response_format, Unset):
            response_format = UNSET
        else:
            response_format = ResponseFormat.from_dict(_response_format)

        unified_chat_request = cls(
            service=service,
            messages=messages,
            params=params,
            stream=stream,
            model=model,
            routing_strategy=routing_strategy,
            response_format=response_format,
        )

        unified_chat_request.additional_properties = d
        return unified_chat_request

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
