import { Api } from "../generated/Api";

export { Api } from "../generated/Api";
export * from "../generated/data-contracts";
export { ContentType } from "../generated/http-client";
export type {
	FullRequestParams,
	HttpResponse,
	QueryParamsType,
	RequestParams,
	ResponseFormat,
} from "../generated/http-client";
export {
	HttpClient,
	HttpError,
	NetworkError,
	ParseError,
	TimeoutError,
} from "./base-client";
export type { ApiConfig } from "./base-client";

export interface CreateApiOptions {
	timeoutMs?: number;
	credentials?: RequestCredentials;
	headers?: Record<string, string>;
}

export function createApi(baseUrl: string, options: CreateApiOptions = {}) {
	return new Api({
		baseUrl,
		timeoutMs: options.timeoutMs,
		credentials: options.credentials,
		headers: options.headers,
	});
}
