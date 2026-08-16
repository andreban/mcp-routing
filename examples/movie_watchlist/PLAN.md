# CineList: Movie Watchlist & Recommendation MCP Server Plan

A comprehensive, real-world Model Context Protocol (MCP) server example for **CineList**, demonstrating all features of `mcp-routing` in a single cohesive, production-grade application.

---

## 1. Overview & Architecture

CineList enables AI agents to:
1. Search and inspect a rich curated movie catalog (Public).
2. Build and maintain personalized movie watchlists (Protected, multi-user).
3. Rate movies (1–10 scale) with review notes and record watch history (Protected).
4. Generate intelligent movie recommendations based on user ratings and watch history (Protected).
5. Provide streaming platform availability checks via request middleware and user preferences.
6. Generate visual multimodal movie posters/cards.
7. Support multi-turn prompts and context-aware, privacy-scoped autocompletions.

```
                  ┌──────────────────────────────────────────────────────────┐
                  │              CineList MCP Server (Axum)                  │
                  │       McpRouter::new("cinelist-mcp", "1.0.0")            │
                  └─────────────┬──────────────────────────────┬─────────────┘
                                │                              │
          ┌─────────────────────┴────────────┐   ┌─────────────┴────────────────────┐
          │     Tools (Public & Protected)   │   │       Resources & Templates      │
          ├──────────────────────────────────┤   ├──────────────────────────────────┤
          │ [Public]                         │   │ [Public Resources]               │
          │ • search_movies                  │   │ • movies://genres/catalog (MD)   │
          │ • get_movie_details              │   │ • movies://curated/top250 (JSON) │
          │ • generate_movie_poster (PNG)    │   │ • movies://branding/logo.png(PNG)│
          │                                  │   │ • movies://catalog/{genre}/{id}  │
          │ [Protected / Auth Required]      │   │                                  │
          │ • create_watchlist               │   │ [Protected User Resources]       │
          │ • add_to_watchlist               │   │ • movies://users/{id}/watchlists │
          │ • remove_from_watchlist          │   │ • movies://users/{id}/history    │
          │ • delete_watchlist               │   │ (Enforces caller identity match) │
          │ • rate_movie (1-10 range)        │   │                                  │
          │ • get_recommendations            │   │                                  │
          └──────────────────────────────────┘   └──────────────────────────────────┘
                                │                              │
          ┌─────────────────────┴────────────┐   ┌─────────────┴────────────────────┐
          │      Prompts & Workflows         │   │    Extractors, State & Security  │
          ├──────────────────────────────────┤   ├──────────────────────────────────┤
          │ • movie_night_planner (Multi-turn│   │ • State<Arc<RwLock<MovieDb>>>    │
          │ • draft_review (Structured)      │   │ • BearerAuth (Token validation)  │
          │ • Autocompletions:               │   │ • Extension<StreamingServices>   │
          │   - genre, mood (Public)         │   │ • IDOR & Access Control Guard    │
          │   - list_name (User-Scoped)      │   │ • Public & Private HTTP Caching  │
          │   - genre -> movie_id (context)  │   │                                  │
          └──────────────────────────────────┘   └──────────────────────────────────┘
```

---

## 2. Security, Privacy & Multi-User Identity Model

### 2.1 Authentication & Token Resolution
* **Mechanism**: Handlers for protected endpoints extract [`BearerAuth`](file:///src/extract/auth.rs) (`Authorization: Bearer <token>`).
* **Validation**: The server validates tokens against an in-memory token registry (`auth_tokens: HashMap<String, String>`):
  * `"token_alice_secret"` $\rightarrow$ User `"alice"`
  * `"token_bob_secret"` $\rightarrow$ User `"bob"`
* **Explicit Rejection**: Invalid or missing tokens on protected tools/resources return an explicit authentication error (`401 Unauthorized` / `-32001` or [`ExtractionError::InvalidAuthorization`](file:///src/extract/error.rs)), rather than silently falling back to a shared account.
* **Public Tools Exemption**: Public catalog tools (`search_movies`, `get_movie_details`, `generate_movie_poster`) do not require authentication.

### 2.2 Broken Object Level Authorization (IDOR) Protection
* When reading user resources (`movies://users/{user_id}/watchlists/{list_name}` or `movies://users/{user_id}/history`):
  * The resource handler extracts [`BearerAuth`](file:///src/extract/auth.rs) to resolve the authenticated caller.
  * The handler verifies that `authenticated_user_id == requested_user_id`.
  * If a mismatched ID is requested (e.g., Bob trying to read Alice's private watchlists), the request is rejected with a forbidden/authorization error.

### 2.3 Cross-Tenant Privacy & Scoped Autocomplete
* `list_name` autocompletion resolves the caller's identity and only returns list names belonging to the caller's private profile.
* User recommendation calculations and watch histories are strictly isolated per `user_id`.

### 2.4 HTTP Caching & Privacy
* **Public Catalog Resources**: Configured with `CacheScope::Public` (`ttl_ms: 86400000`) and `ETag` generation.
* **User Resources**: Configured with `CacheScope::Private` (`Cache-Control: private, max-age=10`) and `Vary: Authorization` so intermediate caches do not leak private data between users.

---

## 3. In-Memory Data Model (`State<Arc<RwLock<MovieDb>>>`)

```rust
pub struct MovieDb {
    pub catalog: HashMap<String, Movie>,
    pub users: HashMap<String, UserProfile>,
    pub auth_tokens: HashMap<String, String>, // token -> user_id
}

pub struct UserProfile {
    pub user_id: String,
    pub display_name: String,
    pub watchlists: HashMap<String, Watchlist>,  // list_name -> Watchlist
    pub ratings: HashMap<String, MovieRating>,    // movie_id -> Rating
    pub streaming_subscriptions: Vec<String>,
}

pub struct Watchlist {
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<WatchlistItem>,
    pub created_at: DateTime<Utc>,
}

pub struct WatchlistItem {
    pub movie_id: String,
    pub added_at: DateTime<Utc>,
    pub notes: Option<String>,
}

pub struct MovieRating {
    pub movie_id: String,
    pub rating: f32, // 1.0 - 10.0
    pub review: Option<String>,
    pub watched_at: DateTime<Utc>,
}
```

### Pre-seeded Demo Data
* **Catalog**: 15+ iconic films across genres (Sci-Fi, Drama, Thriller, Animation, Action, Comedy, Horror) with director, cast, runtime, synopsis, and streaming availability.
* **User Alice (`token_alice_secret`)**:
  * Subscriptions: `["Netflix", "Criterion Channel"]`
  * Watchlist: `"Sci-Fi Essentials"` (`["interstellar", "blade-runner-2049"]`)
  * Ratings: `2001-a-space-odyssey` (9.5), `arrival` (9.0)
* **User Bob (`token_bob_secret`)**:
  * Subscriptions: `["Max", "Prime Video"]`
  * Watchlist: `"Classic Noir & Thrillers"` (`["parasite", "se7en"]`)
  * Ratings: `the-dark-knight` (9.8), `spirited-away` (9.2)

---

## 4. Detailed MCP Feature Mapping

### 4.1 Server Discovery (`server/discover`)
* Implementation: `Implementation::new("cinelist-mcp", "1.0.0")`
* Instructions: Guides the LLM on how to explore catalog genres, provide bearer tokens for personalized watchlists and ratings, and request recommendations.
* Cache directive: Public caching for 1 hour (`3_600_000` ms) with `ETag`.
* Logging: Advertises `LoggingLevel::Info` as baseline threshold.

### 4.2 Dedicated Tools (`tools/*`) with Schema Validation & Structured Outputs
* **`search_movies`** *(Public)*:
  * Schema: Optional `query`, `genre`, `min_year`, `max_year`, `min_rating`, `limit`.
  * Return: `(Json<MovieSearchResults>, String)` (Structured JSON payload + Markdown summary).
* **`get_movie_details`** *(Public / Enhanced with Auth)*:
  * Schema: Required `movie_id` (`string`).
  * Return: `Json<MovieDetails>` including streaming availability matched with user subscriptions (if authenticated).
* **`create_watchlist`** *(Protected)*:
  * Schema: Required `list_name` (`string`, `minLength: 1`), optional `description`.
  * Return: `Json<WatchlistSummary>`.
* **`add_to_watchlist`** *(Protected)*:
  * Schema: Required `list_name` (`string`), `movie_id` (`string`), optional `notes`.
  * Return: `Json<WatchlistSummary>`.
* **`remove_from_watchlist`** *(Protected)*:
  * Schema: Required `list_name` (`string`), `movie_id` (`string`).
  * Return: `Json<WatchlistSummary>`.
* **`delete_watchlist`** *(Protected)*:
  * Schema: Required `list_name` (`string`).
  * Return: `String` confirmation.
* **`rate_movie`** *(Protected)*:
  * Schema: Required `movie_id` (`string`), `rating` (`number`, `minimum: 1`, `maximum: 10`), optional `review`.
  * Return: `Json<RatingReceipt>`.
* **`get_recommendations`** *(Protected)*:
  * Schema: Optional `genre`, `mood`, `limit`.
  * Return: `Json<RecommendationResult>` with affinity scores computed from caller's high ratings ($\ge 7.0$), filtering out watched films.
* **`generate_movie_poster`** *(Public)*:
  * Schema: Required `movie_id` (`string`).
  * Return: Multimodal `CallToolResult` with Base64 PNG badge/card and markdown synopsis.
* **Tool Annotations**: Configured on critical tools (`audience: ["user", "assistant"]`, `priority: 0.9`).

### 4.3 Resources & URI Templates (`resources/*`)
* **Direct Static Resources (Public)**:
  * `movies://genres/catalog`: Markdown guide to all supported genres and catalog statistics.
  * `movies://curated/top250`: JSON document containing all-time classics.
  * `movies://branding/cinelist-logo.png`: Binary PNG image blob.
* **RFC 6570 Dynamic URI Templates**:
  * `movies://catalog/{genre}/{movie_id}` *(Public)*: Dynamic resource reading individual movie records.
  * `movies://users/{user_id}/watchlists/{list_id}` *(Protected)*: Dynamic resource reading user watchlists. Enforces caller ID check.
  * `movies://users/{user_id}/history` *(Protected)*: Dynamic resource reading user ratings and watch logs. Enforces caller ID check.

### 4.4 Prompts (`prompts/*`)
* **`movie_night_planner`**:
  * Multi-turn prompt with system role instructions and user scenario.
  * Parameters: `group_size`, `mood`, `max_runtime_minutes`, `disliked_genres`.
  * Generates a tailored double-feature with discussion starters and streaming advice.
* **`draft_review`**:
  * Parameters: `movie_id`, `rating`, `raw_thoughts`.
  * Generates an articulate, spoiler-free film review formatted for social sharing.

### 4.5 Autocompletions (`completion/complete`)
* `genre` completion *(Public)*: Static list (`"Sci-Fi"`, `"Drama"`, `"Action"`, `"Animation"`, `"Thriller"`, `"Comedy"`, `"Horror"`).
* `mood` completion *(Public)*: Static list (`"mind-bending"`, `"cozy"`, `"edge-of-your-seat"`, `"tearjerker"`, `"dark-comedy"`, `"inspiring"`).
* `list_name` completion *(Protected / User-Scoped)*: Dynamic provider inspecting only the authenticated user's watchlists.
* `movie_id` completion *(Context-Aware)*: Inspects `genre` in `CompleteContext` to filter suggested movie IDs to that specific genre.

### 4.6 Extractors & Middleware Context
* `BearerAuth`: Validates caller token and resolves `user_id`.
* `State<Arc<RwLock<MovieDb>>>`: Thread-safe shared database state.
* `Extension<StreamingSubscriptions>`: Tower middleware injecting active streaming subscriptions.
* `Meta` / `RequestMetaObject`: Client metadata and protocol version tracking.
* `CurrentLoggingLevel` & `Option<LoggingLevel>`: Logging and diagnostics threshold.
* `RequestContext` & `HeaderMap`: MCP request headers and tracing context.

---

## 5. Implementation Steps

1. Create `examples/movie_watchlist/main.rs`.
2. Define domain models (`Movie`, `WatchlistItem`, `Watchlist`, `MovieRating`, `UserProfile`, `MovieDb`, `StreamingSubscriptions`).
3. Seed catalog with 15+ iconic films and pre-seed user accounts for `alice` and `bob` with authentication tokens.
4. Implement authentication and authorization helper resolving `BearerAuth` to validated `user_id`.
5. Implement all tool handlers, resource handlers, prompt handlers, and completion handlers with IDOR checks and scoped state access.
6. Assemble the `McpRouter` registering all capabilities, caches, and schema validation.
7. Configure the Axum web application with `Extension` middleware and `State`.
8. Verify compilation and test requests with `cargo check --examples` and `cargo run --example movie_watchlist`.

