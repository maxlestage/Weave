-- CreateSchema
CREATE SCHEMA IF NOT EXISTS "public";

-- CreateTable
CREATE TABLE "accounts" (
    "id" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "emailHash" TEXT NOT NULL,
    "handle" TEXT NOT NULL,
    "displayName" TEXT NOT NULL,
    "birthDate" TIMESTAMP(3) NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'onboarding',
    "weavingHour" INTEGER NOT NULL DEFAULT 18,
    "timezone" TEXT NOT NULL DEFAULT 'Europe/Paris',
    "locale" TEXT NOT NULL DEFAULT 'fr-FR',
    "verified" BOOLEAN NOT NULL DEFAULT false,
    "lastSeenAt" TIMESTAMP(3),
    "deletionRequestedAt" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "accounts_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "otp_challenges" (
    "id" TEXT NOT NULL,
    "emailHash" TEXT NOT NULL,
    "codeHash" TEXT NOT NULL,
    "attempts" INTEGER NOT NULL DEFAULT 0,
    "consumedAt" TIMESTAMP(3),
    "expiresAt" TIMESTAMP(3) NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "otp_challenges_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "refresh_tokens" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "tokenHash" TEXT NOT NULL,
    "deviceId" TEXT,
    "revokedAt" TIMESTAMP(3),
    "rotatedTo" TEXT,
    "expiresAt" TIMESTAMP(3) NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "refresh_tokens_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "profiles" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "city" TEXT NOT NULL,
    "latRounded" DOUBLE PRECISION NOT NULL,
    "lonRounded" DOUBLE PRECISION NOT NULL,
    "gender" TEXT NOT NULL,
    "intent" TEXT NOT NULL DEFAULT 'ouverte',
    "bio" TEXT NOT NULL DEFAULT '',
    "photoKey" TEXT,
    "photoReviewedAt" TIMESTAMP(3),
    "completeness" INTEGER NOT NULL DEFAULT 0,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "profiles_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "profile_fragments" (
    "id" TEXT NOT NULL,
    "profileId" TEXT NOT NULL,
    "promptId" TEXT NOT NULL,
    "kind" TEXT NOT NULL DEFAULT 'question',
    "body" TEXT NOT NULL,
    "audioKey" TEXT,
    "durationSeconds" INTEGER,
    "position" INTEGER NOT NULL DEFAULT 0,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "profile_fragments_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "motif_tags" (
    "id" TEXT NOT NULL,
    "profileId" TEXT NOT NULL,
    "tag" TEXT NOT NULL,
    "weight" INTEGER NOT NULL DEFAULT 50,

    CONSTRAINT "motif_tags_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "prompts" (
    "id" TEXT NOT NULL,
    "text" TEXT NOT NULL,
    "theme" TEXT NOT NULL,
    "locale" TEXT NOT NULL DEFAULT 'fr-FR',
    "active" BOOLEAN NOT NULL DEFAULT true,
    "activeFrom" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "prompts_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "preferences" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "minAge" INTEGER NOT NULL DEFAULT 18,
    "maxAge" INTEGER NOT NULL DEFAULT 99,
    "maxDistanceKm" INTEGER NOT NULL DEFAULT 50,
    "seekingJson" TEXT NOT NULL DEFAULT '[]',
    "intentsJson" TEXT NOT NULL DEFAULT '[]',
    "refinedJson" TEXT NOT NULL DEFAULT '{}',
    "escaleCity" TEXT,
    "escaleUntil" TIMESTAMP(3),
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "preferences_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "thread_ledger" (
    "id" TEXT NOT NULL,
    "viewerId" TEXT NOT NULL,
    "candidateId" TEXT NOT NULL,
    "cacheKey" TEXT NOT NULL,
    "outcome" TEXT NOT NULL DEFAULT 'propose',
    "score" INTEGER NOT NULL DEFAULT 0,
    "servedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "resolvedAt" TIMESTAMP(3),
    "expiresAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "thread_ledger_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "woven_threads" (
    "id" TEXT NOT NULL,
    "initiatorId" TEXT NOT NULL,
    "responderId" TEXT NOT NULL,
    "wovenAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "lastMessageAt" TIMESTAMP(3),
    "state" TEXT NOT NULL DEFAULT 'tisse',
    "exchanges" INTEGER NOT NULL DEFAULT 1,
    "revealPercent" INTEGER NOT NULL DEFAULT 33,
    "closedAt" TIMESTAMP(3),
    "closedBy" TEXT,

    CONSTRAINT "woven_threads_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "messages" (
    "id" TEXT NOT NULL,
    "threadId" TEXT NOT NULL,
    "authorId" TEXT NOT NULL,
    "body" TEXT NOT NULL,
    "audioKey" TEXT,
    "durationSeconds" INTEGER,
    "sentAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "readAt" TIMESTAMP(3),
    "purgeAfter" TIMESTAMP(3),

    CONSTRAINT "messages_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "devices" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "platform" TEXT NOT NULL,
    "vendorId" TEXT NOT NULL,
    "model" TEXT,
    "osVersion" TEXT,
    "appVersion" TEXT,
    "apnsToken" TEXT,
    "pushToStartToken" TEXT,
    "apnsEnvironment" TEXT NOT NULL DEFAULT 'sandbox',
    "lastSeenAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "devices_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "live_activity_sessions" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "deviceId" TEXT NOT NULL,
    "updateToken" TEXT NOT NULL,
    "startedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "lastStateJson" TEXT NOT NULL DEFAULT '{}',
    "lastPushAt" TIMESTAMP(3),
    "endedAt" TIMESTAMP(3),
    "staleAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "live_activity_sessions_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "subscriptions" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "tier" TEXT NOT NULL DEFAULT 'fil',
    "period" TEXT,
    "storeKitProductId" TEXT,
    "originalTransactionId" TEXT,
    "renewsAt" TIMESTAMP(3),
    "expiresAt" TIMESTAMP(3),
    "inGracePeriod" BOOLEAN NOT NULL DEFAULT false,
    "cancelledAt" TIMESTAMP(3),
    "environment" TEXT NOT NULL DEFAULT 'sandbox',
    "updatedAt" TIMESTAMP(3) NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "subscriptions_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "unit_purchases" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "sku" TEXT NOT NULL,
    "transactionId" TEXT NOT NULL,
    "quantity" INTEGER NOT NULL DEFAULT 1,
    "priceCents" INTEGER NOT NULL,
    "currency" TEXT NOT NULL DEFAULT 'EUR',
    "environment" TEXT NOT NULL DEFAULT 'sandbox',
    "consumedAt" TIMESTAMP(3),
    "refundedAt" TIMESTAMP(3),
    "purchasedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "unit_purchases_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "credit_balances" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "sku" TEXT NOT NULL,
    "balance" INTEGER NOT NULL DEFAULT 0,
    "resetsAt" TIMESTAMP(3),
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "credit_balances_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "reports" (
    "id" TEXT NOT NULL,
    "authorId" TEXT NOT NULL,
    "targetId" TEXT NOT NULL,
    "reason" TEXT NOT NULL,
    "details" TEXT NOT NULL DEFAULT '',
    "state" TEXT NOT NULL DEFAULT 'ouvert',
    "handledAt" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "reports_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "blocks" (
    "id" TEXT NOT NULL,
    "authorId" TEXT NOT NULL,
    "targetId" TEXT NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "blocks_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "consent_records" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "kind" TEXT NOT NULL,
    "version" TEXT NOT NULL,
    "granted" BOOLEAN NOT NULL,
    "grantedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "revokedAt" TIMESTAMP(3),

    CONSTRAINT "consent_records_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "audit_events" (
    "id" TEXT NOT NULL,
    "accountId" TEXT,
    "action" TEXT NOT NULL,
    "subject" TEXT,
    "metaJson" TEXT NOT NULL DEFAULT '{}',
    "ip" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "audit_events_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "accounts_email_key" ON "accounts"("email");

-- CreateIndex
CREATE UNIQUE INDEX "accounts_emailHash_key" ON "accounts"("emailHash");

-- CreateIndex
CREATE UNIQUE INDEX "accounts_handle_key" ON "accounts"("handle");

-- CreateIndex
CREATE INDEX "accounts_status_weavingHour_idx" ON "accounts"("status", "weavingHour");

-- CreateIndex
CREATE INDEX "accounts_deletionRequestedAt_idx" ON "accounts"("deletionRequestedAt");

-- CreateIndex
CREATE INDEX "otp_challenges_emailHash_expiresAt_idx" ON "otp_challenges"("emailHash", "expiresAt");

-- CreateIndex
CREATE UNIQUE INDEX "refresh_tokens_tokenHash_key" ON "refresh_tokens"("tokenHash");

-- CreateIndex
CREATE INDEX "refresh_tokens_accountId_expiresAt_idx" ON "refresh_tokens"("accountId", "expiresAt");

-- CreateIndex
CREATE UNIQUE INDEX "profiles_accountId_key" ON "profiles"("accountId");

-- CreateIndex
CREATE INDEX "profiles_city_idx" ON "profiles"("city");

-- CreateIndex
CREATE INDEX "profiles_latRounded_lonRounded_idx" ON "profiles"("latRounded", "lonRounded");

-- CreateIndex
CREATE INDEX "profile_fragments_profileId_position_idx" ON "profile_fragments"("profileId", "position");

-- CreateIndex
CREATE UNIQUE INDEX "profile_fragments_profileId_promptId_key" ON "profile_fragments"("profileId", "promptId");

-- CreateIndex
CREATE INDEX "motif_tags_tag_idx" ON "motif_tags"("tag");

-- CreateIndex
CREATE UNIQUE INDEX "motif_tags_profileId_tag_key" ON "motif_tags"("profileId", "tag");

-- CreateIndex
CREATE UNIQUE INDEX "prompts_text_key" ON "prompts"("text");

-- CreateIndex
CREATE INDEX "prompts_locale_active_idx" ON "prompts"("locale", "active");

-- CreateIndex
CREATE UNIQUE INDEX "preferences_accountId_key" ON "preferences"("accountId");

-- CreateIndex
CREATE UNIQUE INDEX "thread_ledger_cacheKey_key" ON "thread_ledger"("cacheKey");

-- CreateIndex
CREATE INDEX "thread_ledger_viewerId_outcome_idx" ON "thread_ledger"("viewerId", "outcome");

-- CreateIndex
CREATE INDEX "thread_ledger_expiresAt_idx" ON "thread_ledger"("expiresAt");

-- CreateIndex
CREATE UNIQUE INDEX "thread_ledger_viewerId_candidateId_key" ON "thread_ledger"("viewerId", "candidateId");

-- CreateIndex
CREATE INDEX "woven_threads_initiatorId_lastMessageAt_idx" ON "woven_threads"("initiatorId", "lastMessageAt");

-- CreateIndex
CREATE INDEX "woven_threads_responderId_lastMessageAt_idx" ON "woven_threads"("responderId", "lastMessageAt");

-- CreateIndex
CREATE UNIQUE INDEX "woven_threads_initiatorId_responderId_key" ON "woven_threads"("initiatorId", "responderId");

-- CreateIndex
CREATE INDEX "messages_threadId_sentAt_idx" ON "messages"("threadId", "sentAt");

-- CreateIndex
CREATE INDEX "messages_purgeAfter_idx" ON "messages"("purgeAfter");

-- CreateIndex
CREATE INDEX "devices_accountId_platform_idx" ON "devices"("accountId", "platform");

-- CreateIndex
CREATE UNIQUE INDEX "devices_accountId_vendorId_key" ON "devices"("accountId", "vendorId");

-- CreateIndex
CREATE UNIQUE INDEX "live_activity_sessions_updateToken_key" ON "live_activity_sessions"("updateToken");

-- CreateIndex
CREATE INDEX "live_activity_sessions_accountId_endedAt_idx" ON "live_activity_sessions"("accountId", "endedAt");

-- CreateIndex
CREATE INDEX "live_activity_sessions_staleAt_idx" ON "live_activity_sessions"("staleAt");

-- CreateIndex
CREATE UNIQUE INDEX "subscriptions_accountId_key" ON "subscriptions"("accountId");

-- CreateIndex
CREATE UNIQUE INDEX "subscriptions_originalTransactionId_key" ON "subscriptions"("originalTransactionId");

-- CreateIndex
CREATE INDEX "subscriptions_tier_expiresAt_idx" ON "subscriptions"("tier", "expiresAt");

-- CreateIndex
CREATE UNIQUE INDEX "unit_purchases_transactionId_key" ON "unit_purchases"("transactionId");

-- CreateIndex
CREATE INDEX "unit_purchases_accountId_sku_idx" ON "unit_purchases"("accountId", "sku");

-- CreateIndex
CREATE UNIQUE INDEX "credit_balances_accountId_sku_key" ON "credit_balances"("accountId", "sku");

-- CreateIndex
CREATE INDEX "reports_state_createdAt_idx" ON "reports"("state", "createdAt");

-- CreateIndex
CREATE INDEX "blocks_targetId_idx" ON "blocks"("targetId");

-- CreateIndex
CREATE UNIQUE INDEX "blocks_authorId_targetId_key" ON "blocks"("authorId", "targetId");

-- CreateIndex
CREATE INDEX "consent_records_accountId_kind_idx" ON "consent_records"("accountId", "kind");

-- CreateIndex
CREATE INDEX "audit_events_action_createdAt_idx" ON "audit_events"("action", "createdAt");

-- AddForeignKey
ALTER TABLE "refresh_tokens" ADD CONSTRAINT "refresh_tokens_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "profiles" ADD CONSTRAINT "profiles_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "profile_fragments" ADD CONSTRAINT "profile_fragments_profileId_fkey" FOREIGN KEY ("profileId") REFERENCES "profiles"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "profile_fragments" ADD CONSTRAINT "profile_fragments_promptId_fkey" FOREIGN KEY ("promptId") REFERENCES "prompts"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "motif_tags" ADD CONSTRAINT "motif_tags_profileId_fkey" FOREIGN KEY ("profileId") REFERENCES "profiles"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "preferences" ADD CONSTRAINT "preferences_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "thread_ledger" ADD CONSTRAINT "thread_ledger_viewerId_fkey" FOREIGN KEY ("viewerId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "thread_ledger" ADD CONSTRAINT "thread_ledger_candidateId_fkey" FOREIGN KEY ("candidateId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "woven_threads" ADD CONSTRAINT "woven_threads_initiatorId_fkey" FOREIGN KEY ("initiatorId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "woven_threads" ADD CONSTRAINT "woven_threads_responderId_fkey" FOREIGN KEY ("responderId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "messages" ADD CONSTRAINT "messages_threadId_fkey" FOREIGN KEY ("threadId") REFERENCES "woven_threads"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "messages" ADD CONSTRAINT "messages_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "devices" ADD CONSTRAINT "devices_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "live_activity_sessions" ADD CONSTRAINT "live_activity_sessions_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "live_activity_sessions" ADD CONSTRAINT "live_activity_sessions_deviceId_fkey" FOREIGN KEY ("deviceId") REFERENCES "devices"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "subscriptions" ADD CONSTRAINT "subscriptions_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "unit_purchases" ADD CONSTRAINT "unit_purchases_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "credit_balances" ADD CONSTRAINT "credit_balances_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "reports" ADD CONSTRAINT "reports_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "reports" ADD CONSTRAINT "reports_targetId_fkey" FOREIGN KEY ("targetId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "blocks" ADD CONSTRAINT "blocks_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "blocks" ADD CONSTRAINT "blocks_targetId_fkey" FOREIGN KEY ("targetId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "consent_records" ADD CONSTRAINT "consent_records_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "audit_events" ADD CONSTRAINT "audit_events_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE SET NULL ON UPDATE CASCADE;

