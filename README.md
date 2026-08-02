# Neport

タスク管理システム & 稼働管理システム

シンプルなタスク管理と稼働管理・日報機能を統合した内製ツール。Redmine から移行するためのシステム。

## 技術スタック

- **バックエンド**: Rust + axum + sqlx + PostgreSQL
- **フロントエンド**: React + Vite + TypeScript + TanStack Query + Tailwind CSS
- **インフラ**: Docker + Docker Compose + nginx（TLS終端は行わない。外部の
  リバースプロキシ（Let's Encrypt等）を前段に置く前提）

## クイックスタート

### 前提条件

- Docker & Docker Compose
- ポート 8880, 3000, 5432 が利用可能

### 起動

```bash
# リポジトリクローン
git clone <このリポジトリのURL> neport
cd neport

# マイグレーション実行（初回のみ）
docker compose run --rm migrator

# コンテナ起動（frontendもDockerfileでビルドされる）
docker compose up -d --build

# 確認
curl http://localhost:8880/health
curl http://localhost:3000/health
```

上記のように`docker compose`を直接叩くと、フッターに表示されるバージョンは
（コミットハッシュを渡していないため）常に`dev`になる。実際のコミット
ハッシュを表示させたい場合は、代わりに`make up`（初回のみ`make migrate`）を
使う（[Makefile](Makefile)が`git rev-parse --short HEAD`を自動で
`--build-arg GIT_COMMIT`として渡す）。

### 停止

```bash
docker compose down
```

## Cloud Run へのデプロイ

上記の docker compose が既定の起動手段だが、Google Cloud Run（DB: マネージド
Postgres、添付ファイル: GCS）上での運用もサポートしている。手順は
[docs/CLOUD_RUN_DEPLOY.md](docs/CLOUD_RUN_DEPLOY.md) を参照。

## プロジェクトサイトとAPIドキュメント

GitHub Pagesのプロジェクトサイトは、`master`へのpushで自動生成・公開されます。

- 日本語サイト: `https://jo7ueb.github.io/neport/ja/`
- 英語サイト: `https://jo7ueb.github.io/neport/en/`
- APIドキュメント: 各言語サイトの「開発者向け情報」から `/developer/api/` へ移動

ローカルでサイトを生成するには、Rust、Hugo、Node.js 22以降を用意してから次を実行します。

```bash
make site
```

サイト本文は `site/content/ja/` と `site/content/en/` を対応させて追加してください。
APIのルートや型を変更した場合は、バックエンドのOpenAPI定義も同じ変更に合わせます。

## 環境変数

`.env` ファイルで設定（開発環境用デフォルト値あり）:

```
DATABASE_URL=postgres://neport:password@db:5432/neport
RUST_LOG=info
APP_ENV=development
BIND_ADDR=0.0.0.0:3000
SESSION_COOKIE_NAME=neport_session
SESSION_TTL_SECONDS=86400
```

## API エンドポイント（抜粋）

**ページネーション**: プロジェクト一覧・チケット一覧・ユーザー一覧・コメント
一覧・日報履歴・チケット活動ログの各一覧系エンドポイントは `?page=&per_page=`
（`per_page` は 10/25/50/100 が既定のUI選択肢、内部上限500）に対応し、
`{ items, total, page, per_page }` 形式で返す。

### 認証

- `POST /api/v1/auth/login` - ログイン（`/login` のみ送信元IPごとにレート制限）
- `POST /api/v1/auth/logout` - ログアウト（セッション破棄・Cookieクリア）
- `GET /api/v1/auth/me` - 現在のユーザー取得
- `PATCH /api/v1/auth/password` - 自分のパスワード変更（現在のパスワード必須。
  成功すると変更元以外の既存セッションは全て無効化される）
- `PATCH /api/v1/auth/locale` - 自分の表示言語変更（`ja`/`en`。`users.locale` に保存され、
  次回以降のログインにも引き継がれる。既定値は `ja`）

パスワードは新規作成・変更いずれも8文字以上かつ英字・数字を両方含む必要がある。

### ユーザー管理

- `GET /api/v1/users` - ユーザー一覧
- `POST /api/v1/users` - ユーザー作成
- `POST /api/v1/users/:id/suspend` - ユーザー停止
- `POST /api/v1/users/:id/resume` - ユーザー再開
- `POST /api/v1/users/:id/reset-password` - パスワードリセット（管理者限定。
  発行された一時パスワードはレスポンスで一度だけ返され、対象ユーザーの
  既存セッションは全て無効化される）
- `PATCH /api/v1/users/:id/role` - ロール変更（管理者限定、自分自身は不可）
- `DELETE /api/v1/users/:id` - ユーザー削除
- `GET /api/v1/users/:id/activity?date=` - 指定日の活動ログ

**CSRF**: `X-CSRF-Token` ヘッダーをログイン時に発行されたトークンと一致させないと、
GET以外のリクエストは全て403で拒否される（フロントエンドは自動的に付与する）。

### プロジェクト・チケット

- `/api/v1/projects` - プロジェクトCRUD
- `/api/v1/projects/:id/tickets`, `/api/v1/projects/:id/tickets/tree` - チケット一覧・ツリー
- `/api/v1/tickets` - チケットCRUD（無制限ネスト、循環検出）
- `/api/v1/tickets/:id/dependencies` - 依存関係（循環検出）
- `/api/v1/tickets/:id/comments` - コメント
- `/api/v1/tickets/:id/move` - かんばんカード移動
- `/api/v1/tickets/:id/activity` - チケット活動ログ

### マイルストーン・かんばん・ガント

- `/api/v1/projects/:id/milestones` - マイルストーンCRUD
- `/api/v1/projects/:id/kanban/columns` - かんばんカラムCRUD
- `/api/v1/projects/:id/gantt` - ガント集約データ（チケット+依存関係）

### 作業時間・日報

- `/api/v1/work-sessions/{start,end,current,today,monthly}` - 作業開始・終了
  （中抜けはデータとして持たず、終了→再開始の繰り返しで表現）
- `/api/v1/daily-reports`, `/api/v1/daily-reports/today/tickets`, `/api/v1/daily-reports/:date`

### ヘルスチェック

- `GET /health` - サーバー状態確認

## テスト

### 自動テスト（バックエンド）

`backend/tests/` に reqwest ベースの統合テスト（80件以上）が揃っている。
バックエンドを起動した状態で実行する:

```bash
cd backend
TEST_BASE_URL=http://localhost:3000 cargo test
```

### 手動テスト（フロントエンド）

`docs/MANUAL_TESTING.md` を参照。

### 動作確認用 curl（参考）

#### ユーザー作成

```bash
curl -X POST http://localhost:3000/api/v1/users \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "secure_password_123",
    "display_name": "テスト ユーザー"
  }'
```

#### ログイン

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "secure_password_123"
  }'
```

#### ユーザー一覧取得

```bash
curl http://localhost:3000/api/v1/users
```

## 主な機能

### バックエンド

- 認証（login/logout、Argon2idパスワードハッシュ、CSRF、ログインのレート制限）
- ユーザー管理（CRUD、suspend/resume/delete、ロール変更、管理者によるパスワードリセット）
- プロジェクト管理
- チケット CRUD（無制限ネスト・循環検出）、依存関係（循環検出）、コメント
- マイルストーン、かんばんボード（カラムCRUD・カード移動）
- 活動ログ（日報用の自動記録・読み取りAPI）
- ガントチャート集約エンドポイント（チケット + 依存関係）
- 作業時間管理（開始・終了、中抜けはデータとして持たず終了→再開始で表現、
  日付をまたぐ作業にも対応、月次集計）
- 日報（ALKPT、チケット自動収集、無効化状態は保存せず読み取り時に毎回再計算）

### フロントエンド

- ログイン・セッション復元・ログアウト
- プロジェクト一覧・作成
- チケット一覧（フィルタ・締切超過ハイライト）・詳細（子チケット・コメント・依存関係・マイルストーン紐付け）
- かんばんボード（@dnd-kit によるドラッグ&ドロップ）
- マイルストーン管理
- 管理者ユーザー管理（ロール選択付きの作成フォーム、バックエンドの `AdminUser` によるロールガード、自己操作防止）
- ガントチャート（frappe-gantt）
- WBS（作業分解構造）表（ツリー表示・セルクリック編集・インデント/アウトデントによる親子変更・子タスク追加）
- 作業時間管理（作業開始・作業終了の2ボタンのみ、月次一覧、スマホ向け大ボタン）
- 多言語対応（i18next + react-i18next。英語をベースに `frontend/src/i18n/locales/*.json`
  で翻訳を差し替え。既定言語は `ja`、アカウント設定画面から `en` に切替可能。選択言語は
  `users.locale` に保存され次回以降のログインにも引き継がれる）
- 日報（ALKPT、自動収集チケット表示）
- 手動テスト手順は `docs/MANUAL_TESTING.md` を参照

## ディレクトリ構成

```
rehearsal/
├── docker-compose.yml      # コンテナオーケストレーション
├── nginx.conf              # nginx設定
├── .env                    # 環境変数（開発用）
├── .env.example            # 環境変数テンプレート
├── backend/                # Rust バックエンド
│   ├── Cargo.toml
│   ├── Dockerfile
│   ├── entrypoint.sh
│   ├── migrations/         # sqlx マイグレーション
│   └── src/
│       ├── main.rs
│       ├── auth/           # 認証モジュール
│       ├── users/          # ユーザー管理
│       └── ...
├── frontend/               # React フロントエンド
│   ├── Dockerfile          # nginxイメージへのマルチステージビルド
│   ├── src/
│   └── dist/               # ビルド成果物（Docker/npm run build 生成）
├── docs/                   # 開発メモ・手動テスト手順
└── README.md
```

## トラブルシューティング

### データベース接続エラー

```bash
# PostgreSQL コンテナが起動しているか確認
docker compose ps

# ログ確認
docker compose logs db
```

### マイグレーション失敗

```bash
# DB をリセット（危険: データが失われます）
docker compose down -v
docker compose up -d db
docker compose run --rm migrator
```

### ポート衝突

`.env` で `BIND_ADDR` を変更してポートを変更可能。

## パスワード要件

- 最小 8 文字
- Argon2id でハッシュ化（ソルト内蔵）

## セキュリティ

- パスワードは Argon2id でハッシュ化
- セッションはサーバー側で管理（Cookie: HttpOnly/Secure/SameSite=Lax 済み）
- CSRF トークンを検証（GET以外の全リクエストで `X-CSRF-Token` を照合）
- ログインエンドポイントは送信元IPごとにレート制限（ブルートフォース対策）
- HTTPS は本アプリの nginx では終端しない。外部のリバースプロキシ
  （Let's Encrypt 等）を前段に置く前提

## ライセンス

内製ツール（ライセンス未定）

## サポート

質問・バグ報告は [GitHub Issues](https://github.com/anveino/neport/issues) へ（準備中）
