-- Le dernier bilan mensuel accordé par l'abonnement.
--
-- Deux paliers annoncent un « Bilan mensuel » parmi ce qu'ils incluent, et la
-- route l'exigeait pourtant en crédit. Quelqu'un payant 14,99 € par mois se
-- voyait demander 2,99 € de plus pour ce que son abonnement promet.
--
-- Cette date dit quand la part incluse a été servie. Nulle tant qu'aucune ne
-- l'a été.
ALTER TABLE "accounts" ADD COLUMN "lastBilanAt" TIMESTAMP(3);
