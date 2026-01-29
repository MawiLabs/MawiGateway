from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.update_model_assignment import UpdateModelAssignment
from ...types import Response


def _get_kwargs(
    name: str,
    model_id: str,
    *,
    body: UpdateModelAssignment,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "put",
        "url": f"/services/{name}/models/{model_id}",
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json; charset=utf-8"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[str]:
    if response.status_code == 200:
        response_200 = cast(str, response.json())
        return response_200

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[str]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    name: str,
    model_id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    body: UpdateModelAssignment,
) -> Response[str]:
    """Update model assignment (weight, position, RTCROS)

    Args:
        name (str):
        model_id (str):
        body (UpdateModelAssignment):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[str]
    """

    kwargs = _get_kwargs(
        name=name,
        model_id=model_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    name: str,
    model_id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    body: UpdateModelAssignment,
) -> Optional[str]:
    """Update model assignment (weight, position, RTCROS)

    Args:
        name (str):
        model_id (str):
        body (UpdateModelAssignment):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        str
    """

    return sync_detailed(
        name=name,
        model_id=model_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    name: str,
    model_id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    body: UpdateModelAssignment,
) -> Response[str]:
    """Update model assignment (weight, position, RTCROS)

    Args:
        name (str):
        model_id (str):
        body (UpdateModelAssignment):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[str]
    """

    kwargs = _get_kwargs(
        name=name,
        model_id=model_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    name: str,
    model_id: str,
    *,
    client: Union[AuthenticatedClient, Client],
    body: UpdateModelAssignment,
) -> Optional[str]:
    """Update model assignment (weight, position, RTCROS)

    Args:
        name (str):
        model_id (str):
        body (UpdateModelAssignment):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        str
    """

    return (
        await asyncio_detailed(
            name=name,
            model_id=model_id,
            client=client,
            body=body,
        )
    ).parsed
