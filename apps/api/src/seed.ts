/**
 * Jeu de données de développement.
 *
 * Crée la bibliothèque de questions et une petite population de comptes
 * complets, suffisante pour que le moteur de tissage ait de quoi composer trois
 * fils. Idempotent : on peut le relancer sans dupliquer.
 */
import { MOTIF_TAGS } from "@weave/contracts";
import { emailHash } from "./lib/crypto.ts";
import { log } from "./lib/log.ts";
import { prisma } from "./lib/prisma.ts";

const PROMPTS: { text: string; theme: string }[] = [
  // Écrites pour des jeunes adultes : ce qu'on vit entre 18 et 30 ans — les
  // études, le premier travail, le premier appartement, les amitiés qui se
  // déplacent. Elles demandent une vraie réponse, jamais un mot-clé.
  { text: "Qu'est-ce qui vous a fait changer d'avis récemment ?", theme: "esprit" },
  { text: "Décrivez un dimanche réussi, heure par heure.", theme: "rythme" },
  { text: "Ce que vous faites quand un plan tombe à l'eau à 21 h.", theme: "rythme" },
  { text: "La dernière fois que vous vous êtes senti·e vraiment à votre place.", theme: "lien" },
  { text: "Un morceau que vous mettez toujours en premier dans une voiture.", theme: "goûts" },
  { text: "Ce que vous avez appris à faire cette année, même mal.", theme: "faire" },
  { text: "Une amitié qui a tenu malgré la distance, et pourquoi.", theme: "lien" },
  { text: "Ce que vous feriez d'un samedi entier sans téléphone.", theme: "rythme" },
  { text: "Le conseil qu'on vous répète et que vous n'écoutez pas.", theme: "esprit" },
  { text: "Ce qui vous occupe quand personne ne regarde.", theme: "rythme" },
  { text: "Un endroit de votre ville que vous montreriez en premier.", theme: "lieux" },
  { text: "Ce que vous défendez même quand ça vous coûte.", theme: "valeurs" },
  { text: "Ce que vous cherchez ici, dit sans détour.", theme: "intentions" },
  { text: "Une chose que vous avez ratée et refaite.", theme: "faire" },
  { text: "Ce qui vous fait rire alors que ça ne devrait pas.", theme: "goûts" },
  {
    text: "Où vous imaginez vivre dans cinq ans — ou pourquoi vous n'en savez rien.",
    theme: "intentions",
  },
];

const CITIES: Record<string, { lat: number; lon: number }> = {
  Paris: { lat: 48.86, lon: 2.35 },
  Lyon: { lat: 45.76, lon: 4.84 },
  Marseille: { lat: 43.3, lon: 5.37 },
  Bordeaux: { lat: 44.84, lon: -0.58 },
  Nantes: { lat: 47.22, lon: -1.55 },
  Lille: { lat: 50.63, lon: 3.06 },
};

const MOTIF_VOCABULARY = [
  "escalade",
  "vinyles",
  "ciné-club",
  "cuisine",
  "colocation",
  "vélo",
  "impro",
  "concerts",
  "jeux de société",
  "rando",
  "photo",
  "théâtre",
  "natation",
  "skate",
  "podcasts",
  "sérigraphie",
  "course",
  "céramique",
  "bénévolat",
  "jazz",
  "karaoké",
  "brocantes",
  "bouldering",
  "cuisine coréenne",
];

const PEOPLE: {
  name: string;
  gender: string;
  city: keyof typeof CITIES;
  year: number;
  intent: string;
}[] = [
  // Paris — assez nombreux pour garnir un métier entier.
  { name: "Camille", gender: "femme", city: "Paris", year: 1996, intent: "relation" },
  { name: "Inès", gender: "femme", city: "Paris", year: 1997, intent: "ouverte" },
  { name: "Sofia", gender: "femme", city: "Paris", year: 1998, intent: "amitié_dabord" },
  { name: "Louise", gender: "femme", city: "Paris", year: 1999, intent: "relation" },
  { name: "Anouk", gender: "femme", city: "Paris", year: 2000, intent: "ouverte" },
  { name: "Salomé", gender: "femme", city: "Paris", year: 2001, intent: "relation" },
  { name: "Nour", gender: "femme", city: "Paris", year: 2002, intent: "ouverte" },
  { name: "Agathe", gender: "femme", city: "Paris", year: 2003, intent: "relation" },
  { name: "Elsa", gender: "femme", city: "Paris", year: 2004, intent: "amitié_dabord" },
  { name: "Margaux", gender: "femme", city: "Paris", year: 2005, intent: "relation" },
  { name: "Jonas", gender: "homme", city: "Paris", year: 2006, intent: "relation" },
  { name: "Théo", gender: "homme", city: "Paris", year: 1996, intent: "relation" },
  { name: "Aurélien", gender: "homme", city: "Paris", year: 1997, intent: "ouverte" },
  { name: "Noé", gender: "homme", city: "Paris", year: 1998, intent: "relation" },
  { name: "Hugo", gender: "homme", city: "Paris", year: 1999, intent: "ouverte" },
  { name: "Ismaël", gender: "homme", city: "Paris", year: 2000, intent: "relation" },
  { name: "Victor", gender: "homme", city: "Paris", year: 2001, intent: "amitié_dabord" },
  { name: "Antoine", gender: "homme", city: "Paris", year: 2002, intent: "relation" },
  { name: "Gaspard", gender: "homme", city: "Paris", year: 2003, intent: "ouverte" },
  { name: "Simon", gender: "homme", city: "Paris", year: 2004, intent: "relation" },
  { name: "Alex", gender: "non_binaire", city: "Paris", year: 2005, intent: "ouverte" },
  { name: "Charlie", gender: "non_binaire", city: "Paris", year: 2006, intent: "relation" },
  { name: "Camille B", gender: "non_binaire", city: "Paris", year: 1996, intent: "ouverte" },

  // Lyon
  { name: "Léa", gender: "femme", city: "Lyon", year: 1997, intent: "relation" },
  { name: "Manon", gender: "femme", city: "Lyon", year: 1998, intent: "ouverte" },
  { name: "Clara", gender: "femme", city: "Lyon", year: 1999, intent: "relation" },
  { name: "Ravi", gender: "homme", city: "Lyon", year: 2000, intent: "ouverte" },
  { name: "Paul", gender: "homme", city: "Lyon", year: 2001, intent: "relation" },
  { name: "Youssef", gender: "homme", city: "Lyon", year: 2002, intent: "ouverte" },
  { name: "Sacha", gender: "non_binaire", city: "Lyon", year: 2003, intent: "amitié_dabord" },

  // Marseille
  { name: "Malik", gender: "homme", city: "Marseille", year: 2004, intent: "ouverte" },
  { name: "Lucas", gender: "homme", city: "Marseille", year: 2005, intent: "relation" },
  { name: "Nina", gender: "femme", city: "Marseille", year: 2006, intent: "relation" },
  { name: "Jade", gender: "femme", city: "Marseille", year: 1996, intent: "ouverte" },

  // Bordeaux
  { name: "Mathilde", gender: "femme", city: "Bordeaux", year: 1997, intent: "relation" },
  { name: "Romain", gender: "homme", city: "Bordeaux", year: 1998, intent: "ouverte" },
  { name: "Chloé", gender: "femme", city: "Bordeaux", year: 1999, intent: "relation" },

  // Nantes
  { name: "Basile", gender: "homme", city: "Nantes", year: 2000, intent: "relation" },
  { name: "Maëlle", gender: "femme", city: "Nantes", year: 2001, intent: "ouverte" },

  // Lille
  { name: "Adrien", gender: "homme", city: "Lille", year: 2002, intent: "relation" },
  { name: "Zoé", gender: "femme", city: "Lille", year: 2003, intent: "ouverte" },
];

function slug(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function pick<T>(source: readonly T[], count: number, offset: number): T[] {
  const out: T[] = [];
  for (let i = 0; i < count; i++) out.push(source[(offset * 7 + i * 3) % source.length]!);
  return [...new Set(out)];
}

async function seedPrompts(): Promise<string[]> {
  const ids: string[] = [];
  for (const prompt of PROMPTS) {
    const row = await prisma.prompt.upsert({
      where: { text: prompt.text },
      create: { text: prompt.text, theme: prompt.theme, locale: "fr-FR", active: true },
      update: { theme: prompt.theme, active: true },
      select: { id: true },
    });
    ids.push(row.id);
  }
  return ids;
}

async function seedPeople(promptIds: string[]): Promise<void> {
  for (const [index, person] of PEOPLE.entries()) {
    const email = `${slug(person.name)}@weave.test`;
    const hash = emailHash(email);
    const coords = CITIES[person.city]!;

    const account = await prisma.account.upsert({
      where: { emailHash: hash },
      create: {
        email,
        emailHash: hash,
        handle: slug(person.name),
        displayName: person.name,
        birthDate: new Date(Date.UTC(person.year, index % 12, 1 + (index % 27))),
        status: "active",
        verified: index % 3 === 0,
        weavingHour: [8, 12, 18, 21][index % 4]!,
        lastSeenAt: new Date(Date.now() - index * 60 * 60 * 1000),
        preference: {
          create: {
            minAge: 18,
            maxAge: 32,
            maxDistanceKm: 60,
            seekingJson: JSON.stringify(["femme", "homme", "non_binaire"]),
            intentsJson: JSON.stringify(["relation", "ouverte", "amitié_dabord"]),
          },
        },
        subscription: { create: { tier: index === 0 ? "chaine" : "fil" } },
      },
      update: { status: "active" },
      select: { id: true },
    });

    const profile = await prisma.profile.upsert({
      where: { accountId: account.id },
      create: {
        accountId: account.id,
        city: person.city,
        // Légère dispersion pour que les distances ne soient pas toutes nulles.
        latRounded: Math.round((coords.lat + (index % 5) * 0.02) * 100) / 100,
        lonRounded: Math.round((coords.lon + (index % 4) * 0.02) * 100) / 100,
        gender: person.gender,
        intent: person.intent,
        bio: "",
        photoKey: `demo/${slug(person.name)}.jpg`,
        photoReviewedAt: new Date(),
        completeness: 100,
      },
      update: { photoKey: `demo/${slug(person.name)}.jpg`, completeness: 100 },
      select: { id: true },
    });

    const tags = pick(MOTIF_VOCABULARY, MOTIF_TAGS, index);
    await prisma.motifTag.deleteMany({ where: { profileId: profile.id } });
    await prisma.motifTag.createMany({
      data: tags.map((tag, position) => ({
        profileId: profile.id,
        tag,
        weight: 100 - position * 10,
      })),
    });

    const chosen = pick(promptIds, 3, index);
    for (const [position, promptId] of chosen.entries()) {
      await prisma.profileFragment.upsert({
        where: { profileId_promptId: { profileId: profile.id, promptId } },
        create: {
          profileId: profile.id,
          promptId,
          kind: "question",
          body: `Réponse de ${person.name} — ${tags[position % tags.length]}, surtout le matin.`,
          position,
        },
        update: { position },
      });
    }
  }
}

const promptIds = await seedPrompts();
await seedPeople(promptIds);

const counts = {
  questions: await prisma.prompt.count(),
  comptes: await prisma.account.count(),
  profils: await prisma.profile.count(),
  fragments: await prisma.profileFragment.count(),
};

log.info("Jeu de données prêt", counts);
await prisma.$disconnect();
