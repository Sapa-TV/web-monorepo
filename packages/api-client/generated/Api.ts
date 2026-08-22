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
  ActionResponse,
  AddAdminRequest,
  AdminResponse,
  CreateSessionRequest,
  IngressCredentialsResponse,
  RarityId,
  RarityResponse,
  RewardResponse,
  RouletteSlotId,
  RouletteSlotResponse,
  RuleResponse,
  SessionResponse,
  StreamStatusResponse,
  TwitchAuthCallbackResponse,
  TwitchAuthStartResponse,
  TwitchLoginCallbackResponse,
  TwitchLoginStartResponse,
  TwitchUserResponse,
  UpsertActionRequest,
  UpsertRarityRequest,
  UpsertRouletteSlotRequest,
  UpsertRuleRequest,
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
   * @name ListActions
   * @request GET:/api/admin/actions
   */
  listActions = (params: RequestParams = {}) =>
    this.request<ActionResponse[], any>({
      path: `/api/admin/actions`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateAction
   * @request POST:/api/admin/actions
   */
  createAction = (data: UpsertActionRequest, params: RequestParams = {}) =>
    this.request<ActionResponse, any>({
      path: `/api/admin/actions`,
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
   * @name UpdateAction
   * @request PUT:/api/admin/actions/{id}
   */
  updateAction = (
    id: number,
    data: UpsertActionRequest,
    params: RequestParams = {},
  ) =>
    this.request<ActionResponse, void>({
      path: `/api/admin/actions/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteAction
   * @request DELETE:/api/admin/actions/{id}
   */
  deleteAction = (id: number, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/actions/${id}`,
      method: "DELETE",
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
   * @name ListRewards
   * @request GET:/api/admin/rewards
   */
  listRewards = (params: RequestParams = {}) =>
    this.request<RewardResponse[], void>({
      path: `/api/admin/rewards`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListRarities
   * @request GET:/api/admin/roulette/rarities
   */
  listRarities = (params: RequestParams = {}) =>
    this.request<RarityResponse[], any>({
      path: `/api/admin/roulette/rarities`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateRarity
   * @request POST:/api/admin/roulette/rarities
   */
  createRarity = (data: UpsertRarityRequest, params: RequestParams = {}) =>
    this.request<RarityResponse, any>({
      path: `/api/admin/roulette/rarities`,
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
   * @name UpdateRarity
   * @request PUT:/api/admin/roulette/rarities/{id}
   */
  updateRarity = (
    id: RarityId,
    data: UpsertRarityRequest,
    params: RequestParams = {},
  ) =>
    this.request<RarityResponse, void>({
      path: `/api/admin/roulette/rarities/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteRarity
   * @request DELETE:/api/admin/roulette/rarities/{id}
   */
  deleteRarity = (id: RarityId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/roulette/rarities/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListSlots
   * @request GET:/api/admin/roulette/slots
   */
  listSlots = (params: RequestParams = {}) =>
    this.request<RouletteSlotResponse[], any>({
      path: `/api/admin/roulette/slots`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateSlot
   * @request POST:/api/admin/roulette/slots
   */
  createSlot = (data: UpsertRouletteSlotRequest, params: RequestParams = {}) =>
    this.request<RouletteSlotResponse, any>({
      path: `/api/admin/roulette/slots`,
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
   * @name UpdateSlot
   * @request PUT:/api/admin/roulette/slots/{id}
   */
  updateSlot = (
    id: RouletteSlotId,
    data: UpsertRouletteSlotRequest,
    params: RequestParams = {},
  ) =>
    this.request<RouletteSlotResponse, void>({
      path: `/api/admin/roulette/slots/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteSlot
   * @request DELETE:/api/admin/roulette/slots/{id}
   */
  deleteSlot = (id: RouletteSlotId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/roulette/slots/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListRules
   * @request GET:/api/admin/rules
   */
  listRules = (params: RequestParams = {}) =>
    this.request<RuleResponse[], any>({
      path: `/api/admin/rules`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateRule
   * @request POST:/api/admin/rules
   */
  createRule = (data: UpsertRuleRequest, params: RequestParams = {}) =>
    this.request<RuleResponse, void>({
      path: `/api/admin/rules`,
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
   * @name UpdateRule
   * @request PUT:/api/admin/rules/{id}
   */
  updateRule = (
    id: number,
    data: UpsertRuleRequest,
    params: RequestParams = {},
  ) =>
    this.request<RuleResponse, void>({
      path: `/api/admin/rules/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteRule
   * @request DELETE:/api/admin/rules/{id}
   */
  deleteRule = (id: number, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/rules/${id}`,
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
    query: {
      code: string;
      state: string;
    },
    params: RequestParams = {},
  ) =>
    this.request<TwitchAuthCallbackResponse, void>({
      path: `/api/admin/twitch/auth/callback`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name FindTwitchUser
   * @request GET:/api/admin/twitch/users
   */
  findTwitchUser = (
    query: {
      login: string;
    },
    params: RequestParams = {},
  ) =>
    this.request<TwitchUserResponse, void>({
      path: `/api/admin/twitch/users`,
      method: "GET",
      query: query,
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
    query: {
      code: string;
      state: string;
    },
    params: RequestParams = {},
  ) =>
    this.request<TwitchLoginCallbackResponse, void>({
      path: `/api/auth/twitch/callback`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags system
   * @name Health
   * @request GET:/api/health
   */
  health = (params: RequestParams = {}) =>
    this.request<string, any>({
      path: `/api/health`,
      method: "GET",
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
  /**
   * No description
   *
   * @tags system
   * @name Version
   * @request GET:/api/version
   */
  version = (params: RequestParams = {}) =>
    this.request<object, any>({
      path: `/api/version`,
      method: "GET",
      format: "json",
      ...params,
    });
}
