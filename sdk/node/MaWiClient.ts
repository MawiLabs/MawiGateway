/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { BaseHttpRequest } from './core/BaseHttpRequest';
import type { OpenAPIConfig } from './core/OpenAPI';
import { AxiosHttpRequest } from './core/AxiosHttpRequest';
import { AnalyticsService } from './services/AnalyticsService';
import { AuthService } from './services/AuthService';
import { ChatService } from './services/ChatService';
import { DefaultService } from './services/DefaultService';
import { ModelsService } from './services/ModelsService';
import { ProvidersService } from './services/ProvidersService';
import { ServicesService } from './services/ServicesService';
import { SystemService } from './services/SystemService';
import { UserService } from './services/UserService';
type HttpRequestConstructor = new (config: OpenAPIConfig) => BaseHttpRequest;
export class MaWiClient {
    public readonly analytics: AnalyticsService;
    public readonly auth: AuthService;
    public readonly chat: ChatService;
    public readonly default: DefaultService;
    public readonly models: ModelsService;
    public readonly providers: ProvidersService;
    public readonly services: ServicesService;
    public readonly system: SystemService;
    public readonly user: UserService;
    public readonly request: BaseHttpRequest;
    constructor(config?: Partial<OpenAPIConfig>, HttpRequest: HttpRequestConstructor = AxiosHttpRequest) {
        this.request = new HttpRequest({
            BASE: config?.BASE ?? 'http://localhost:8030/v1',
            VERSION: config?.VERSION ?? '1.0',
            WITH_CREDENTIALS: config?.WITH_CREDENTIALS ?? false,
            CREDENTIALS: config?.CREDENTIALS ?? 'include',
            TOKEN: config?.TOKEN,
            USERNAME: config?.USERNAME,
            PASSWORD: config?.PASSWORD,
            HEADERS: config?.HEADERS,
            ENCODE_PATH: config?.ENCODE_PATH,
        });
        this.analytics = new AnalyticsService(this.request);
        this.auth = new AuthService(this.request);
        this.chat = new ChatService(this.request);
        this.default = new DefaultService(this.request);
        this.models = new ModelsService(this.request);
        this.providers = new ProvidersService(this.request);
        this.services = new ServicesService(this.request);
        this.system = new SystemService(this.request);
        this.user = new UserService(this.request);
    }
}

