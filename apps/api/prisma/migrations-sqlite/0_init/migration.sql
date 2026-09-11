-- CreateTable
CREATE TABLE "accounts" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "email" TEXT NOT NULL,
    "emailHash" TEXT NOT NULL,
    "handle" TEXT NOT NULL,
    "displayName" TEXT NOT NULL,
    "birthDate" DATETIME NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'onboarding',
    "timezone" TEXT NOT NULL DEFAULT 'Europe/Paris',
    "locale" TEXT NOT NULL DEFAULT 'fr-FR',
    "verified" BOOLEAN NOT NULL DEFAULT false,
    "lastSeenAt" DATETIME,
    "deletionRequestedAt" DATETIME,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL
);

-- CreateTable
CREATE TABLE "otp_challenges" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "emailHash" TEXT NOT NULL,
    "codeHash" TEXT NOT NULL,
    "attempts" INTEGER NOT NULL DEFAULT 0,
    "consumedAt" DATETIME,
    "expiresAt" DATETIME NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- CreateTable
CREATE TABLE "refresh_tokens" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "tokenHash" TEXT NOT NULL,
    "deviceId" TEXT,
    "revokedAt" DATETIME,
    "rotatedTo" TEXT,
    "expiresAt" DATETIME NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "refresh_tokens_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "profiles" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "city" TEXT NOT NULL,
    "latRounded" REAL NOT NULL,
    "lonRounded" REAL NOT NULL,
    "gender" TEXT NOT NULL,
    "bio" TEXT NOT NULL DEFAULT '',
    "photoKey" TEXT,
    "photoReviewedAt" DATETIME,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "profiles_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "preferences" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "minAge" INTEGER NOT NULL DEFAULT 18,
    "maxAge" INTEGER NOT NULL DEFAULT 32,
    "maxDistanceKm" INTEGER NOT NULL DEFAULT 25,
    "seekingJson" TEXT NOT NULL DEFAULT '[]',
    "categoriesJson" TEXT NOT NULL DEFAULT '[]',
    "escaleCity" TEXT,
    "escaleUntil" DATETIME,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "preferences_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "plans" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "authorId" TEXT NOT NULL,
    "title" TEXT NOT NULL,
    "note" TEXT NOT NULL DEFAULT '',
    "category" TEXT NOT NULL,
    "startsAt" DATETIME NOT NULL,
    "city" TEXT NOT NULL,
    "latRounded" REAL NOT NULL,
    "lonRounded" REAL NOT NULL,
    "capacity" INTEGER NOT NULL DEFAULT 1,
    "state" TEXT NOT NULL DEFAULT 'ouvert',
    "cancelledAt" DATETIME,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "plans_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "join_requests" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "planId" TEXT NOT NULL,
    "authorId" TEXT NOT NULL,
    "message" TEXT NOT NULL,
    "state" TEXT NOT NULL DEFAULT 'envoyee',
    "sentAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "decidedAt" DATETIME,
    CONSTRAINT "join_requests_planId_fkey" FOREIGN KEY ("planId") REFERENCES "plans" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "join_requests_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "conversations" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "planId" TEXT NOT NULL,
    "requestId" TEXT NOT NULL,
    "hostId" TEXT NOT NULL,
    "guestId" TEXT NOT NULL,
    "openedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "lastMessageAt" DATETIME,
    "closedAt" DATETIME,
    "closedBy" TEXT,
    CONSTRAINT "conversations_planId_fkey" FOREIGN KEY ("planId") REFERENCES "plans" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "conversations_requestId_fkey" FOREIGN KEY ("requestId") REFERENCES "join_requests" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "conversations_hostId_fkey" FOREIGN KEY ("hostId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "conversations_guestId_fkey" FOREIGN KEY ("guestId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "messages" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "conversationId" TEXT NOT NULL,
    "authorId" TEXT NOT NULL,
    "body" TEXT NOT NULL,
    "sentAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "readAt" DATETIME,
    "purgeAfter" DATETIME,
    CONSTRAINT "messages_conversationId_fkey" FOREIGN KEY ("conversationId") REFERENCES "conversations" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "messages_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "devices" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "platform" TEXT NOT NULL,
    "vendorId" TEXT NOT NULL,
    "model" TEXT,
    "osVersion" TEXT,
    "appVersion" TEXT,
    "apnsToken" TEXT,
    "pushToStartToken" TEXT,
    "apnsEnvironment" TEXT NOT NULL DEFAULT 'sandbox',
    "lastSeenAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "devices_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "live_activity_sessions" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "deviceId" TEXT NOT NULL,
    "updateToken" TEXT NOT NULL,
    "startedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "lastStateJson" TEXT NOT NULL DEFAULT '{}',
    "lastPushAt" DATETIME,
    "endedAt" DATETIME,
    "staleAt" DATETIME NOT NULL,
    CONSTRAINT "live_activity_sessions_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "live_activity_sessions_deviceId_fkey" FOREIGN KEY ("deviceId") REFERENCES "devices" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "subscriptions" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "tier" TEXT NOT NULL DEFAULT 'depart',
    "period" TEXT,
    "storeKitProductId" TEXT,
    "originalTransactionId" TEXT,
    "renewsAt" DATETIME,
    "expiresAt" DATETIME,
    "inGracePeriod" BOOLEAN NOT NULL DEFAULT false,
    "cancelledAt" DATETIME,
    "environment" TEXT NOT NULL DEFAULT 'sandbox',
    "updatedAt" DATETIME NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "subscriptions_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "unit_purchases" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "sku" TEXT NOT NULL,
    "transactionId" TEXT NOT NULL,
    "quantity" INTEGER NOT NULL DEFAULT 1,
    "priceCents" INTEGER NOT NULL,
    "currency" TEXT NOT NULL DEFAULT 'EUR',
    "environment" TEXT NOT NULL DEFAULT 'sandbox',
    "consumedAt" DATETIME,
    "refundedAt" DATETIME,
    "purchasedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "unit_purchases_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "credit_balances" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "sku" TEXT NOT NULL,
    "balance" INTEGER NOT NULL DEFAULT 0,
    "resetsAt" DATETIME,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "credit_balances_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "reports" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "authorId" TEXT NOT NULL,
    "targetId" TEXT NOT NULL,
    "reason" TEXT NOT NULL,
    "details" TEXT NOT NULL DEFAULT '',
    "state" TEXT NOT NULL DEFAULT 'ouvert',
    "handledAt" DATETIME,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "reports_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "reports_targetId_fkey" FOREIGN KEY ("targetId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "blocks" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "authorId" TEXT NOT NULL,
    "targetId" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "blocks_authorId_fkey" FOREIGN KEY ("authorId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT "blocks_targetId_fkey" FOREIGN KEY ("targetId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "consent_records" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "kind" TEXT NOT NULL,
    "version" TEXT NOT NULL,
    "granted" BOOLEAN NOT NULL,
    "grantedAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "revokedAt" DATETIME,
    CONSTRAINT "consent_records_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

-- CreateTable
CREATE TABLE "audit_events" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT,
    "action" TEXT NOT NULL,
    "subject" TEXT,
    "metaJson" TEXT NOT NULL DEFAULT '{}',
    "ip" TEXT,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "audit_events_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE SET NULL ON UPDATE CASCADE
);

-- CreateIndex
CREATE UNIQUE INDEX "accounts_email_key" ON "accounts"("email");

-- CreateIndex
CREATE UNIQUE INDEX "accounts_emailHash_key" ON "accounts"("emailHash");

-- CreateIndex
CREATE UNIQUE INDEX "accounts_handle_key" ON "accounts"("handle");

-- CreateIndex
CREATE INDEX "accounts_status_idx" ON "accounts"("status");

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
CREATE UNIQUE INDEX "preferences_accountId_key" ON "preferences"("accountId");

-- CreateIndex
CREATE INDEX "plans_state_startsAt_idx" ON "plans"("state", "startsAt");

-- CreateIndex
CREATE INDEX "plans_latRounded_lonRounded_idx" ON "plans"("latRounded", "lonRounded");

-- CreateIndex
CREATE INDEX "plans_authorId_startsAt_idx" ON "plans"("authorId", "startsAt");

-- CreateIndex
CREATE INDEX "join_requests_authorId_state_idx" ON "join_requests"("authorId", "state");

-- CreateIndex
CREATE INDEX "join_requests_planId_state_idx" ON "join_requests"("planId", "state");

-- CreateIndex
CREATE UNIQUE INDEX "join_requests_planId_authorId_key" ON "join_requests"("planId", "authorId");

-- CreateIndex
CREATE UNIQUE INDEX "conversations_requestId_key" ON "conversations"("requestId");

-- CreateIndex
CREATE INDEX "conversations_hostId_lastMessageAt_idx" ON "conversations"("hostId", "lastMessageAt");

-- CreateIndex
CREATE INDEX "conversations_guestId_lastMessageAt_idx" ON "conversations"("guestId", "lastMessageAt");

-- CreateIndex
CREATE INDEX "messages_conversationId_sentAt_idx" ON "messages"("conversationId", "sentAt");

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

