# 開発ガイド - Neport（タスク管理システム & 稼働管理システム）

## プロジェクト概要

Redmine から移行するための、シンプルなタスク管理・稼働管理・日報システム。
Rust + React + PostgreSQL で構成される内製ツール。

## アーキテクチャ

### 全体構成

```
Frontend (React/Vite)
  ↓ REST API
Backend (Rust/axum)
  ↓ SQL
PostgreSQL
```

### ディレクトリ構成

```
rehearsal/
├── backend/           # Rust バックエンド（REST API サーバー）
├── frontend/          # React フロントエンド（SPA）
├── docker-compose.yml # 標準の起動手段（`docker compose up` で本番相当の設定で立ち上がる）
├── docker-compose.test.yml # 統合テスト実行用オーバーレイ（rate limit 緩和 + 専用テストDB）
├── nginx.conf         # リバースプロキシ設定
├── README.md          # ユーザー向けドキュメント
└── CLAUDE.md          # 開発者向けガイド（本ファイル）
```

## バックエンド（Rust）

### 設計方針

1. **Webフレームワーク**: axum
   - tower/hyper ベース
   - extractor ベースの依存性注入（DI）が強力
   - session/CSRF/ロール管理と相性が良い

2. **DB アクセス**: sqlx
   - ORM ではなく、型安全なクエリビルダ
   - 再帰 CTE（チケットツリー、依存グラフ）を素の SQL で記述可能
   - クエリは全て実行時チェック（`sqlx::query()`/`query_as::<_, T>()`）。
     `query!`/`query_as!` マクロによるコンパイル時検証・offline mode は未導入

3. **マイグレーション**: sqlx migrate
   - 単純な `.sql` ファイル管理
   - Docker entrypoint で自動実行

### モジュール構成

```
src/
├── main.rs            # bootstrap、ルーター構築
├── config.rs          # 環境変数読込（dotenv）
├── error.rs           # AppError -> HTTP レスポンスマッピング
├── state.rs           # AppState（DB pool など）
├── auth/              # 認証
│   ├── mod.rs
│   ├── models.rs      # User, UserRole, UserStatus
│   ├── password.rs    # argon2id ハッシング
│   ├── csrf.rs        # CSRF トークン生成
│   ├── session.rs     # session CRUD
│   └── extractors.rs  # AuthUser, AdminUser extractors
├── users/             # ユーザー管理
│   ├── mod.rs
│   └── repo.rs        # DB クエリ
├── projects/          # プロジェクト管理（Phase 1）
├── tickets/           # チケット管理（Phase 1）
├── milestones/        # マイルストーン（Phase 2）
├── kanban/            # かんばん（Phase 2）
├── gantt/             # ガントチャート（Phase 3）
├── work_sessions/     # 作業時間管理（Phase 4）
└── daily_reports/     # 日報（Phase 4）
```

### コード規約

1. **エラーハンドリング**
   - `Result<T>` = `std::result::Result<T, AppError>`
   - 全エンドポイントは `Result<Json<T>>` を返す
   - `?` 演算子で自動的に HTTP レスポンスに変換

2. **モジュール構成**
   - 各ドメイン（users, tickets など）は `mod.rs` + `repo.rs` で構成
   - `repo.rs` は DB 操作のみ（ビジネスロジック不要）
   - ハンドラーは `main.rs` に記述（小規模時）

3. **命名規則**
   - 関数: snake_case
   - 構造体: PascalCase
   - enum: PascalCase
   - 定数: UPPER_SNAKE_CASE

4. **型安全性**
   - 強く型付けされた request/response 構造体を使用
   - `serde` で JSON serialize/deserialize
   - sqlx で SQL の型は実行時にチェック（マクロによるコンパイル時検証は未導入）

### 開発フロー

1. **マイグレーション追加**
   ```bash
   # migrations/YYYYMMDDHHMMSS_description.sql を作成
   # sqlx migrate add -r description でテンプレート生成可能
   ```

2. **新モジュール追加**
   ```
   1. src/new_feature/ を作成
   2. src/new_feature/mod.rs を作成
   3. src/new_feature/repo.rs で DB クエリ記述
   4. src/main.rs でハンドラーを追加
   5. router に route を追加
   ```

3. **ビルド・実行**
   ```bash
   docker compose up --build
   ```

## フロントエンド（React）

**現在**: ディレクトリのみ存在（`frontend/dist/index.html` はプレースホルダー）

**Phase 1 で実装予定**:
- Vite + React + TypeScript + React Router
- TanStack Query（server state 管理）
- Tailwind CSS（スタイリング）

### 設計予定

- ページ: login, projects, tickets, kanban, gantt, milestones, work-sessions, daily-report
- Gantt ライブラリ: frappe-gantt（軽量無償） → dhtmlx-gantt（必要なら）
- Kanban: @dnd-kit で自作（固定カラム想定）

## データベース

### 設計方針

1. **主キー**: UUID（v4）
   - ID enumeration 回避
   - 分散環境での ID 生成が容易

2. **タイムスタンプ**: TIMESTAMPTZ
   - タイムゾーン考慮必須
   - UTC で統一

3. **ソフトデリート**: status フラグ
   - 履歴の参照整合性を保持
   - 監査ログとして機能

4. **インデックス**
   - 外部キー
   - where 句で頻用されるカラム
   - 部分インデックス（e.g., `due_date WHERE status != 'done'`）

### 主要テーブル

| テーブル | 用途 | フェーズ |
|---------|------|--------|
| users | ユーザー | 0 ✅ |
| sessions | セッション管理 | 0 ✅ |
| projects | プロジェクト | 1 ✅ |
| tickets | チケット（ツリー構造） | 1 ✅ |
| ticket_dependencies | 依存関係（有向グラフ） | 1 ✅ |
| ticket_comments | コメント | 1 ✅ |
| milestones | マイルストーン | 2 ✅ |
| kanban_columns | かんばんカラム設定 | 2 ✅ |
| ticket_activity_log | 活動ログ（日報用） | 2 ✅ |
| audit_log | システム全体の監査ログ（ログイン/ログアウト/パスワード変更、ユーザー停止・削除、プロジェクトアーカイブ等。実行者のIPアドレスも記録） | 2 ✅ |
| work_sessions | 作業開始・終了（中抜けは持たず、開始/終了ペアの列挙のみ） | 4 ✅ |
| daily_reports | 日報 | 4 ✅ |

## 認証・セキュリティ

### 実装済み（Phase 0）

- ✅ パスワード: Argon2id（ソルト内蔵）
- ✅ セッション: DB 管理
- ✅ CSRF トークン: 生成・検証（`AuthUser` extractor が GET 以外の全リクエストで
  `X-CSRF-Token` ヘッダーとセッションの `csrf_token` を照合。不一致は 403）
- ✅ rate limiting: `POST /api/v1/auth/login` に `tower-governor`（IPごと、
  `SmartIpKeyExtractor` で `X-Forwarded-For`/`X-Real-IP` を優先参照）を適用。
  閾値は `LOGIN_RATE_LIMIT_PER_SECOND`/`LOGIN_RATE_LIMIT_BURST` 環境変数で
  調整可能（既定値は本番向けの控えめな設定。`docker-compose.yml` 単体では
  未設定のままこの既定値に委ねる ＝ `docker compose up` は常に本番相当の
  安全な設定で立ち上がる。統合テストが同一IPから大量ログインするための
  緩和値は `docker-compose.test.yml` オーバーレイでのみ適用される）
- ✅ テストDB分離: 統合テストは開発/本番用の `db` サービスとは別の
  `test-db` サービス・別ボリューム（`pgdata_test`）に対して実行される
  （`docker-compose.test.yml` が `backend`/`migrator` の `DATABASE_URL` を
  `test-db` に向け替える）。統合テストは大量のテストユーザー・プロジェクト
  を作成しっぱなしにするため、`db` を共用すると開発中に見ているデータに
  混入してしまう問題があった
- ✅ パスワードリセット: 管理者トリガー方式（`POST /users/:id/reset-password`）
  ＋ 本人によるパスワード変更（`PATCH /auth/password`）

### 実装予定

- ✅ Cookie Secure/HttpOnly/SameSite フラグ
- TLS終端は本アプリの nginx では行わない。外部のリバースプロキシ
  （Let's Encrypt 証明書を持つホスト側 nginx / Caddy / Traefik 等）が
  前段で HTTPS を終端し、本アプリの nginx へは HTTP で転送する構成を想定。
  `nginx.conf` は `X-Forwarded-Proto` ヘッダーを信頼してそのままバックエンドに
  中継する（ヘッダーが無い場合はローカルの `$scheme` にフォールバック）。

## 開発環境

### 必須

- Docker & Docker Compose
- エディタ: VS Code 推奨（Rust analyzer 拡張）

### オプション

- sqlx-cli（ローカル DB ない場合は docker run で実行）
- cargo（ローカルビルド用、Docker でも可）

### ローカル起動

```bash
# 1. DB 起動
docker compose up -d db

# 2. マイグレーション
docker compose run --rm migrator

# 3. バックエンド起動（ローカル cargo）
cd backend
cargo run

# 4. フロントエンド起動（TBD）
# cd frontend
# npm run dev
```

## テスト戦略

### バックエンド

1. **ユニットテスト**
   - 各モジュール（auth/password.rs など）に `#[cfg(test)]` ブロック
   - `cargo test` で実行

2. **統合テスト**
   - `backend/tests/` ディレクトリで実装
   - 初回・マイグレーション追加時は先にテストDBへ適用:
     `docker compose -f docker-compose.yml -f docker-compose.test.yml build migrator`
     `docker compose -f docker-compose.yml -f docker-compose.test.yml run --rm migrator`
   - `docker compose -f docker-compose.yml -f docker-compose.test.yml up -d`
     でスタックを起動してから `cargo test` を実行（rate limit 緩和 + 専用テストDB
     `test-db` を使うため。開発用の `db` とはデータが混ざらない）

3. **手動テスト**
   - curl で API テスト
   - Postman や REST Client 使用可

### フロントエンド

- TBD（React Testing Library 想定）

## デプロイ

**既定の起動手段は docker compose**（README.mdのクイックスタート参照）。
これに加えて、Google Cloud Run上での運用もサポートしている（詳細手順は
[docs/CLOUD_RUN_DEPLOY.md](docs/CLOUD_RUN_DEPLOY.md)）。DBはマネージドPostgres、
添付ファイルはGCSバケットを使う想定。

### CI/CD（GitHub Actions）

2つのワークフローに分かれている: [.github/workflows/test.yml](.github/workflows/test.yml)
（`make cov`でテスト+カバレッジ計測。全ブランチへのpushで実行される参考用CI）と、
[.github/workflows/deploy.yml](.github/workflows/deploy.yml)（testがmasterブランチで
成功した時だけ`workflow_run`で発火し、Cloud Runへ本番デプロイ。手動承認ステップ
なしの完全自動）。GCP側の初期セットアップ（Workload Identity Federation、
Artifact Registry、Secret Manager等）は[docs/CLOUD_RUN_DEPLOY.md](docs/CLOUD_RUN_DEPLOY.md)の
「CI/CD（GitHub Actions）による自動デプロイ」節を参照。

### 添付ファイルストレージの抽象化（`backend/src/storage/`）

`AttachmentStorage` trait（`exists`/`write`/`read`/`list`/`clear`）が
「添付ファイルの実体がどこにあるか」を抽象化している。`GCS_BUCKET`環境変数が
未設定なら`LocalDiskStorage`（今までのdocker compose用ローカルディスク、
`uploads_data`ボリューム）、設定されていれば`GcsStorage`（Cloud Run向け、
Application Default Credentials経由でGCSバケットに読み書きする）が使われる
（`backend/src/storage/mod.rs`の`build()`）。`backend/src/attachments/`と
`backend/src/backup/`はこのtraitだけを見ており、どちらのバックエンドか
意識しない。docker composeでは`GCS_BUCKET`を設定しないため、挙動は今までと
完全に同じ。

### nginxのAPIプロキシ先（Cloud Run対応）

`nginx.conf`は`BACKEND_UPSTREAM`環境変数で埋められるテンプレートになっている
（`frontend/docker-entrypoint.sh`が`envsubst`で静的な`proxy_pass`先として
展開 - DNS解決はnginx起動時に一度だけで、`resolver`ディレクティブは不要）。
docker composeでは未設定のままなので、今まで通り`http://backend:3000`＋
dockerの埋め込みDNSに解決される。Cloud Runでは`frontend`と`backend`が
それぞれ別のCloud Run URLになるため、`BACKEND_UPSTREAM`に`backend`
サービスのHTTPS URLを設定する。ブラウザからは`frontend`の
URLしか見えず、`backend`へはnginxがサーバーサイドで中継する構成を維持して
いるため、Cookie/CSRF（`SameSite=Lax`前提）は無改修で済んでいる。

なお`proxy_set_header Host`は`$host`（frontend自身のHost）ではなく
`$proxy_host`（backendのHost）を使う。`$host`のままだと、SNIは正しく
backendを指しているのにHostヘッダーはfrontendのままという不一致になり、
Cloud RunのHost/SNIベースのエッジルーティングが混乱して400/404/429/502が
不定期に発生した（実際に踏んだ不具合。詳細は[nginx.conf](nginx.conf)の
コメント参照）。

### 本番環境（docker compose）

1. **イメージビルド**
   ```bash
   docker build -t neport-backend:v1.0 ./backend
   ```

2. **マイグレーション**
   ```bash
   docker run ... sqlx migrate run
   ```

3. **コンテナ起動**
   - `docker compose up -d --build`

### 設定管理

- 環境変数: `.env` ファイル（本番は別途管理）
- シークレット: AWS Secrets Manager など（TBD。Cloud Run利用時はSecret Manager）

### バックアップ / リストア

管理画面（`/admin/backup`）から**GUIで**実行する。デプロイ先（docker-compose /
Cloud Run 等）を問わず同じコードで動くよう、operatorがシェルやdocker CLIで
何かを実行する前提を置かず、**バックエンドアプリケーション自身**が
`pg_dump`/`pg_restore` をサブプロセスとして起動する設計にしている
（`backend/src/backup/`）。以前は `scripts/backup.sh`/`restore.sh`
（`docker compose exec`＋docker volumeマウント前提）で実装していたが、
それだと Cloud Run 環境では動かせない（`docker compose exec` で入れる
コンテナも、共有できる永続ローカルディスクも存在しないため）ことが判明し、
このGUI方式に置き換えた。

- **バックアップ** (`POST /api/v1/backup`, 管理者のみ): バックエンドが自分の
  `DATABASE_URL`（`state.database_url`）を使って `pg_dump -Fc` をサブプロセス
  実行し、自分がマウントしている添付ファイルディレクトリ（`state.upload_dir`）
  の中身と合わせて1つの tar.gz（`tar`/`flate2` クレートでプログラム的に構築）
  にまとめ、ブラウザへのダウンロードとして返す。
- **リストア** (`POST /api/v1/backup/restore`, 管理者のみ): アップロードされた
  tar.gz を展開し、`pg_restore --clean --if-exists` で復元、添付ファイルの中身
  を丸ごと置き換える。**事前にシステム設定でメンテナンスモードを有効にして
  いないと400で拒否される**（一般ユーザーの書き込みがリストア中に混ざらない
  ことを強制するため）。破壊的操作。
- 接続情報は `PGPASSWORD` 環境変数経由でサブプロセスに渡している
  （接続文字列をまるごとコマンドライン引数に渡すと `ps aux` 等でパスワードが
  見えてしまうため）。
- Dockerfile のランタイムステージには `postgresql-client-16`
  （PGDGの公式aptリポジトリ経由。Debian bookworm標準は15系のため）が
  入っている。`db`/`test-db` は `postgres:16` なので、クライアントの
  メジャーバージョンを合わせている。
- Cloud Run など将来的な移行を見据えた設計判断: この方式はバックエンド
  プロセス自身がDB接続とアップロードディレクトリの両方に直接アクセスできる
  ことを利用しているだけなので、DBがCloud SQL（接続文字列経由でそのまま
  使える）に変わっても、添付ファイルが今のところローカルディスクのままで
  ある限りコードを変える必要がない。添付ファイルが将来オブジェクトストレージ
  （GCS等）に移行した場合は、`backend/src/backup/service.rs` の添付ファイル
  部分（ローカルファイルの列挙・書き込み）だけをそのストアのAPI呼び出しに
  差し替えればよい。

## FAQ

### Q: Cargo.lock を git に commit する？
A: Yes（本来は bin crate であれば commit 推奨）。但しビルドの都合で現在は .gitignore に含める。

### Q: sqlx offline mode を使う？
A: 現状未導入。全クエリが実行時チェックの `sqlx::query()`/`query_as::<_, T>()`
スタイルで書かれており、`query!`/`query_as!` マクロによるコンパイル時検証・
`.sqlx` キャッシュは使っていない。

### Q: どのガントチャートライブラリを使う？
A: frappe-gantt から開始。依存線 UX が不足すれば dhtmlx-gantt へ移行。

### Q: ロール管理（admin/general）の実装は？
A: AdminUser extractor で 403 Forbidden を返す。詳細な permission は Phase 2+。

## 参考資料

- [axum ドキュメント](https://docs.rs/axum/)
- [sqlx ドキュメント](https://docs.rs/sqlx/)
- [Argon2 ドキュメント](https://docs.rs/argon2/)
- [PostgreSQL 再帰 CTE](https://www.postgresql.jp/document/current/html/queries-with.html)
