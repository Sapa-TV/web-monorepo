/* eslint-disable */
/* tslint:disable */
// @ts-nocheck
/*
 * ---------------------------------------------------------------
 * ## THIS FILE WAS GENERATED VIA SWAGGER-TYPESCRIPT-API        ##
 * ##                                                           ##
 * ## AUTHOR: acacode                                           ##
 * ## SOURCE: https://github.com/acacode/swagger-typescript-api ##
 * ---------------------------------------------------------------
 */

import {
  AddAdminRequest,
  AdminResponse,
  CreateSessionRequest,
  IngressCredentialsResponse,
  SessionResponse,
  StreamStatusResponse,
  TwitchAuthCallbackResponse,
  TwitchAuthStartResponse,
  TwitchLoginCallbackResponse,
  TwitchLoginStartResponse,
  WidgetAccessKeyResponse,
} from "./data-contracts";
import { ContentType, HttpClient, RequestParams } from "./http-client";

export class Api<
  SecurityDataType = unknown,
> extends HttpClient<SecurityDataType> {
  /**
   * No description
   *
   * @tags admin
   * @name ListAdmins
   * @request GET:/api/admin
   */
  listAdmins = (params: RequestParams = {}) =>
    this.request<AdminResponse[], any>({
      path: `/api/admin`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name AddAdmin
   * @request POST:/api/admin
   */
  addAdmin = (data: AddAdminRequest, params: RequestParams = {}) =>
    this.request<AdminResponse, void>({
      path: `/api/admin`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name GetIngressCredentials
   * @request GET:/api/admin/ingress/credentials
   */
  getIngressCredentials = (params: RequestParams = {}) =>
    this.request<IngressCredentialsResponse, any>({
      path: `/api/admin/ingress/credentials`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name RevokeIngressCredentials
   * @request DELETE:/api/admin/ingress/credentials
   */
  revokeIngressCredentials = (params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/ingress/credentials`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name StartTwitchAuth
   * @request GET:/api/admin/twitch/auth
   */
  startTwitchAuth = (params: RequestParams = {}) =>
    this.request<TwitchAuthStartResponse, void>({
      path: `/api/admin/twitch/auth`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name TwitchAuthCallback
   * @request GET:/api/admin/twitch/auth/callback
   */
  twitchAuthCallback = (
    code: string,
    state: string,
    params: RequestParams = {},
  ) =>
    this.request<TwitchAuthCallbackResponse, void>({
      path: `/api/admin/twitch/auth/callback`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name GetWidgetAccessKey
   * @request GET:/api/admin/widget-access-key
   */
  getWidgetAccessKey = (params: RequestParams = {}) =>
    this.request<WidgetAccessKeyResponse, any>({
      path: `/api/admin/widget-access-key`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name RotateWidgetAccessKey
   * @request POST:/api/admin/widget-access-key
   */
  rotateWidgetAccessKey = (params: RequestParams = {}) =>
    this.request<WidgetAccessKeyResponse, any>({
      path: `/api/admin/widget-access-key`,
      method: "POST",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name RemoveAdmin
   * @request DELETE:/api/admin/{twitch_id}
   */
  removeAdmin = (twitchId: string, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/${twitchId}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name StartTwitchLogin
   * @request GET:/api/auth/twitch
   */
  startTwitchLogin = (params: RequestParams = {}) =>
    this.request<TwitchLoginStartResponse, void>({
      path: `/api/auth/twitch`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name TwitchLoginCallback
   * @request GET:/api/auth/twitch/callback
   */
  twitchLoginCallback = (
    code: string,
    state: string,
    params: RequestParams = {},
  ) =>
    this.request<TwitchLoginCallbackResponse, void>({
      path: `/api/auth/twitch/callback`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name CreateSession
   * @request POST:/api/sessions
   */
  createSession = (data: CreateSessionRequest, params: RequestParams = {}) =>
    this.request<SessionResponse, void>({
      path: `/api/sessions`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name GetMe
   * @request GET:/api/sessions/me
   */
  getMe = (params: RequestParams = {}) =>
    this.request<SessionResponse, void>({
      path: `/api/sessions/me`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name Logout
   * @request DELETE:/api/sessions/me
   */
  logout = (params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/sessions/me`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags stream
   * @name GetStreamStatus
   * @request GET:/api/stream/status
   */
  getStreamStatus = (params: RequestParams = {}) =>
    this.request<StreamStatusResponse, any>({
      path: `/api/stream/status`,
      method: "GET",
      format: "json",
      ...params,
    });
}
