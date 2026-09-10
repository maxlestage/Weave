/**
 * Service des médias, sous URL signée.
 *
 * Une photo n'est jamais accessible par une URL devinable ni servie nette avant
 * que la révélation progressive l'autorise : le niveau de flou est inscrit dans
 * la signature, donc impossible à modifier côté client.
 */
import { Elysia, t } from "elysia";
import { env } from "../env.ts";
import { verifyMediaSignature } from "../lib/crypto.ts";
import { forbidden } from "../lib/errors.ts";

export const mediaRoutes = new Elysia({ prefix: "/media", tags: ["Médias"] }).get(
  "/:key",
  ({ params, query, set }) => {
    const valid = verifyMediaSignature(
      env.media.signingSecret,
      decodeURIComponent(params.key),
      Number(query.exp),
      Number(query.blur ?? 0),
      query.sig,
    );
    if (!valid) throw forbidden("Lien média expiré ou invalide.");

    // En production, la requête est relayée vers le stockage objet, qui applique
    // la transformation de flou correspondant au paramètre signé.
    set.headers["cache-control"] = "private, max-age=60";
    return {
      key: decodeURIComponent(params.key),
      blur: Number(query.blur ?? 0),
      note: "Le relais vers le stockage objet est branché au déploiement.",
    };
  },
  {
    params: t.Object({ key: t.String() }),
    query: t.Object({
      exp: t.String(),
      sig: t.String(),
      blur: t.Optional(t.String()),
    }),
    detail: {
      summary: "Servir un média signé",
      description: "Le niveau de flou fait partie de la signature : il ne peut pas être contourné.",
    },
  },
);
