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

/** Auto-generated discriminant enum variants */
export enum RuleTrigger {
  ChatMessage = "chat_message",
  RewardRedemption = "reward_redemption",
}

export enum QueueStatus {
  Pending = "Pending",
  Spinning = "Spinning",
  Completed = "Completed",
  Error = "Error",
  Cancelled = "Cancelled",
}

export enum MessageMatcher {
  Contains = "contains",
  StartsWith = "starts_with",
  Equals = "equals",
  EndsWith = "ends_with",
}

/**
 * @format int32
 * @min 0
 */
export type ActionId = number;

export interface ActionIdParam {
  /**
   * @format int32
   * @min 0
   */
  id: number;
}

export type ActionKind =
  | {
      type: "no_action";
    }
  | {
      type: "enqueue_roulette";
    }
  | {
      message_template: string;
      type: "chat_reply";
    };

export interface ActionResponse {
  created_at: string;
  enabled: boolean;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  kind: ActionKind;
  name: string;
  updated_at: string;
}

export interface AddAdminRequest {
  display_name?: string | null;
  twitch_id: string;
}

export interface AdminResponse {
  created_at: string;
  display_name?: string | null;
  is_root: boolean;
  twitch_id: string;
}

export interface AnonymousEnqueueRequest {
  name: string;
}

export interface CreateRarityRequest {
  color: string;
  display_name: string;
  image: string;
  name: string;
}

export interface CreateRouletteSlotRequest {
  action: string;
  name: string;
  rarity_id: RarityId;
  /**
   * @format int64
   * @min 0
   */
  weight: number;
}

export interface CreateSessionRequest {
  ticket: string;
}

export interface CreateUserRequest {
  display_name: string;
}

export interface EnqueueRequest {
  user_id: UserId;
  user_name: string;
}

export interface IngressCredentialsResponse {
  configured: boolean;
}

export interface LinkPlatformRequest {
  platform: string;
  platform_user_id: string;
  platform_username: string;
}

export interface MessageConditions {
  matcher: MessageMatcher;
  pattern?: string | null;
}

export interface NextResponse {
  entry: QueueEntryResponse;
  slot: RouletteSlot;
}

/**
 * @format int32
 * @min 0
 */
export type PlatformId = number;

export interface PlatformResponse {
  id: PlatformId;
  name: string;
}

/**
 * @format int32
 * @min 0
 */
export type QueueEntryId = number;

export interface QueueEntryResponse {
  created_at: string;
  id: QueueEntryId;
  result_slot_id?: null | RouletteSlotId;
  slot_name?: string | null;
  status: QueueStatus;
  updated_at: string;
  user_id: UserId;
  user_name: string;
}

export interface QueueListResponse {
  entries: QueueEntryResponse[];
  next_cursor?: null | QueueEntryId;
}

export interface QueueStats {
  /**
   * @format int32
   * @min 0
   */
  cancelled: number;
  /**
   * @format int32
   * @min 0
   */
  completed: number;
  /**
   * @format int32
   * @min 0
   */
  error: number;
  /**
   * @format int32
   * @min 0
   */
  pending: number;
  /**
   * @format int32
   * @min 0
   */
  spinning: number;
}

/**
 * @format int32
 * @min 0
 */
export type RarityId = number;

export interface RarityResponse {
  color: string;
  display_name: string;
  id: RarityId;
  image: string;
  name: string;
}

export interface RewardConditions {
  reward_id?: string | null;
}

export interface RewardResponse {
  /**
   * @format int64
   * @min 0
   */
  cost: number;
  id: string;
  is_enabled: boolean;
  is_paused: boolean;
  title: string;
  used_in_rules: boolean;
}

export interface RouletteSlot {
  action: string;
  id: RouletteSlotId;
  name: string;
  rarity_id: RarityId;
  /**
   * @format int64
   * @min 0
   */
  weight: number;
}

/**
 * @format int32
 * @min 0
 */
export type RouletteSlotId = number;

export interface RouletteSlotResponse {
  action: string;
  id: RouletteSlotId;
  name: string;
  rarity_id: RarityId;
  /**
   * @format int64
   * @min 0
   */
  weight: number;
}

export type RuleConditions =
  | (MessageConditions & {
      trigger: "chat_message";
    })
  | (RewardConditions & {
      trigger: "reward_redemption";
    });

export interface RuleIdParam {
  /**
   * @format int32
   * @min 0
   */
  id: number;
}

export interface RuleResponse {
  /**
   * @format int32
   * @min 0
   */
  action_id: number;
  conditions: RuleConditions;
  created_at: string;
  enabled: boolean;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  name: string;
  /** Auto-generated discriminant enum variants */
  trigger: RuleTrigger;
  updated_at: string;
}

export interface SessionResponse {
  expires_at: string;
  is_root: boolean;
  twitch_user_id: string;
  twitch_user_name?: string | null;
}

export interface SetStreamStatusRequest {
  online: boolean;
}

export interface StreamStatusResponse {
  online: boolean;
}

export interface TwitchAuthCallbackResponse {
  user_id: string;
  user_name?: string | null;
}

export interface TwitchAuthStartResponse {
  auth_url: string;
}

export interface TwitchLoginCallbackResponse {
  ticket: string;
  twitch_user_id: string;
  twitch_user_name?: string | null;
}

export interface TwitchLoginStartResponse {
  auth_url: string;
}

export interface UpdatePlatformRequest {
  platform_username: string;
}

export interface UpdateRarityRequest {
  color: string;
  display_name: string;
  image: string;
  name: string;
}

export interface UpdateRouletteSlotRequest {
  action: string;
  name: string;
  rarity_id: RarityId;
  /**
   * @format int64
   * @min 0
   */
  weight: number;
}

export interface UpdateUserRequest {
  display_name: string;
}

export interface UpsertActionRequest {
  enabled: boolean;
  kind: ActionKind;
  name: string;
}

export interface UpsertRuleRequest {
  action_id: ActionId;
  conditions: RuleConditions;
  enabled: boolean;
  name: string;
  /** Auto-generated discriminant enum variants */
  trigger: RuleTrigger;
}

/**
 * @format int32
 * @min 0
 */
export type UserId = number;

/**
 * @format int32
 * @min 0
 */
export type UserPlatformId = number;

export interface UserPlatformResponse {
  id: UserPlatformId;
  platform: string;
  platform_user_id: string;
  platform_username: string;
}

export interface UserResponse {
  created_at: string;
  display_name: string;
  id: UserId;
  platforms: UserPlatformResponse[];
  updated_at: string;
}

export interface WidgetAccessKeyResponse {
  widget_access_key: string;
}
