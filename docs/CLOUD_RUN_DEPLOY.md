# Cloud Run へのデプロイ

Cloud RunではマネージドPostgreSQLと添付用のCloud Storageバケットを使う。フロントエンドのnginxが同一オリジンのAPIリクエストをバックエンドへ転送する。[アーキテクチャ](ARCHITECTURE.md#現行システム)

## 前提条件

- `gcloud` CLI（ログイン済み、対象プロジェクトが選択済み）
- GCPプロジェクトで以下のAPIを有効化: Cloud Run, Artifact Registry, Cloud Storage
- 手動で`docker push`する自分のgcloudユーザーアカウントに`roles/artifactregistry.writer`
  が付与済み（Artifact Registryのリポジトリ自体は存在するのに
  `denied: Permission "artifactregistry.repositories.uploadArtifacts" ...`
  で拒否される場合はこれが不足している）:
  ```bash
  gcloud projects add-iam-policy-binding "$(gcloud config get-value project)" \
    --member="user:$(gcloud config get-value account)" \
    --role=roles/artifactregistry.writer
  ```
- マネージドPostgresのデータベースを作成済み（接続文字列を控えておく。
  TLS接続が必須なら`?sslmode=require`を付与すること）
- 添付ファイル用のGCSバケットを作成済み（例: `gsutil mb gs://<bucket-name>`）
- バックエンドの実行時サービスアカウントに、そのバケットへの
  `roles/storage.objectAdmin`を付与済み（Cloud Runのデフォルトサービス
  アカウントで動かす場合はそのアカウントに付与する）

## CI/CD（GitHub Actions）による自動デプロイ

- すべてのブランチのpushで `Test & coverage` が `make cov` を実行し、結果を成果物に保存する。[テストのワークフロー](../.github/workflows/test.yml)
- `master` のテストが成功すると、Cloud Runのデプロイが自動で始まる。テスト対象のコミットから3イメージをビルドし、DB移行、バックエンド、フロントエンドの順に反映する。[デプロイのワークフロー](../.github/workflows/deploy.yml)

自動デプロイの初期設定にはWorkload Identity Federationを使用する。ワークフローは発行されたアクセストークンでArtifact Registryにログインする。

### 1. Workload Identity Federation + デプロイ用サービスアカウント

```bash
export PROJECT_ID=<your-gcp-project-id>
export PROJECT_NUMBER=$(gcloud projects describe "${PROJECT_ID}" --format='value(projectNumber)')
export GITHUB_REPO=jo7ueb/neport

gcloud services enable iamcredentials.googleapis.com sts.googleapis.com \
  run.googleapis.com artifactregistry.googleapis.com secretmanager.googleapis.com \
  storage.googleapis.com --project "${PROJECT_ID}"

# デプロイ専用のサービスアカウント（Cloud Runの実行時サービスアカウントとは別物）
gcloud iam service-accounts create github-actions-deployer \
  --project "${PROJECT_ID}" \
  --display-name "GitHub Actions deployer"

# Cloud Run へのデプロイ・イメージのpush・デプロイ時に実行時サービス
# アカウントとしてact-asする権限
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member "serviceAccount:github-actions-deployer@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role roles/run.admin
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member "serviceAccount:github-actions-deployer@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role roles/artifactregistry.writer
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member "serviceAccount:github-actions-deployer@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role roles/iam.serviceAccountUser

# Workload Identity Pool + Provider（このGitHubリポジトリからのOIDCトークンのみ信頼）
gcloud iam workload-identity-pools create github \
  --project "${PROJECT_ID}" --location global \
  --display-name "GitHub Actions"

gcloud iam workload-identity-pools providers create-oidc github-provider \
  --project "${PROJECT_ID}" --location global \
  --workload-identity-pool github \
  --display-name "GitHub Actions provider" \
  --attribute-mapping "google.subject=assertion.sub,attribute.repository=assertion.repository" \
  --attribute-condition "assertion.repository=='${GITHUB_REPO}'" \
  --issuer-uri "https://token.actions.githubusercontent.com"

# 上記プールを通じて、対象リポジトリのGitHub Actionsだけがこのサービス
# アカウントをimpersonateできるようにする
gcloud iam service-accounts add-iam-policy-binding \
  "github-actions-deployer@${PROJECT_ID}.iam.gserviceaccount.com" \
  --project "${PROJECT_ID}" \
  --role roles/iam.workloadIdentityUser \
  --member "principalSet://iam.googleapis.com/projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/github/attribute.repository/${GITHUB_REPO}"

# ワークフローに設定するプロバイダのフルリソース名を控えておく
gcloud iam workload-identity-pools providers describe github-provider \
  --project "${PROJECT_ID}" --location global \
  --workload-identity-pool github --format 'value(name)'
```

### 2. Artifact Registry・GCSバケット・Secret Manager

```bash
export REGION=asia-northeast1

gcloud artifacts repositories create neport \
  --project "${PROJECT_ID}" --location "${REGION}" --repository-format docker

gsutil mb -l "${REGION}" "gs://<bucket-name>"

# Cloud Runの実行時サービスアカウント（デフォルトのCompute Engineサービス
# アカウントを使う場合。デプロイ用のgithub-actions-deployerとは別物）
export RUNTIME_SA="${PROJECT_NUMBER}-compute@developer.gserviceaccount.com"
gsutil iam ch "serviceAccount:${RUNTIME_SA}:roles/storage.objectAdmin" "gs://<bucket-name>"

printf '%s' "postgres://<user>:<password>@<DBホスト>/<db>?sslmode=require" | \
  gcloud secrets create neport-database-url --project "${PROJECT_ID}" --data-file=-
printf '%s' "admin@example.com" | \
  gcloud secrets create neport-admin-email --project "${PROJECT_ID}" --data-file=-
printf '%s' "<初回admin用の強いパスワード>" | \
  gcloud secrets create neport-admin-password --project "${PROJECT_ID}" --data-file=-
printf '%s' "管理者" | \
  gcloud secrets create neport-admin-display-name --project "${PROJECT_ID}" --data-file=-

for secret in neport-database-url neport-admin-email neport-admin-password neport-admin-display-name; do
  gcloud secrets add-iam-policy-binding "${secret}" \
    --project "${PROJECT_ID}" \
    --member "serviceAccount:${RUNTIME_SA}" \
    --role roles/secretmanager.secretAccessor
done
```

`ADMIN_*`の3つのシークレットは初回ブートストラップ（usersテーブルが空の時だけ
効く）専用なので、初回デプロイ後は`gcloud run services update neport-backend
--remove-secrets ADMIN_EMAIL,ADMIN_PASSWORD,ADMIN_DISPLAY_NAME`で外して構わない。

### 3. GitHub側の設定

リポジトリの Settings → Secrets and variables → Actions で以下を設定する
（`gh` CLIでも可: `gh secret set NAME`/`gh variable set NAME --body VALUE`）。

**Secrets**（機密情報）:
| 名前 | 値 |
|---|---|
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | 上記手順1最後の`gcloud ... describe ... --format 'value(name)'`の出力 |
| `GCP_SERVICE_ACCOUNT` | `github-actions-deployer@<PROJECT_ID>.iam.gserviceaccount.com` |
| `PROD_DATABASE_URL` | 本番DBの接続文字列（TLS接続が必須なら`?sslmode=require`付き）。マイグレーション実行にのみ使う |

**Variables**（非機密。省略時のデフォルトは`deploy.yml`参照）:
| 名前 | 値 |
|---|---|
| `GCP_PROJECT_ID` | GCPプロジェクトID |
| `GCP_REGION` | 省略可（既定`asia-northeast1`） |
| `GCP_ARTIFACT_REPO` | 省略可（既定`neport`） |
| `GCS_BUCKET` | 添付ファイル用GCSバケット名 |

設定後は`master`へpushするだけで、テスト→カバレッジ→（green なら）デプロイまで
自動で走る。以降の手動デプロイ手順（1〜4）は初回のGCP側動作確認や
トラブルシュート用に残してある。

## 1. イメージのビルド・push（手動デプロイ手順・参考）

`docker push`する前に、DockerにArtifact Registry向けの認証情報を一度だけ
設定しておく必要がある（これを忘れると`denied: Unauthenticated request`で
pushが拒否される）:

```bash
export REGION=asia-northeast1
export REPO=neport   # Artifact Registryのリポジトリ名（事前に作成しておく）
export PROJECT=<your-gcp-project-id>

gcloud auth configure-docker "${REGION}-docker.pkg.dev"

docker build -t "${REGION}-docker.pkg.dev/${PROJECT}/${REPO}/backend:latest" ./backend
docker build --build-arg GIT_COMMIT="$(git rev-parse --short HEAD)" \
  -t "${REGION}-docker.pkg.dev/${PROJECT}/${REPO}/frontend:latest" -f frontend/Dockerfile .

docker push "${REGION}-docker.pkg.dev/${PROJECT}/${REPO}/backend:latest"
docker push "${REGION}-docker.pkg.dev/${PROJECT}/${REPO}/frontend:latest"
```

（この手順はユーザー自身の実在するGoogleアカウントで`gcloud auth
login`済みであることが前提なので、`gcloud auth configure-docker`で問題
ない。CI/CDのWFI impersonationではこのヘルパーが機能しない件は上記
「CI/CD（GitHub Actions）による自動デプロイ」節を参照）

`frontend`のビルドコンテキストがリポジトリルート（`.`）である点に注意
（`nginx.conf`をDockerfileから参照するため。詳細はコード内コメント参照）。

### フッターのバージョン表示

フロントエンドの画面下部フッターには、ビルド時にDockerイメージへ焼き込んだ
コミットハッシュを表示する（[Layout.tsx](../frontend/src/components/Layout.tsx)、
`import.meta.env.VITE_GIT_COMMIT`）。`--build-arg GIT_COMMIT=...`を渡さずに
ビルドすると`frontend/Dockerfile`の既定値`dev`のまま表示される（上記の
手動ビルド手順のように明示的に渡すか、`docker compose`経由のローカル開発では
[Makefile](../Makefile)の`make build`/`make up`が`git rev-parse --short
HEAD`を自動で渡す）。CI/CD（`deploy.yml`）は`workflow_run`イベントの
`head_sha`から同様に渡している。

## 2. マイグレーション

Cloud Run側に何かをデプロイする必要はない。スキーマ変更時のみ手元から実行すれば
十分な頻度なので、docker compose用の`migrator`イメージをビルドだけ`docker
compose`で行い、実行は素の`docker run`で行う（`docker compose run migrator`は
使わない - `docker-compose.yml`の`migrator`サービスは`depends_on: db`を持つため、
`docker compose run`だとローカル開発用の`db`コンテナまで余計に起動してしまう。
`docker run`はcomposeの依存関係を見ないので、本番DBだけに単独で接続できる）:

```bash
# 1行目はサービス名 `migrator` を指定してビルドするだけ（起動はしない）。
docker compose build migrator

# 2行目はビルドされた「イメージ」を素の docker run で直接実行する。
# イメージ名は `<dockerディレクトリ名>-<サービス名>` = `neport-migrator`
# になる（サービス名の `migrator` とは別物なので注意）。
docker run --rm \
  -e DATABASE_URL="postgres://<user>:<password>@<DBホスト>/<db>?sslmode=require" \
  neport-migrator
```

## 3. `backend`サービスのデプロイ

```bash
gcloud run deploy neport-backend \
  --image "${REGION}-docker.pkg.dev/${PROJECT}/${REPO}/backend:latest" \
  --region "${REGION}" \
  --allow-unauthenticated \
  --set-env-vars "APP_ENV=production,SESSION_COOKIE_NAME=neport_session,SESSION_TTL_SECONDS=86400,GCS_BUCKET=<bucket-name>" \
  --set-secrets "DATABASE_URL=neport-database-url:latest,ADMIN_EMAIL=neport-admin-email:latest,ADMIN_PASSWORD=neport-admin-password:latest,ADMIN_DISPLAY_NAME=neport-admin-display-name:latest"
```

- `--allow-unauthenticated`にしているのは、docker compose構成でも`backend`は
  ネットワーク層の認証を持たず、セッション/CSRF/ログインのレート制限という
  アプリケーション層の防御だけで守られているため（[auth/extractors.rs](../backend/src/auth/extractors.rs)）。
  Cloud Run移行でもこの前提は変えない。ネットワーク層でさらに絞りたい場合は
  Serverless VPCコネクタ経由の内部限定公開も可能だが、追加のインフラ・コストが
  必要になるため、今回は導入しない。
- `ADMIN_*`はusersテーブルが空の場合のみ効く初回ブートストラップ用なので、
  初回デプロイ後は環境変数から外して構わない（[main.rs](../backend/src/main.rs)参照）。
- `PORT`はCloud Runが自動的に注入し、バックエンドはそれを見て自動的に
  bindする（`BIND_ADDR`を明示的に設定しない限り）。
- `DATABASE_URL`はマネージドPostgresの接続文字列にする（TLS接続が必須なら
  `?sslmode=require`を含める）。

## 4. `frontend`サービスのデプロイ

まず上記でデプロイした`backend`サービスのURLを控える
（`gcloud run services describe neport-backend --region "${REGION}" --format 'value(status.url)'`）。

```bash
gcloud run deploy neport-frontend \
  --image "${REGION}-docker.pkg.dev/${PROJECT}/${REPO}/frontend:latest" \
  --region "${REGION}" \
  --allow-unauthenticated \
  --set-env-vars "BACKEND_UPSTREAM=https://neport-backend-xxxxxxxx-an.a.run.app"
```

- `BACKEND_UPSTREAM`は手順3で控えた`backend`のURL（スキームを含むフルURL）。
  `frontend/docker-entrypoint.sh`がenvsubstでこれを`nginx.conf`の静的な
  `proxy_pass`先に埋め込む（DNS解決はnginx起動時に一度だけなので
  `resolver`ディレクティブは不要 - 実際に試行錯誤した経緯は
  [nginx.conf](../nginx.conf)のコメント参照）。

### 監査ログのIPアドレス

nginxが `X-Real-IP` へ値を設定し、バックエンドの監査ログがこれを優先する。デプロイ後に監査ログの記録を接続元IPと照合する。異なる場合は各プロキシが転送するヘッダーを確認する。[nginx設定](../nginx.conf)・[IP抽出](../backend/src/audit/client_ip.rs)

## 5. カスタムドメイン（任意）

`neport-frontend-xxxxxxxx.a.run.app`ではなく独自ドメイン（例: `neport.example.com`）
でアクセスできるようにする場合、Cloud Runの「ドメインマッピング」機能を使う。
**単純にCNAMEでCloud RunのURLを指すだけでは404になる** - Cloud RunはHost/SNIで
どのサービスにルーティングするかを判断しており、ドメインマッピングとして
明示的に登録されていないホスト名は認識されない。

```bash
gcloud beta run domain-mappings create \
  --service=neport-frontend \
  --domain=neport.example.com \
  --region="${REGION}"
```

（`beta`が必要 - `gcloud run domain-mappings`だけだと`--region`が
「unrecognized arguments」で弾かれるバージョンがある。また、事前にそのドメインの
所有権をSearch Consoleで確認しておく必要がある。未確認だと確認用URLが
案内される）

出力される`CNAME`レコード（`ghs.googlehosted.com.`宛）をDNSに設定する:

```bash
gcloud beta run domain-mappings describe \
  --domain=neport.example.com --region="${REGION}" \
  --format="yaml(status.resourceRecords)"
```

**DNSプロバイダがCloudflare等のCDN/プロキシサービスの場合、そのレコードは
必ず「DNS only」（Cloudflareでいうグレークラウド。オレンジクラウド＝
「Proxied」にしない）にすること。** プロキシを経由すると、CDN側がTLSを
横取りしてしまい、Googleが証明書を発行・検証できず「Waiting for certificate
provisioning」から進まなくなる（実際に踏んだ不具合）。DNS変更後は下記で
進捗を確認できる:

```bash
gcloud beta run domain-mappings describe \
  --domain=neport.example.com --region="${REGION}" \
  --format="value(status.conditions[].message)"
```

証明書発行には数分〜（まれに）最大24時間程度かかることがある。ポーリング
間隔が5分→1時間と伸びていくのは正常な指数バックオフで、悪化しているわけ
ではない。長時間進まない場合は、ドメインの`CAA`レコード
（`dig <ドメイン> CAA`）が証明書発行元を制限していないかも確認する
（空＝制限なしで問題ない）。

## 動作確認

```bash
curl https://<neport-frontendのURL>/health
```

ブラウザで`https://<neport-frontendのURL>/`にアクセスし、ログイン・チケット作成・
添付ファイルのアップロード/ダウンロードが問題なく動作することを確認する。

## バックアップと復元（Compose・Cloud Run共通）

管理画面の「バックアップ」から実行する。バックエンドが `pg_dump` を起動し、DBと添付ストレージを1つのアーカイブにまとめる。復元は事前にメンテナンスモードを有効にし、バックアップの内容でDBと添付を置き換える。復元確認には専用の環境を使う。[バックアップ処理](../backend/src/backup/handlers.rs)・[サブプロセスと接続情報の扱い](../backend/src/backup/service.rs)・[PostgreSQLクライアント](../backend/Dockerfile)
