# Neport

Neportは、タスク・工程・稼働をひとつの場所で管理するセルフホスト型のシステムです。複数の案件を抱える個人やチームが、次の仕事を選び、進捗と実績を振り返るために使えます。

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

[ドキュメント目次](docs/README.md)から、試用・設計・開発・運用それぞれの入口を探せます。

## ライセンスとフィードバック

ライセンスは [GNU GPL v3](LICENSE) を参照してください。質問・不具合・改善提案は [GitHub Issues](https://github.com/jo7ueb/neport/issues) にお寄せください。
