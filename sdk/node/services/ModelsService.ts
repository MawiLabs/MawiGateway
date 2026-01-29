/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { CreateModel } from '../models/CreateModel';
import type { Model } from '../models/Model';
import type { UpdateModel } from '../models/UpdateModel';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class ModelsService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * List all models with health status
     * @returns any
     * @throws ApiError
     */
    public getModels(): CancelablePromise<Array<any>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/models',
        });
    }
    /**
     * Create model
     * @param requestBody
     * @returns Model
     * @throws ApiError
     */
    public postModels(
        requestBody: CreateModel,
    ): CancelablePromise<Model> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/models',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Get model by ID
     * @param id
     * @returns Model
     * @throws ApiError
     */
    public getModels1(
        id: string,
    ): CancelablePromise<Model> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/models/{id}',
            path: {
                'id': id,
            },
        });
    }
    /**
     * Update model
     * @param id
     * @param requestBody
     * @returns Model
     * @throws ApiError
     */
    public putModels(
        id: string,
        requestBody: UpdateModel,
    ): CancelablePromise<Model> {
        return this.httpRequest.request({
            method: 'PUT',
            url: '/models/{id}',
            path: {
                'id': id,
            },
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Delete model
     * @param id
     * @returns string
     * @throws ApiError
     */
    public deleteModels(
        id: string,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'DELETE',
            url: '/models/{id}',
            path: {
                'id': id,
            },
        });
    }
}
