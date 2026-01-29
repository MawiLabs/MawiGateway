/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { UpdateModelAssignment } from '../models/UpdateModelAssignment';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class DefaultService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * Update model assignment (weight, position, RTCROS)
     * @param name
     * @param modelId
     * @param requestBody
     * @returns string
     * @throws ApiError
     */
    public putServicesModels(
        name: string,
        modelId: string,
        requestBody: UpdateModelAssignment,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'PUT',
            url: '/services/{name}/models/{model_id}',
            path: {
                'name': name,
                'model_id': modelId,
            },
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Remove model from service
     * @param name
     * @param modelId
     * @returns string
     * @throws ApiError
     */
    public deleteServicesModels(
        name: string,
        modelId: string,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'DELETE',
            url: '/services/{name}/models/{model_id}',
            path: {
                'name': name,
                'model_id': modelId,
            },
        });
    }
}
