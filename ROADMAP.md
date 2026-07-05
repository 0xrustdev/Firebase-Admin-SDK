# Roadmap

This is the single source of truth for what this crate supports today versus
the official [Firebase Admin Node.js SDK](https://github.com/firebase/firebase-admin-node),
and what's left to build. Every method the official SDK exposes is listed
below, marked ✅ **Done** or 🔲 **Planned**, with an effort estimate
(**Easy** / **Medium** / **Hard**) on everything outstanding.

Looking to contribute? Pick any 🔲 item, especially an **Easy** one, and open
a PR — see [CONTRIBUTING.md](CONTRIBUTING.md). If you want to propose
something not listed here, open a feature request first (see
[CONTRIBUTING.md](CONTRIBUTING.md#reporting-bugs--requesting-features)).

This file lists *what* to build, not *when* — no version numbers are
promised here. See [CHANGELOG.md](CHANGELOG.md) for what has actually shipped.

## Authentication

Source: `firebase-admin-node`'s `BaseAuth` class. This crate's implementation
lives in [`src/auth/`](src/auth/).

- [x] ✅ **Done** — `createCustomToken` — mint a Firebase custom token for a uid, signed locally with a service account key
- [x] ✅ **Done** — `verifyIdToken` — verify a Firebase ID token and return its decoded claims
- [x] ✅ **Done** — `getUser` — fetch a user by uid
- [x] ✅ **Done** — `getUserByEmail` — fetch a user by email address
- [ ] 🔲 **Planned** — `getUserByPhoneNumber` — fetch a user by E.164 phone number — **Easy** (mirrors the existing `getUserByEmail` lookup path)
- [ ] 🔲 **Planned** — `getUserByProviderUid` — fetch a user by federated provider id + uid — **Medium**
- [ ] 🔲 **Planned** — `getUsers` — batch-fetch up to 100 users by mixed identifiers (uid/email/phone/provider) — **Medium** (needs a `UserIdentifier` enum)
- [x] ✅ **Done** — `createUser` — create a new user
- [x] ✅ **Done** — `updateUser` — update an existing user's properties
- [x] ✅ **Done** — `setCustomUserClaims` — set or clear developer claims on a user
- [x] ✅ **Done** — `deleteUser` — delete a user by uid
- [ ] 🔲 **Planned** — `deleteUsers` — bulk-delete up to 1000 users, returns per-user success/failure — **Medium**
- [x] ✅ **Done** — `listUsers` — paginated listing of all users
- [ ] 🔲 **Planned** — `revokeRefreshTokens` — invalidate all of a user's existing sessions/refresh tokens — **Easy** (implemented as `updateUser` setting `validSince`)
- [ ] 🔲 **Planned** — `importUsers` — bulk-import up to 1000 users, including pre-hashed passwords — **Hard** (requires modeling Firebase's password hash configs)
- [x] ✅ **Done** — `createSessionCookie` — exchange a verified ID token for a long-lived session cookie
- [x] ✅ **Done** — `verifySessionCookie` — verify a session cookie and return its decoded claims
- [ ] 🔲 **Planned** — `generatePasswordResetLink` — generate a password-reset email action link — **Medium** (new `accounts:sendOobCode` endpoint)
- [ ] 🔲 **Planned** — `generateEmailVerificationLink` — generate an email-verification action link — **Easy** (once `sendOobCode` exists)
- [ ] 🔲 **Planned** — `generateVerifyAndChangeEmailLink` — generate a verify-and-change-email action link — **Easy** (once `sendOobCode` exists)
- [ ] 🔲 **Planned** — `generateSignInWithEmailLink` — generate a passwordless sign-in email action link — **Easy** (once `sendOobCode` exists)
- [ ] 🔲 **Planned** — `listProviderConfigs` — list configured SAML/OIDC identity providers — **Hard**, niche
- [ ] 🔲 **Planned** — `getProviderConfig` — fetch one SAML/OIDC provider config — **Hard**, niche
- [ ] 🔲 **Planned** — `createProviderConfig` — create a SAML/OIDC provider config — **Hard**, niche
- [ ] 🔲 **Planned** — `updateProviderConfig` — update a SAML/OIDC provider config — **Hard**, niche
- [ ] 🔲 **Planned** — `deleteProviderConfig` — delete a SAML/OIDC provider config — **Hard**, niche
- [ ] 🔲 **Planned** — `tenantManager` — get the `TenantManager` handle for the project — **Hard**
- [ ] 🔲 **Planned** — `TenantManager.createTenant` — create a new auth tenant — **Hard** (once `tenantManager` exists)
- [ ] 🔲 **Planned** — `TenantManager.getTenant` — fetch a tenant by id — **Medium** (once `tenantManager` exists)
- [ ] 🔲 **Planned** — `TenantManager.listTenants` — paginated listing of tenants — **Medium** (once `tenantManager` exists)
- [ ] 🔲 **Planned** — `TenantManager.updateTenant` — update a tenant's config — **Medium** (once `tenantManager` exists)
- [ ] 🔲 **Planned** — `TenantManager.deleteTenant` — delete a tenant — **Easy** (once `tenantManager` exists)
- [ ] 🔲 **Planned** — `TenantManager.authForTenant` — get a tenant-scoped `TenantAwareAuth` client — **Hard** (a distinct subclass overriding `verifyIdToken`/`createSessionCookie`/`verifySessionCookie` to scope them to the tenant, plus re-scoping every user-management operation above)

## Cloud Firestore

Source: `firebase-admin-node`'s `firestore/` module, which is mostly a thin
wrapper re-exporting `@google-cloud/firestore` for a Firebase app's
credentials, plus a few admin-only additions.

- [ ] 🔲 **Planned** — `getFirestore()` — obtain a Firestore client scoped to the app's credentials/project (default or named database) — **Hard**
- [ ] 🔲 **Planned** — Document/collection CRUD (`get`, `set`, `update`, `delete`, `add`) — **Hard**
- [ ] 🔲 **Planned** — Queries (`where`, `orderBy`, `limit`, collection group queries) — **Hard**
- [ ] 🔲 **Planned** — Transactions (`runTransaction`) — **Hard**
- [ ] 🔲 **Planned** — Batched writes (`WriteBatch`) — **Hard**
- [ ] 🔲 **Planned** — `BulkWriter` — high-throughput batched writes with automatic retry — **Hard**
- [ ] 🔲 **Planned** — `recursiveDelete` — delete a document/collection and all descendants — **Medium** (once base CRUD exists)
- [ ] 🔲 **Planned** — `setLogFunction` — hook Firestore SDK internal logging — **Easy** (once base client exists)
- [ ] 🔲 **Planned** — Bundles (`BundleBuilder`) — package query/document snapshots for client caching — **Medium**

## Realtime Database

Source: `firebase-admin-node`'s `database/` module (`Database` class,
extending `@firebase/database-compat`'s `FirebaseDatabase`).

- [ ] 🔲 **Planned** — `getDatabase()` / `ref` / `refFromURL` — obtain a Database client and data references — **Hard**
- [ ] 🔲 **Planned** — `goOnline` / `goOffline` — control the client's realtime connection — **Medium** (once base client exists)
- [ ] 🔲 **Planned** — `getRules` — fetch currently applied security rules as a string (with comments) — **Medium**
- [ ] 🔲 **Planned** — `getRulesJSON` — fetch currently applied security rules as parsed JSON — **Medium**
- [ ] 🔲 **Planned** — `setRules` — deploy new security rules from a string, buffer, or object — **Medium**

## Cloud Storage

Source: `firebase-admin-node`'s `storage/` module — a thin wrapper handing
out `@google-cloud/storage` bucket handles under the app's credentials.

- [ ] 🔲 **Planned** — `getStorage()` / `bucket(name?)` — obtain a Cloud Storage bucket handle scoped to the app's credentials — **Medium**
- [ ] 🔲 **Planned** — Object operations (upload/download/delete/list/signed URLs) — **Hard** (this is the full `@google-cloud/storage` surface once a bucket handle exists; scope of "Storage support" needs its own design discussion)

## Cloud Messaging (FCM)

Source: `firebase-admin-node`'s `messaging/` module (`Messaging` class).

- [ ] 🔲 **Planned** — `send` — send a single FCM message, with optional dry-run mode — **Medium**
- [ ] 🔲 **Planned** — `sendEach` — send up to 500 messages individually, returns a batch response — **Medium** (once `send` exists)
- [ ] 🔲 **Planned** — `sendEachForMulticast` — send one message to multiple registration tokens/FIDs — **Medium** (once `send` exists)
- [ ] 🔲 **Planned** — `subscribeToTopic` — subscribe device tokens to an FCM topic — **Easy**
- [ ] 🔲 **Planned** — `unsubscribeFromTopic` — unsubscribe device tokens from an FCM topic — **Easy**
- [ ] 🔲 **Planned** — `enableLegacyHttpTransport` — opt `sendEach`/`sendEachForMulticast` into HTTP/1.1 transport — **Easy** (once those exist)

## Remote Config

Source: `firebase-admin-node`'s `remote-config/` module (`RemoteConfig` class).

- [ ] 🔲 **Planned** — `getTemplate` — fetch the current active Remote Config template — **Medium**
- [ ] 🔲 **Planned** — `getTemplateAtVersion` — fetch a specific historical template version — **Easy** (once `getTemplate` exists)
- [ ] 🔲 **Planned** — `validateTemplate` — validate a template without publishing it — **Medium**
- [ ] 🔲 **Planned** — `publishTemplate` — deploy a template, with optional force-update — **Medium**
- [ ] 🔲 **Planned** — `rollback` — revert to a previously published template version — **Easy** (once `publishTemplate` exists)
- [ ] 🔲 **Planned** — `listVersions` — list published template versions chronologically — **Medium**
- [ ] 🔲 **Planned** — `createTemplateFromJSON` — build a template instance from a JSON string — **Easy**
- [ ] 🔲 **Planned** — `getServerTemplate` — fetch and cache the latest template for server-side use — **Medium**
- [ ] 🔲 **Planned** — `initServerTemplate` — construct a server template instance without a network fetch — **Easy**

## Security Rules

Source: `firebase-admin-node`'s `security-rules/` module (`SecurityRules` class).

- [ ] 🔲 **Planned** — `getRuleset` — fetch a ruleset by name — **Medium**
- [ ] 🔲 **Planned** — `getFirestoreRuleset` — fetch the ruleset currently applied to Firestore — **Medium**
- [ ] 🔲 **Planned** — `releaseFirestoreRulesetFromSource` — create and deploy a new Firestore ruleset from source — **Medium**
- [ ] 🔲 **Planned** — `releaseFirestoreRuleset` — apply an existing ruleset to Firestore — **Easy** (once ruleset CRUD exists)
- [ ] 🔲 **Planned** — `getStorageRuleset` — fetch the ruleset currently applied to a Storage bucket — **Medium**
- [ ] 🔲 **Planned** — `releaseStorageRulesetFromSource` — create and deploy a new Storage ruleset from source — **Medium**
- [ ] 🔲 **Planned** — `releaseStorageRuleset` — apply an existing ruleset to a Storage bucket — **Easy** (once ruleset CRUD exists)
- [ ] 🔲 **Planned** — `createRulesFileFromSource` — build a rules file object from a name + source — **Easy**
- [ ] 🔲 **Planned** — `createRuleset` — create a new ruleset from a rules file — **Medium**
- [ ] 🔲 **Planned** — `deleteRuleset` — delete a ruleset by name — **Easy**
- [ ] 🔲 **Planned** — `listRulesetMetadata` — paginated listing of ruleset metadata — **Medium**

## Project Management

Source: `firebase-admin-node`'s `project-management/` module (`ProjectManagement` class).

- [ ] 🔲 **Planned** — `listAndroidApps` — list up to 100 Android apps linked to the project — **Medium**
- [ ] 🔲 **Planned** — `listIosApps` — list up to 100 iOS apps linked to the project — **Medium**
- [ ] 🔲 **Planned** — `androidApp` — get an Android app reference by app id (no network call) — **Easy**
- [ ] 🔲 **Planned** — `iosApp` — get an iOS app reference by app id (no network call) — **Easy**
- [ ] 🔲 **Planned** — `shaCertificate` — build a SHA certificate object from a hash — **Easy**
- [ ] 🔲 **Planned** — `createAndroidApp` — provision a new Android app in the project — **Medium**
- [ ] 🔲 **Planned** — `createIosApp` — provision a new iOS app in the project — **Medium**
- [ ] 🔲 **Planned** — `listAppMetadata` — list metadata for up to 100 apps in the project — **Medium**
- [ ] 🔲 **Planned** — `setDisplayName` — update the Firebase project's display name — **Easy**

## Machine Learning

Source: `firebase-admin-node`'s `machine-learning/` module (`MachineLearning` class).

- [ ] 🔲 **Planned** — `createModel` — create an ML model in the project — **Hard**
- [ ] 🔲 **Planned** — `updateModel` — modify an ML model's metadata or file — **Medium** (once `createModel` exists)
- [ ] 🔲 **Planned** — `publishModel` — make a model available for client download — **Easy** (once model CRUD exists)
- [ ] 🔲 **Planned** — `unpublishModel` — remove a model from client availability — **Easy** (once model CRUD exists)
- [ ] 🔲 **Planned** — `getModel` — fetch a model by id — **Medium**
- [ ] 🔲 **Planned** — `listModels` — list models with optional filtering/pagination — **Medium**
- [ ] 🔲 **Planned** — `deleteModel` — delete a model — **Easy** (once model CRUD exists)

## App Check

Source: `firebase-admin-node`'s `app-check/` module (`AppCheck` class).

- [ ] 🔲 **Planned** — `createToken` — mint an App Check token for an app id — **Medium**
- [ ] 🔲 **Planned** — `verifyToken` — verify an App Check token (JWT) and return decoded claims — **Medium** (shares JWT-verification patterns with `verify_id_token`)

## Installations

Source: `firebase-admin-node`'s `installations/` module (`Installations` class).

- [ ] 🔲 **Planned** — `deleteInstallation` — delete an installation id and its associated data — **Easy**

## Cloud Functions (admin management)

Source: `firebase-admin-node`'s `functions/` module (`Functions`/`TaskQueue` classes). This is task-queue management for `onTaskDispatched` functions, not function deployment or runtime.

- [ ] 🔲 **Planned** — `taskQueue` — get a reference to a named function's task queue — **Medium**
- [ ] 🔲 **Planned** — `TaskQueue.enqueue` — add a task to the queue — **Medium** (once `taskQueue` exists)
- [ ] 🔲 **Planned** — `TaskQueue.delete` — remove an enqueued, not-yet-completed task — **Easy** (once `taskQueue` exists)

## Extensions

Source: `firebase-admin-node`'s `extensions/` module (`Extensions` class).

- [ ] 🔲 **Planned** — `runtime` — get a `Runtime` handle for modifying an extension instance's runtime data — **Medium**

## Not currently planned

These exist in the official SDK but have no design work started and are not
on the near-term path. PRs proposing a design are welcome:

- **Eventarc** — publishing custom events to Eventarc channels
- **Data Connect** — executing Data Connect queries/mutations from the backend
- **Phone Number Verification (Identity Platform)** — server-side phone number verification outside the standard client sign-in flow
