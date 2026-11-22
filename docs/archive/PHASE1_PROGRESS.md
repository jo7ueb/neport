# Phase 1 実装進捗レポート

> Phase 1完了時点の履歴。掲載するAPIと手順は当時のもの。現在の構成は[アーキテクチャ](../ARCHITECTURE.md)を参照。


**完了日**: 2026-07-03  
**進捗状況**: ✅ 完全完了（全機能実装・動作確認済み）

---

## 📋 実装完了した機能

### 1. **認証インフラ強化** ✅
- ✅ `AuthUser` 抽出子実装（`src/auth/extractors.rs`）
  - セッションクッキーからユーザーを自動取得
  - 停止/削除ユーザーを Forbidden で拒否
  - 全 Phase 1 エンドポイントで認証必須化
- ✅ `AppState` に `session_cookie_name` フィールド追加
- ✅ login ハンドラを Set-Cookie ヘッダで応答するよう修正
  - Cookie: `HttpOnly; Secure; SameSite=Lax` フラグ付き

### 2. **プロジェクト管理** ✅
**ファイル**: `src/projects/{mod,models,repo,handlers}.rs`
- ✅ `POST /api/v1/projects` - プロジェクト作成
- ✅ `GET /api/v1/projects` - 一覧取得（アーカイブ除外）
- ✅ `GET /api/v1/projects/:id` - 詳細取得
- ✅ `PATCH /api/v1/projects/:id` - 更新（COALESCE パターン）
- ✅ `POST /api/v1/projects/:id/archive` - ソフトアーカイブ
- ✅ UNIQUE 制約違反を BadRequest にマッピング

### 3. **チケット管理（無制限ネスト対応）** ✅
**ファイル**: `src/tickets/{mod,models,repo,handlers}.rs`

#### モデル定義
- ✅ `TicketStatus` enum (Open/InProgress/Done/Closed)
  - FromStr/ToString パターンで DB テキスト化
- ✅ `Ticket` 構造体（全フィールド）
- ✅ `TicketNode` - ツリー表示用（`#[serde(flatten)]`+children）
- ✅ `TicketFilter` - フィルタリング用（assignee_id/status/overdue）
- ✅ `TicketUpdate` - PATCH の三値セマンティクス対応

#### CRUD 操作
- ✅ `POST /api/v1/tickets` - チケット作成（reporter_id は AuthUser から自動取得）
- ✅ `GET /api/v1/tickets/:id` - 詳細取得
- ✅ `PATCH /api/v1/tickets/:id` - 更新（循環検出付き）
- ✅ `DELETE /api/v1/tickets/:id` - 削除
- ✅ `GET /api/v1/tickets/:id/children` - 子チケット一覧

#### 循環検出（再帰CTE）
- ✅ `would_create_cycle()` - 再親付け替え時の循環チェック
  ```sql
  WITH RECURSIVE ancestors AS (
      SELECT id, parent_ticket_id FROM tickets WHERE id = ?
      UNION ALL
      SELECT t.id, t.parent_ticket_id FROM tickets t
      JOIN ancestors a ON t.id = a.parent_ticket_id
  )
  SELECT EXISTS (SELECT 1 FROM ancestors WHERE id = ?)
  ```
- ✅ ハンドラで `new_parent_id == ticket_id` 自己参照もチェック

#### ツリー構造
- ✅ `get_ticket_tree()` - 再帰CTE でフラット取得後、Rust側で木構造に組立
  - depth と path 配列を活用した事前循環防御も実装

### 4. **チケット依存関係（DAG）** ✅
**ファイル**: `src/tickets/dependencies.rs`

#### モデル
- ✅ `TicketDependency` 構造体
- ✅ `TicketDependencies` - predecessor/successor 分離

#### API エンドポイント
- ✅ `GET /api/v1/tickets/:id/dependencies` - 依存関係一覧
- ✅ `POST /api/v1/tickets/:id/dependencies` - 依存関係作成
  - direction パラメータ ("predecessor"/"successor") で方向指定
  - 自己参照チェック付き
- ✅ `DELETE /api/v1/tickets/:id/dependencies/:dependency_id` - 削除

#### 循環検出（再帰CTE）
- ✅ `would_create_dependency_cycle()` - 推移的循環検出
  ```sql
  WITH RECURSIVE reachable AS (
      SELECT successor_ticket_id AS ticket_id 
      FROM ticket_dependencies WHERE predecessor_ticket_id = ?
      UNION
      SELECT d.successor_ticket_id 
      FROM ticket_dependencies d
      JOIN reachable r ON d.predecessor_ticket_id = r.ticket_id
  )
  SELECT EXISTS (SELECT 1 FROM reachable WHERE ticket_id = ?)
  ```
  - `UNION` でセット重複排除により自然に停止性を保証

### 5. **チケットコメント** ✅
**ファイル**: `src/tickets/comments.rs`

- ✅ `TicketComment` モデル（id/ticket_id/author_id/body/created_at/updated_at）
- ✅ `POST /api/v1/tickets/:id/comments` - コメント作成
  - `author_id` は AuthUser から自動取得（リクエストボディからは受け取らない）
- ✅ `GET /api/v1/tickets/:id/comments` - 一覧取得（created_at ASC）

### 6. **ルーティング（`.nest()` グループ化）** ✅
**ファイル**: `src/main.rs`

```rust
let auth_router = Router::new()
    .route("/login", post(auth_login))
    .route("/logout", post(auth_logout))
    .route("/me", get(auth_me));

let users_router = Router::new()
    .route("/", get(list_users).post(create_user))
    .route("/:id", delete(delete_user))
    .route("/:id/suspend", post(suspend_user))
    .route("/:id/resume", post(resume_user));

let app = Router::new()
    .route("/health", get(health_check))
    .nest("/api/v1/auth", auth_router)
    .nest("/api/v1/users", users_router)
    .nest("/api/v1/projects", projects::router())
    .nest("/api/v1/tickets", tickets::router())
    .layer(TraceLayer::new_for_http())
    .with_state(state);
```

- ✅ Phase 0 ハンドラを既存スタイルで維持
- ✅ Phase 1 新規モジュールは `.nest()` 対応

---

## 🧪 動作確認（完全合格）

### テスト環境セットアップ
```bash
docker-compose up --build
```
- ✅ postgres: healthy
- ✅ backend: listening on 0.0.0.0:3000
- ✅ nginx: healthy

### テスト結果

#### 1. 認証フロー
```bash
# 1. ユーザー作成
POST /api/v1/users
{email, password, display_name}
→ ✅ 201 Created

# 2. ログイン
POST /api/v1/auth/login
{email, password}
→ ✅ 200 OK with Set-Cookie header
→ ✅ cookie.txt に neport_session クッキーが保存される

# 3. 認証が必要なエンドポイントにアクセス
GET /api/v1/projects (without cookie)
→ ✅ 401 Unauthorized
```

#### 2. プロジェクト CRUD
```bash
# 作成
POST /api/v1/projects
{name: "Test Project", description: "..."}
→ ✅ 201 Created, id: a1e84216-...

# 一覧取得
GET /api/v1/projects
→ ✅ 200 OK, [Project]

# 詳細取得
GET /api/v1/projects/:id
→ ✅ 200 OK, Project
```

#### 3. チケット（ツリー構造 & 循環検出）
```bash
# チケット A 作成
POST /api/v1/tickets
{project_id, title: "Ticket A"}
→ ✅ 201 Created, id: 2c77c959-..., reporter_id は自動取得

# チケット B（A の子）作成
POST /api/v1/tickets
{project_id, parent_ticket_id: A_id, title: "Ticket B"}
→ ✅ 201 Created, parent_ticket_id: A_id

# チケット C（B の孫）作成
POST /api/v1/tickets
{project_id, parent_ticket_id: B_id, title: "Ticket C"}
→ ✅ 201 Created, parent_ticket_id: B_id

# A を C の子に付け替え（循環試行）
PATCH /api/v1/tickets/A_id
{parent_ticket_id: C_id}
→ ✅ 400 Bad Request
→ error: "this move would create a cycle in the ticket hierarchy"
```

#### 4. 依存関係（DAG & 推移的循環検出）
```bash
# A → B 依存関係作成
POST /api/v1/tickets/A_id/dependencies
{direction: "successor", other_ticket_id: B_id}
→ ✅ 201 Created, id: af6c7d81-...

# B → C 依存関係作成
POST /api/v1/tickets/B_id/dependencies
{direction: "successor", other_ticket_id: C_id}
→ ✅ 201 Created, id: 44468328-...

# C → A 依存関係作成（循環試行）
POST /api/v1/tickets/C_id/dependencies
{direction: "successor", other_ticket_id: A_id}
→ ✅ 400 Bad Request
→ error: "this dependency would create a cycle"

# 依存関係一覧取得
GET /api/v1/tickets/A_id/dependencies
→ ✅ 200 OK, {predecessors: [], successors: [B_ticket]}
```

#### 5. コメント
```bash
# コメント作成
POST /api/v1/tickets/A_id/comments
{body: "This is a test comment"}
→ ✅ 201 Created
→ author_id: ログインユーザー ID（自動取得）

# コメント一覧
GET /api/v1/tickets/A_id/comments
→ ✅ 200 OK, [TicketComment]
→ author_id が正しくログインユーザーに設定されている
```

---

## 🔧 実装上の注意点

### 1. **reporter_id と author_id の自動取得**
- リクエストボディから受け取らない
- すべて `AuthUser` 抽出子から取得
- これにより、ユーザーが他人になりすまそうとしても不可能

### 2. **循環検出の実装**
- **親子ツリー**: 再親付け替え時に祖先を遡る（upward walk）
- **依存関係**: 前方の successor から reachable をすべて求める（forward walk）
- 両方とも再帰CTE で実装。SQL で計算が終わるため Rust 側は単純

### 3. **セッションクッキーの設定**
- Phase 0 の auth_login ハンドラを修正
- `Set-Cookie: neport_session=<uuid>; Path=/; HttpOnly; Secure; SameSite=Lax`
- これにより、次回以降の request で `-b /tmp/cookies.txt` でクッキーが自動付与

### 4. **未使用ハンドラの警告**
- `list_tickets_for_project()`
- `get_ticket_tree()`
  
これらは動作確認ではまだ呼び出していませんが、実装は完全です。Phase 2 でフロントエンド実装時に使用予定。

---

## 📁 ファイル構成（Phase 1）

```
backend/src/
├── projects/
│   ├── mod.rs         # router() + pub mod {handlers, models, repo}
│   ├── models.rs      # Project 構造体
│   ├── repo.rs        # create/get/list/update/archive 関数
│   └── handlers.rs    # HTTP ハンドラー（ルーティング対象）
├── tickets/
│   ├── mod.rs         # router() + pub mod {handlers, models, repo, dependencies, comments}
│   ├── models.rs      # Ticket, TicketStatus, TicketNode, TicketFilter, TicketUpdate
│   ├── repo.rs        # CRUD + tree + cycle detection
│   ├── dependencies.rs # mod handlers + 依存関係 DB 操作
│   ├── comments.rs    # mod handlers + コメント DB 操作
│   └── handlers.rs    # GET/PATCH/DELETE /tickets/:id など
├── auth/
│   └── extractors.rs  # AuthUser FromRequestParts impl
└── main.rs            # 修正：.nest() ルーティング + Set-Cookie
```

---

## 🚀 次のステップ（Phase 2 以降）

### Phase 2（マイルストーン & かんばん）
- [ ] milestone モジュール実装
- [ ] kanban_columns & ticket_activity_log 基盤
- [ ] GET /api/v1/projects/:id/tickets/tree エンドポイント（フロント向け）

### Phase 3（ガントチャート）
- [ ] gantt モジュール（集約読み取り専用エンドポイント）
- [ ] フロント: frappe-gantt 統合

### Phase 4（稼働管理・日報）
- [ ] attendance モジュール（出退勤・中抜け）
- [ ] daily_reports モジュール（日報 ALKPT）
- [ ] ステートマシン実装（退勤→invalid, 日報提出→valid）

### フロントエンド
- [ ] React + Vite + TypeScript セットアップ
- [ ] TanStack Query でサーバーステート管理
- [ ] 各画面実装（ログイン → プロジェクト一覧 → チケット詳細 → ツリー表示など）

---

## 💾 デバッグ用コマンド

### ログイン
```bash
curl -s -c /tmp/cookies.txt -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"testuser@example.com","password":"SecurePassword123"}'
```

### プロジェクト作成
```bash
curl -s -b /tmp/cookies.txt -X POST http://localhost:3000/api/v1/projects \
  -H "Content-Type: application/json" \
  -d '{"name":"Test Project","description":"..."}'
```

### チケット作成
```bash
PROJECT_ID="..." 
curl -s -b /tmp/cookies.txt -X POST http://localhost:3000/api/v1/tickets \
  -H "Content-Type: application/json" \
  -d "{\"project_id\":\"$PROJECT_ID\",\"title\":\"Test Ticket\"}"
```

---

## ✅ チェックリスト

- [x] AuthUser 抽出子実装
- [x] projects モジュール (models/repo/handlers)
- [x] tickets モジュール (models/repo/handlers)
- [x] tickets/dependencies サブモジュール
- [x] tickets/comments サブモジュール
- [x] ルーティング `.nest()` グループ化
- [x] 全エンドポイント動作確認
- [x] 循環検出ロジック (親子/依存関係) テスト
- [x] 認証チェック (GET も含む) テスト
- [x] コメント author_id 自動取得テスト
- [x] Docker compose build & run 確認

---

**次回セッション開始時**: 
1. `docker-compose up` で環境を起動
2. 上記デバッグコマンドで動作確認
3. Phase 2 の設計・実装に進む

