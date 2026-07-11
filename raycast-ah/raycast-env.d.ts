/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** ah Binary Path - Path to ah binary (leave empty for auto-detect) */
  "ahPath"?: string,
  /** DeepSeek API Key - TX_DEEPSEEK_KEY */
  "deepseekKey"?: string,
  /** OpenAI API Key - TX_OPENAI_KEY */
  "openaiKey"?: string
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `explain-selected` command */
  export type ExplainSelected = ExtensionPreferences & {}
  /** Preferences accessible in the `explain-input` command */
  export type ExplainInput = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `explain-selected` command */
  export type ExplainSelected = {}
  /** Arguments passed to the `explain-input` command */
  export type ExplainInput = {
  /** map, useEffect, ... */
  "query": string
}
}

