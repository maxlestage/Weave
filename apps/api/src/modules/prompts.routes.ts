/**
 * Bibliothèque de questions. Elles sont la matière première des fragments : un
 * profil Weave se lit à travers des réponses, pas à travers une galerie photo.
 */
import { Elysia, t } from "elysia";
import { keys } from "../lib/cache.ts";
import { prisma } from "../lib/prisma.ts";
import { getJson, setJson } from "../lib/redis.ts";
import { localDay } from "../lib/time.ts";
import { authPlugin } from "../plugins/auth.ts";

/** La question du jour est identique pour tout le monde : elle se partage. */
const PROMPT_OF_DAY_TTL_SECONDS = 24 * 60 * 60;

interface PublicPrompt {
  id: string;
  text: string;
  theme: string;
}

export const promptRoutes = new Elysia({ prefix: "/v1/prompts", tags: ["Questions"] })
  .use(authPlugin)

  .get(
    "/",
    async ({ query }) => {
      const locale = query.locale ?? "fr-FR";
      const rows = await prisma.prompt.findMany({
        where: { locale, active: true },
        orderBy: { theme: "asc" },
        select: { id: true, text: true, theme: true },
      });
      return rows satisfies PublicPrompt[];
    },
    {
      query: t.Object({ locale: t.Optional(t.String({ maxLength: 10 })) }),
      detail: { summary: "Lister les questions disponibles" },
    },
  )

  .get(
    "/today",
    async ({ query, account }) => {
      const locale = query.locale ?? account?.locale ?? "fr-FR";
      const timezone = account?.timezone ?? "Europe/Paris";
      const day = localDay(timezone);
      const key = keys.promptOfDay(locale, day);

      const cached = await getJson<PublicPrompt>(key);
      if (cached !== null) return cached;

      const candidates = await prisma.prompt.findMany({
        where: { locale, active: true },
        select: { id: true, text: true, theme: true },
      });
      if (candidates.length === 0) return null;

      // Tirage déterministe : la même journée donne la même question à tout le
      // monde, sans avoir à planifier quoi que ce soit.
      const seed = [...`${locale}:${day}`].reduce((acc, char) => (acc * 31 + char.charCodeAt(0)) % 2147483647, 7);
      const prompt = candidates[seed % candidates.length]!;

      await setJson(key, prompt, PROMPT_OF_DAY_TTL_SECONDS);
      return prompt;
    },
    {
      query: t.Object({ locale: t.Optional(t.String({ maxLength: 10 })) }),
      detail: {
        summary: "Question du jour",
        description: "Identique pour toutes les personnes d'une même langue, renouvelée chaque jour.",
      },
    },
  );
