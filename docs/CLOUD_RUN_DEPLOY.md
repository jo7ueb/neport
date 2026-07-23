# Cloud Run へのデプロイ

Neport の**既定のデプロイ・開発手段は引き続き docker compose**（[README.md](../README.md)参照）。
本ドキュメントは、それに加えて Google Cloud Run 上でも動かせるようにするための
追加の選択肢を説明する。DBはマネージドPostgres（Cloud Runに永続ローカルディスク
がないため必須。具体的なサービスは問わない）、添付ファイルはCloud Storage
（GCS）を使う構成を前提とする。

コード側の対応内容は [CLAUDE.md](../CLAUDE.md) の「デプロイ」節を参照。要点だけ書くと:

- 添付ファイルの保存先は `backend/src/storage`（`AttachmentStorage` trait）で
  抽象化されており、`GCS_BUCKET` 環境変数が未設定ならローカルディスク
  （docker composeでの今までの挙動）、設定されていればGCSバケットを使う。
- フロントエンドの `nginx.conf` は環境変数（`BACKEND_UPSTREAM`）でAPIプロキシ先を
  差し替えられるテンプレートになっている。未設定ならdocker composeと全く同じ
  `http://backend:3000`になる。`frontend/docker-entrypoint.sh`がenvsubstで
  静的な`proxy_pass`先に埋め込むため、DNS解決はnginx起動時に一度だけ
  （`resolver`ディレクティブは不要）。
- ブラウザから見えるオリジンは`frontend`のCloud Run URLひとつだけで、
  `backend`へはnginxがサーバーサイドで中継する（Cookie/CSRFのSameSite=Lax
  前提を変えないため）。

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

2つのワークフローに分かれている（テストは全ブランチの参考情報、デプロイは
masterかつテストgreenの場合のみという要件を分離するため）:

1. [.github/workflows/test.yml](../.github/workflows/test.yml)（`Test & coverage`）:
   **全ブランチ**への push で実行される参考用CI（master限定ではない。まだ
   存在しないdevelop等の開発系ブランチも含む）。`make cov`
   （`backend/scripts/run-coverage.sh`）で、ユニットテスト+統合テスト
   (`backend/tests/*.rs`)を実行しつつ`cargo-llvm-cov`でカバレッジを計測。
   レポート(HTML/lcov)はActionsのartifactとして保存される。このワークフロー
   自身はデプロイを一切トリガーしない。
2. [.github/workflows/deploy.yml](../.github/workflows/deploy.yml)（`Deploy to
   Cloud Run`）: 上記`Test & coverage`が**masterブランチで**成功
   (`workflow_run`イベント、`conclusion == 'success'`)した時だけ発火する
   （手動承認ステップなしの完全自動）。backend/frontend/migratorの3イメージを
   ビルドしてArtifact Registryへpush、migratorイメージを本番DBの
   `DATABASE_URL`向けにその場で`docker run`してマイグレーション適用、
   `neport-backend`→`neport-frontend`の順でCloud Runへデプロイ（frontendの
   `BACKEND_UPSTREAM`はbackendデプロイ直後に取得したURLを使う）。
   デプロイするコミットは`workflow_run`イベントの`head_sha`
   （テストされたコミットそのもの）を明示的に使う。frontendイメージの
   ビルドには`--build-arg GIT_COMMIT="${IMAGE_TAG:0:7}"`を渡しており、
   フッターの表示バージョン（下記「フッターのバージョン表示」参照）が
   実際のデプロイ済みコミットと一致するようにしている。

**Docker認証にはWIF impersonationと相性の悪い`gcloud auth
configure-docker`を使わない** - `google-github-actions/auth@v2`ステップに
`token_format: access_token`を指定してアクセストークンを直接取得し、
`docker login -u oauth2accesstoken --password-stdin`でログインしている
（[deploy.yml](../.github/workflows/deploy.yml)参照）。`gcloud auth
configure-docker`が登録する`docker-credential-gcloud`ヘルパーは、トークン
リフレッシュ時にサービスアカウントのGaia IDを引こうとして失敗する
（サービスアカウントはGaia IDを持たないため常に失敗する - 実際に踏んだ
不具合。`docker push`が`ERROR: (gcloud.auth.docker-helper) ... Gaia id not
found for email ***`→`denied: Unauthenticated request ...
artifactregistry.repositories.uploadArtifacts ...`で失敗する形で発現した）。
このヘルパーを経由しない直接ログインなら、WIFでimpersonateした
サービスアカウントの認証情報でも問題なく動く。

GCPへの認証はWorkload Identity Federation（キーを保持しないkeyless方式）を
使う。以下は**一度だけ**手動で行う初期セットアップ（`gcloud`はログイン済みかつ
対象プロジェクトが選択されている前提。このリポジトリの開発環境には認証情報が
ないため、これらのコマンドは利用者自身の端末で実行する）。

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

### 監査ログのIPアドレスとCloud Runの2段プロキシ構成

監査ログ（`/admin/audit-log`、`backend/src/audit/`）にはログイン・ログアウト等の
実行者IPアドレスを記録している。docker-composeでは`nginx.conf`が
`X-Real-IP $remote_addr`をそのままbackendに渡していたが、Cloud Runでは
`frontend`（nginx）自身がGoogleの内部プロキシ（GFE）から接続を受けるため、
`$remote_addr`はブラウザのIPではなくGFE自身のIPになってしまう。

そこで`nginx.conf`に`$true_client_ip`という`map`を追加し、Cloud Runが
`frontend`サービスへの直接のリクエストに対して確実に設定する`X-Forwarded-For`の
先頭エントリ（Google公式ドキュメントで明記）を優先し、それが無いトポロジ
（docker-compose、ローカルの`cargo run`+curl）では`$remote_addr`にフォールバック
するようにした。この値を`X-Real-IP`としてbackendに転送する。

**未検証のまま踏み込んだ判断**: `frontend`（nginx）が`backend`のCloud Run URLを
呼ぶのは「Cloud Run→Cloud Runの2段目の呼び出し」になるが、この2段目でGFEが
`X-Forwarded-For`を（前段の値を維持して）追記するのか、上書きしてしまうのかは
Google公式ドキュメントに明記がなく、断定できなかった。そのため
`backend/src/audit/client_ip.rs`の`ClientIp`抽出は`X-Forwarded-For`ではなく
`X-Real-IP`を優先する設計にしている - `X-Real-IP`は`frontend`のnginxが1段目
（ここは確実に正しい）で確定させて設定する値であり、Cloud Run自身が転送用に
管理するヘッダーではない（GFEが書き換える理由を持つのは`X-Forwarded-For`/
`X-Forwarded-Proto`のような標準の転送ヘッダーのみと考えられる）ため、2段目を
素通りする可能性がより高いという判断。**ただしこれはこの環境にGCP認証情報が
なく実際にCloud Runへデプロイして検証できなかったための推測であり、確証はない。**
デプロイ後、監査ログに記録されたIPが実際に自分のインターネット側IPと一致するかを
確認してほしい。一致しない場合は、`nginx.conf`の`$true_client_ip`マップか、
GFEが2段目で`X-Real-IP`ヘッダー自体を素通ししていない可能性を疑うこと。

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

**注記**: GCS/マネージドPostgres/Cloud Run自体はこのリポジトリの開発環境からは実際に
デプロイ・動作確認ができない（GCP認証情報がないため）。`GcsStorage`の実装は
コンパイルが通ることまでは確認済みだが、実バケットに対する読み書きは
上記手順に従って利用者自身が確認する必要がある。
