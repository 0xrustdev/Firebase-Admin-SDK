# Roadmap

This is the single source of truth for what this crate supports today versus
the official [Firebase Admin Node.js SDK](https://github.com/firebase/firebase-admin-node),
and what's left to build. Every method the official SDK exposes is listed
below, marked with one of:

| | Status |
|---|---|
| 🟢 | Done — implemented and working |
| 🟡 | Not started |
| ⚪ | Blocked — waiting on another unfinished item, named in the entry |
| 🔵 | In progress — an open PR or branch exists |
| 🔴 | Broken/regressed — was working, currently known-broken |

Every 🟡/⚪/🔵/🔴 item carries a leading effort tag — `[Easy]` / `[Medium]` /
`[Hard]`.

Looking to contribute? Pick any 🟡 item, especially an `[Easy]` one, and open
a PR — see [CONTRIBUTING.md](CONTRIBUTING.md). If you want to propose
something not listed here, open a feature request first (see
[CONTRIBUTING.md](CONTRIBUTING.md#reporting-bugs--requesting-features)).

This file lists *what* to build, not *when* — no version numbers are
promised here. See [CHANGELOG.md](CHANGELOG.md) for what has actually shipped.

## Authentication

Source: `firebase-admin-node`'s `BaseAuth` class. This crate's implementation
lives in [`src/auth/`](src/auth/).

- 🟢 `createCustomToken` — mint a Firebase custom token for a uid, signed locally with a service account key
- 🟢 `verifyIdToken` — verify a Firebase ID token and return its decoded claims
- 🟢 `getUser` — fetch a user by uid
- 🟢 `getUserByEmail` — fetch a user by email address
- 🟡 `[Easy]` — `getUserByPhoneNumber` — fetch a user by E.164 phone number (mirrors the existing `getUserByEmail` lookup path)
- 🟡 `[Medium]` — `getUserByProviderUid` — fetch a user by federated provider id + uid
- 🟡 `[Medium]` — `getUsers` — batch-fetch up to 100 users by mixed identifiers (uid/email/phone/provider); needs a `UserIdentifier` enum
- 🟢 `createUser` — create a new user
- 🟢 `updateUser` — update an existing user's properties
- 🟢 `setCustomUserClaims` — set or clear developer claims on a user
- 🟢 `deleteUser` — delete a user by uid
- 🟡 `[Medium]` — `deleteUsers` — bulk-delete up to 1000 users, returns per-user success/failure
- 🟢 `listUsers` — paginated listing of all users
- 🟡 `[Easy]` — `revokeRefreshTokens` — invalidate all of a user's existing sessions/refresh tokens (implemented as `updateUser` setting `validSince`)
- 🟡 `[Hard]` — `importUsers` — bulk-import up to 1000 users, including pre-hashed passwords; requires modeling Firebase's password hash configs
- 🟢 `createSessionCookie` — exchange a verified ID token for a long-lived session cookie
- 🟢 `verifySessionCookie` — verify a session cookie and return its decoded claims
- 🟡 `[Medium]` — `generatePasswordResetLink` — generate a password-reset email action link; needs a new `accounts:sendOobCode` endpoint
- ⚪ `[Easy]` — `generateEmailVerificationLink` — generate an email-verification action link (once `sendOobCode` exists)
- ⚪ `[Easy]` — `generateVerifyAndChangeEmailLink` — generate a verify-and-change-email action link (once `sendOobCode` exists)
- ⚪ `[Easy]` — `generateSignInWithEmailLink` — generate a passwordless sign-in email action link (once `sendOobCode` exists)
- 🟡 `[Hard]` — `listProviderConfigs` — list configured SAML/OIDC identity providers (niche)
- 🟡 `[Hard]` — `getProviderConfig` — fetch one SAML/OIDC provider config (niche)
- 🟡 `[Hard]` — `createProviderConfig` — create a SAML/OIDC provider config (niche)
- 🟡 `[Hard]` — `updateProviderConfig` — update a SAML/OIDC provider config (niche)
- 🟡 `[Hard]` — `deleteProviderConfig` — delete a SAML/OIDC provider config (niche)
- 🟡 `[Hard]` — `tenantManager` — get the `TenantManager` handle for the project
- ⚪ `[Hard]` — `TenantManager.createTenant` — create a new auth tenant (once `tenantManager` exists)
- ⚪ `[Medium]` — `TenantManager.getTenant` — fetch a tenant by id (once `tenantManager` exists)
- ⚪ `[Medium]` — `TenantManager.listTenants` — paginated listing of tenants (once `tenantManager` exists)
- ⚪ `[Medium]` — `TenantManager.updateTenant` — update a tenant's config (once `tenantManager` exists)
- ⚪ `[Easy]` — `TenantManager.deleteTenant` — delete a tenant (once `tenantManager` exists)
- 🟡 `[Hard]` — `TenantManager.authForTenant` — get a tenant-scoped `TenantAwareAuth` client; a distinct subclass overriding `verifyIdToken`/`createSessionCookie`/`verifySessionCookie` to scope them to the tenant, plus re-scoping every user-management operation above

## Cloud Firestore

Source: `firebase-admin-node`'s `firestore/` module, which is mostly a thin
wrapper re-exporting `@google-cloud/firestore` for a Firebase app's
credentials, plus a few admin-only additions.

- 🟡 `[Hard]` — `getFirestore()` — obtain a Firestore client scoped to the app's credentials/project (default or named database)
- 🟡 `[Hard]` — Document/collection CRUD (`get`, `set`, `update`, `delete`, `add`)
- 🟡 `[Hard]` — Queries (`where`, `orderBy`, `limit`, collection group queries)
- 🟡 `[Hard]` — Transactions (`runTransaction`)
- 🟡 `[Hard]` — Batched writes (`WriteBatch`)
- 🟡 `[Hard]` — `BulkWriter` — high-throughput batched writes with automatic retry
- ⚪ `[Medium]` — `recursiveDelete` — delete a document/collection and all descendants (once base CRUD exists)
- ⚪ `[Easy]` — `setLogFunction` — hook Firestore SDK internal logging (once base client exists)
- 🟡 `[Medium]` — Bundles (`BundleBuilder`) — package query/document snapshots for client caching

## Realtime Database

Source: `firebase-admin-node`'s `database/` module (`Database` class,
extending `@firebase/database-compat`'s `FirebaseDatabase`).

- 🟡 `[Hard]` — `getDatabase()` / `ref` / `refFromURL` — obtain a Database client and data references
- ⚪ `[Medium]` — `goOnline` / `goOffline` — control the client's realtime connection (once base client exists)
- 🟡 `[Medium]` — `getRules` — fetch currently applied security rules as a string (with comments)
- 🟡 `[Medium]` — `getRulesJSON` — fetch currently applied security rules as parsed JSON
- 🟡 `[Medium]` — `setRules` — deploy new security rules from a string, buffer, or object

## Cloud Storage

Source: `firebase-admin-node`'s `storage/` module — a thin wrapper handing
out `@google-cloud/storage` bucket handles under the app's credentials.

- 🟡 `[Medium]` — `getStorage()` / `bucket(name?)` — obtain a Cloud Storage bucket handle scoped to the app's credentials
- 🟡 `[Hard]` — Object operations (upload/download/delete/list/signed URLs) — this is the full `@google-cloud/storage` surface once a bucket handle exists; scope of "Storage support" needs its own design discussion

## Cloud Messaging (FCM)

Source: `firebase-admin-node`'s `messaging/` module (`Messaging` class).

- 🟡 `[Medium]` — `send` — send a single FCM message, with optional dry-run mode
- ⚪ `[Medium]` — `sendEach` — send up to 500 messages individually, returns a batch response (once `send` exists)
- ⚪ `[Medium]` — `sendEachForMulticast` — send one message to multiple registration tokens/FIDs (once `send` exists)
- 🟡 `[Easy]` — `subscribeToTopic` — subscribe device tokens to an FCM topic
- 🟡 `[Easy]` — `unsubscribeFromTopic` — unsubscribe device tokens from an FCM topic
- 🟡 `[Easy]` — `enableLegacyHttpTransport` — opt `sendEach`/`sendEachForMulticast` into HTTP/1.1 transport (once those exist)

## Remote Config

Source: `firebase-admin-node`'s `remote-config/` module (`RemoteConfig` class).

- 🟡 `[Medium]` — `getTemplate` — fetch the current active Remote Config template
- ⚪ `[Easy]` — `getTemplateAtVersion` — fetch a specific historical template version (once `getTemplate` exists)
- 🟡 `[Medium]` — `validateTemplate` — validate a template without publishing it
- 🟡 `[Medium]` — `publishTemplate` — deploy a template, with optional force-update
- ⚪ `[Easy]` — `rollback` — revert to a previously published template version (once `publishTemplate` exists)
- 🟡 `[Medium]` — `listVersions` — list published template versions chronologically
- 🟡 `[Easy]` — `createTemplateFromJSON` — build a template instance from a JSON string
- 🟡 `[Medium]` — `getServerTemplate` — fetch and cache the latest template for server-side use
- 🟡 `[Easy]` — `initServerTemplate` — construct a server template instance without a network fetch

## Security Rules

Source: `firebase-admin-node`'s `security-rules/` module (`SecurityRules` class).

- 🟡 `[Medium]` — `getRuleset` — fetch a ruleset by name
- 🟡 `[Medium]` — `getFirestoreRuleset` — fetch the ruleset currently applied to Firestore
- 🟡 `[Medium]` — `releaseFirestoreRulesetFromSource` — create and deploy a new Firestore ruleset from source
- ⚪ `[Easy]` — `releaseFirestoreRuleset` — apply an existing ruleset to Firestore (once ruleset CRUD exists)
- 🟡 `[Medium]` — `getStorageRuleset` — fetch the ruleset currently applied to a Storage bucket
- 🟡 `[Medium]` — `releaseStorageRulesetFromSource` — create and deploy a new Storage ruleset from source
- ⚪ `[Easy]` — `releaseStorageRuleset` — apply an existing ruleset to a Storage bucket (once ruleset CRUD exists)
- 🟡 `[Easy]` — `createRulesFileFromSource` — build a rules file object from a name + source
- 🟡 `[Medium]` — `createRuleset` — create a new ruleset from a rules file
- 🟡 `[Easy]` — `deleteRuleset` — delete a ruleset by name
- 🟡 `[Medium]` — `listRulesetMetadata` — paginated listing of ruleset metadata

## Project Management

Source: `firebase-admin-node`'s `project-management/` module (`ProjectManagement` class).

- 🟡 `[Medium]` — `listAndroidApps` — list up to 100 Android apps linked to the project
- 🟡 `[Medium]` — `listIosApps` — list up to 100 iOS apps linked to the project
- 🟡 `[Easy]` — `androidApp` — get an Android app reference by app id (no network call)
- 🟡 `[Easy]` — `iosApp` — get an iOS app reference by app id (no network call)
- 🟡 `[Easy]` — `shaCertificate` — build a SHA certificate object from a hash
- 🟡 `[Medium]` — `createAndroidApp` — provision a new Android app in the project
- 🟡 `[Medium]` — `createIosApp` — provision a new iOS app in the project
- 🟡 `[Medium]` — `listAppMetadata` — list metadata for up to 100 apps in the project
- 🟡 `[Easy]` — `setDisplayName` — update the Firebase project's display name

## Machine Learning

Source: `firebase-admin-node`'s `machine-learning/` module (`MachineLearning` class).

- 🟡 `[Hard]` — `createModel` — create an ML model in the project
- ⚪ `[Medium]` — `updateModel` — modify an ML model's metadata or file (once `createModel` exists)
- ⚪ `[Easy]` — `publishModel` — make a model available for client download (once model CRUD exists)
- ⚪ `[Easy]` — `unpublishModel` — remove a model from client availability (once model CRUD exists)
- 🟡 `[Medium]` — `getModel` — fetch a model by id
- 🟡 `[Medium]` — `listModels` — list models with optional filtering/pagination
- ⚪ `[Easy]` — `deleteModel` — delete a model (once model CRUD exists)

## App Check

Source: `firebase-admin-node`'s `app-check/` module (`AppCheck` class).

- 🟡 `[Medium]` — `createToken` — mint an App Check token for an app id
- 🟡 `[Medium]` — `verifyToken` — verify an App Check token (JWT) and return decoded claims; shares JWT-verification patterns with `verify_id_token`

## Installations

Source: `firebase-admin-node`'s `installations/` module (`Installations` class).

- 🟡 `[Easy]` — `deleteInstallation` — delete an installation id and its associated data

## Cloud Functions (admin management)

Source: `firebase-admin-node`'s `functions/` module (`Functions`/`TaskQueue` classes). This is task-queue management for `onTaskDispatched` functions, not function deployment or runtime.

- 🟡 `[Medium]` — `taskQueue` — get a reference to a named function's task queue
- ⚪ `[Medium]` — `TaskQueue.enqueue` — add a task to the queue (once `taskQueue` exists)
- ⚪ `[Easy]` — `TaskQueue.delete` — remove an enqueued, not-yet-completed task (once `taskQueue` exists)

## Extensions

Source: `firebase-admin-node`'s `extensions/` module (`Extensions` class).

- 🟡 `[Medium]` — `runtime` — get a `Runtime` handle for modifying an extension instance's runtime data

## Not currently planned

These exist in the official SDK but have no design work started and are not
on the near-term path. PRs proposing a design are welcome:

- **Eventarc** — publishing custom events to Eventarc channels
- **Data Connect** — executing Data Connect queries/mutations from the backend
- **Phone Number Verification (Identity Platform)** — server-side phone number verification outside the standard client sign-in flow
