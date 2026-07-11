import { getPreferenceValues } from "@raycast/api";
import { execSync } from "child_process";

interface Preferences {
  ahPath?: string;
  deepseekKey?: string;
  openaiKey?: string;
}

export interface ExplainResult {
  translation: string;
  explanation: string;
  usage: string;
}

function findAhPath(): string {
  const prefs = getPreferenceValues<Preferences>();
  if (prefs.ahPath) return prefs.ahPath;

  const candidates = [
    "/usr/local/bin/ah",
    "/opt/homebrew/bin/ah",
    `${process.env.HOME}/.local/bin/ah`,
    `${process.env.HOME}/.cargo/bin/ah`,
    "ah",
  ];

  for (const p of candidates) {
    try {
      execSync(`"${p}" --version 2>/dev/null`, { stdio: "ignore" });
      return p;
    } catch {}
  }
  return "ah";
}

function buildEnv(): Record<string, string> {
  const prefs = getPreferenceValues<Preferences>();
  const env: Record<string, string> = {};
  for (const [k, v] of Object.entries(process.env)) {
    if (v !== undefined) env[k] = v;
  }
  if (prefs.deepseekKey) env.TX_DEEPSEEK_KEY = prefs.deepseekKey;
  if (prefs.openaiKey) env.TX_OPENAI_KEY = prefs.openaiKey;
  return env;
}

function escapeShellArg(s: string): string {
  // Escape single quotes for shell: 'text' with single quotes escaped as '\'' (end quote, escaped quote, start quote)
  return `'${s.replace(/'/g, "'\\''")}'`;
}

export function explain(text: string, expand?: boolean): ExplainResult & { raw: string } {
  const bin = findAhPath();
  const args = expand ? ["explain", "--expand", "--json"] : ["explain", "--json"];
  const input = text.trim().replace(/\n/g, " ");
  const env = buildEnv();

  let stdout: string;
  try {
    stdout = execSync(`${escapeShellArg(bin)} ${args.join(" ")} ${escapeShellArg(input)}`, {
      encoding: "utf-8",
      timeout: 30000,
      env,
    }).trim();
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("command not found") || msg.includes("ENOENT")) {
      throw new Error(
        "`ah` not found.\n\nInstall: cargo install --path /path/to/ah\nThen set the path in Raycast extension preferences."
      );
    }
    if (msg.includes("No available AI provider") || msg.includes("API Key")) {
      throw new Error(
        "API Key not configured.\n\nIn Raycast Settings → Extensions → ah → set your DeepSeek or OpenAI key."
      );
    }
    throw new Error(`ah failed: ${msg}`);
  }

  try {
    const parsed = JSON.parse(stdout) as ExplainResult;
    return { ...parsed, raw: stdout };
  } catch {
    const clean = stdout.replace(/\u001b\[\d+m/g, "");
    return {
      translation: clean.split("\n")[0] || "",
      explanation: clean,
      usage: "",
      raw: stdout,
    };
  }
}
