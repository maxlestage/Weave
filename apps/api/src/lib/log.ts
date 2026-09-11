/** Journalisation structurée, une ligne JSON par événement. */
import { env } from "../env.ts";

type Level = "debug" | "info" | "warn" | "error";

const ORDER: Record<Level, number> = { debug: 10, info: 20, warn: 30, error: 40 };
const threshold = ORDER[(process.env.LOG_LEVEL as Level) ?? (env.isProduction ? "info" : "debug")];

function emit(level: Level, message: string, fields?: Record<string, unknown>): void {
  if (ORDER[level] < threshold) return;
  const line = JSON.stringify({
    ts: new Date().toISOString(),
    level,
    msg: message,
    ...fields,
  });
  if (level === "error" || level === "warn") console.error(line);
  else console.log(line);
}

export const log = {
  debug: (message: string, fields?: Record<string, unknown>) => emit("debug", message, fields),
  info: (message: string, fields?: Record<string, unknown>) => emit("info", message, fields),
  warn: (message: string, fields?: Record<string, unknown>) => emit("warn", message, fields),
  error: (message: string, fields?: Record<string, unknown>) => emit("error", message, fields),
};
