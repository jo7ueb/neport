# Neport

Neportは、タスク・工程・稼働をひとつの場所で管理するセルフホスト型のシステムです。Redmineからの移行をきっかけに生まれ、個人の研究・教育・校務から研究室やチームでの業務管理までを想定しています。

- **仕事を分解し、工程を見渡す**。親子チケットでWBSを作り、依存関係をガントチャートで確認できます。節目の管理にはマイルストーンを使えます。
- **チームで進捗を共有する**。案件ごとのワークフローをカンバンで運用し、担当・締切・完了確認をチケットに残せます。
- **稼働と日報を記録する**。開始・終了の打刻を月次で集計し、その日に操作したチケットを日報に自動収集します。

## クイックスタート

DockerとDocker Composeが必要です。

```bash
git clone https://github.com/jo7ueb/neport.git
cd neport
cp .env.example .env
```

`.env` の `POSTGRES_PASSWORD` を設定し、初回管理者の `ADMIN_EMAIL`、`ADMIN_PASSWORD`、`ADMIN_DISPLAY_NAME` を設定してください。

```bash
docker compose run --rm migrator
docker compose up -d --build
```

<http://localhost:8880> を開き、初回管理者でログインします。設定の詳細や起動に失敗した場合の確認先は[ローカル環境ガイド](docs/LOCAL_SETUP.md)にあります。

## ドキュメント

| 目的 | 参照先 |
| --- | --- |
| 起動・運用 | [ローカル環境](docs/LOCAL_SETUP.md)・[Cloud Runとバックアップ](docs/CLOUD_RUN_DEPLOY.md) |
| API・外部連携 | [REST APIとMCP](docs/INTEGRATIONS.md) |
| 開発・テスト | [開発の入口](AGENTS.md)・[現行アーキテクチャ](docs/ARCHITECTURE.md) |
| プロダクトの今後 | [目的と原則](docs/PRODUCT.md)・[要求と未決事項](docs/REQUIREMENTS.md)・[ロードマップ](docs/ROADMAP.md) |

## ライセンスとフィードバック

ライセンスは [GNU GPL v3](LICENSE) を参照してください。質問・不具合・改善提案は [GitHub Issues](https://github.com/jo7ueb/neport/issues) にお寄せください。
