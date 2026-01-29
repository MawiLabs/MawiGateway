/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Model } from './Model';
import type { ProviderResponse } from './ProviderResponse';
import type { ServiceWithModels } from './ServiceWithModels';
export type TopologyResponse = {
    providers: Array<ProviderResponse>;
    services: Array<ServiceWithModels>;
    models: Array<Model>;
};

