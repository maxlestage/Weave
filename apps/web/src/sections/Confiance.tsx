import { ACCOUNT_PURGE_DAYS, MESSAGE_RETENTION_DAYS, MIN_AGE } from "@weave/contracts";
import { Carte, filVar, Section, type Fil } from "../composants.tsx";
import { useTraduction } from "../contexte-langue.tsx";
import type { Traduit } from "../langues.ts";

type Engagement = { readonly titre: string; readonly texte: string };

const ENGAGEMENTS: Traduit<readonly Engagement[]> = {
  fr: [
    {
      titre: "Rien à vendre à personne",
      texte:
        "Pas de publicité, pas de courtier en données, pas de revente. Le produit est financé par celles et ceux qui s'abonnent ou achètent à l'unité.",
    },
    {
      titre: "Localisation au kilomètre",
      texte:
        "Votre position est arrondie avant d'être enregistrée, et un plan n'affiche jamais d'adresse dans le fil : une ville, une distance arrondie. L'endroit exact se dit dans la conversation, à qui vous avez accepté.",
    },
    {
      titre: "Ce que vous consultez n'est pas archivé",
      texte:
        "Le fil est recomposé à la demande et vit quelques minutes dans un cache. Nous ne constituons pas d'historique des plans que vous avez regardés, et personne ne peut le consulter.",
    },
    {
      titre: "Blocage immédiat",
      texte:
        "Bloquer ou signaler coupe tout des deux côtés — demandes en attente closes, conversation fermée — sans notification à l'autre personne. Un signalement entraîne toujours un blocage.",
    },
  ],
  en: [
    {
      titre: "Nothing to sell to anyone",
      texte:
        "No advertising, no data brokers, no resale. The product is paid for by the people who subscribe or buy things singly.",
    },
    {
      titre: "Location to the kilometre",
      texte:
        "Your position is rounded before it is stored, and a plan never shows an address in the feed: a town, a rounded distance. The exact place is said in the conversation, to whoever you accepted.",
    },
    {
      titre: "What you look at is not kept",
      texte:
        "The feed is rebuilt on demand and lives a few minutes in a cache. We build no history of the plans you looked at, and nobody can consult one.",
    },
    {
      titre: "Blocking takes effect at once",
      texte:
        "Blocking or reporting cuts everything off both ways — pending requests closed, conversation shut — with no notification to the other person. A report always carries a block with it.",
    },
  ],
  es: [
    {
      titre: "Nada que vender a nadie",
      texte:
        "Sin publicidad, sin intermediarios de datos, sin reventa. El producto lo financian quienes se suscriben o compran por unidades.",
    },
    {
      titre: "Ubicación al kilómetro",
      texte:
        "Tu posición se redondea antes de guardarse, y un plan nunca muestra una dirección en el muro: una ciudad, una distancia redondeada. El sitio exacto se dice en la conversación, a quien hayas aceptado.",
    },
    {
      titre: "Lo que consultas no se archiva",
      texte:
        "El muro se recompone bajo demanda y vive unos minutos en una caché. No creamos ningún historial de los planes que has mirado, y nadie puede consultarlo.",
    },
    {
      titre: "Bloqueo inmediato",
      texte:
        "Bloquear o denunciar corta todo por ambos lados — peticiones pendientes cerradas, conversación cerrada — sin notificar a la otra persona. Una denuncia conlleva siempre un bloqueo.",
    },
  ],
};

const TEXTES: Traduit<{
  titre: string;
  chapeau: string;
  age: string;
  ans: (n: number) => string;
  messages: string;
  jours: (n: number) => string;
  apresCloture: string;
  compte: string;
  avantPurge: string;
}> = {
  fr: {
    titre: "Ce à quoi nous nous engageons",
    chapeau:
      "Une application où l'on donne son heure et son lieu manipule ce qu'il y a de plus sensible. Voici ce que nous nous interdisons.",
    age: "Âge minimum",
    ans: (n) => `${n} ans`,
    messages: "Messages conservés",
    jours: (n) => `${n} jours`,
    apresCloture: "après clôture d'une conversation",
    compte: "Compte supprimé",
    avantPurge: "avant purge définitive",
  },
  en: {
    titre: "What we commit to",
    chapeau:
      "An app where you give away your time and your place handles the most sensitive thing there is. Here is what we rule out for ourselves.",
    age: "Minimum age",
    ans: (n) => `${n} and over`,
    messages: "Messages kept",
    jours: (n) => `${n} days`,
    apresCloture: "after a conversation is closed",
    compte: "Deleted account",
    avantPurge: "before permanent purge",
  },
  es: {
    titre: "A qué nos comprometemos",
    chapeau:
      "Una aplicación en la que das tu hora y tu sitio maneja lo más sensible que hay. Esto es lo que nos prohibimos.",
    age: "Edad mínima",
    ans: (n) => `${n} años`,
    messages: "Mensajes conservados",
    jours: (n) => `${n} días`,
    apresCloture: "tras cerrar una conversación",
    compte: "Cuenta eliminada",
    avantPurge: "antes del borrado definitivo",
  },
};

export function Confiance() {
  const t = useTraduction(TEXTES);
  const engagements = useTraduction(ENGAGEMENTS);

  return (
    <Section id="confiance" fil={1} titre={t.titre} chapeau={t.chapeau} alterne>
      <div className="grid gap-4 sm:grid-cols-2">
        {engagements.map((engagement, index) => (
          <Carte key={engagement.titre} fil={((index % 6) + 1) as Fil}>
            <h3 className="text-lg font-bold" style={{ color: filVar(((index % 6) + 1) as Fil) }}>
              {engagement.titre}
            </h3>
            <p className="mt-2.5 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
              {engagement.texte}
            </p>
          </Carte>
        ))}
      </div>

      <dl className="mt-10 grid gap-6 sm:grid-cols-3">
        <div>
          <dt className="text-sm" style={{ color: "var(--texte-doux)" }}>
            {t.age}
          </dt>
          <dd className="mt-1 text-2xl font-semibold tabular-nums">{t.ans(MIN_AGE)}</dd>
        </div>
        <div>
          <dt className="text-sm" style={{ color: "var(--texte-doux)" }}>
            {t.messages}
          </dt>
          <dd className="mt-1 text-2xl font-semibold tabular-nums">
            {t.jours(MESSAGE_RETENTION_DAYS)}
          </dd>
          <p className="mt-1 text-sm" style={{ color: "var(--texte-doux)" }}>
            {t.apresCloture}
          </p>
        </div>
        <div>
          <dt className="text-sm" style={{ color: "var(--texte-doux)" }}>
            {t.compte}
          </dt>
          <dd className="mt-1 text-2xl font-semibold tabular-nums">
            {t.jours(ACCOUNT_PURGE_DAYS)}
          </dd>
          <p className="mt-1 text-sm" style={{ color: "var(--texte-doux)" }}>
            {t.avantPurge}
          </p>
        </div>
      </dl>
    </Section>
  );
}
