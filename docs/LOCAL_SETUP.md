# ローカル環境ガイド

初回起動は[クイックスタート](../README.md#クイックスタート)を参照。Docker ComposeがPostgreSQL、Rust API、Reactを配信するnginxを起動する。フロントエンドのビルドにもDockerを使う。ホストではWeb用の `8880` とDB用の `5432` を使用し、APIの `3000` は通常のCompose構成ではコンテナ側のポート。[Compose設定](../docker-compose.yml)・[フロントエンドのビルド](../frontend/Dockerfile)

## 初期設定

- ComposeのDB接続は `POSTGRES_USER`、`POSTGRES_PASSWORD`、`POSTGRES_DB` から組み立てる。`DATABASE_URL` はAPIをホスト側から直接起動する場合の接続先として使う。
- 管理者の自動作成は、初回管理者の設定が揃い、`users` テーブルが空の場合に行う。パスワードには8文字以上で英字と数字を含める。接続設定やログ・セッションの変数はテンプレートを参照する。[環境変数の例](../.env.example)・[パスワード検証](../backend/src/auth/password.rs)
- 公開環境では外部リバースプロキシでHTTPSを終端する。認証方式や認可の詳細は[アーキテクチャ](ARCHITECTURE.md#認証と可視範囲)を参照。[nginx設定](../nginx.conf)

## 更新・停止

マイグレーションは初回およびスキーマ更新時に適用する。適用するSQLはmigratorイメージに含まれるため、更新時は先に再ビルドする。

```bash
docker compose build migrator
docker compose run --rm migrator
make up
```

`make up` はフロントエンドにコミット情報を含めてビルドする。Composeを直接起動した場合の既定表示は `dev`。停止は `docker compose down`。DBや添付のボリュームは保持される。[Makefile](../Makefile)・[Composeのボリューム](../docker-compose.yml)

## 起動に失敗したとき

```bash
docker compose ps
docker compose logs db backend frontend
curl -f http://localhost:8880/health
```

- DB接続エラーでは、`db` の状態と `.env` の資格情報を確認する。マイグレーションが失敗した場合は、migratorの出力、接続設定、適用対象のSQLを確認する。データを消して作り直す操作が必要な場合は、先に保全対象を確認する。[バックアップと復元](CLOUD_RUN_DEPLOY.md#バックアップと復元composecloud-run共通)
- ポートが競合する場合は、Composeの `ports` にあるホスト側の番号を調整する。APIの `BIND_ADDR` はコンテナ内の待受設定。[Compose設定](../docker-compose.yml)

画面の確認手順と開発時の試験コマンドは個別資料にある。[手動試験](MANUAL_TESTING.md)・[開発の入口](../AGENTS.md)
