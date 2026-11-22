# Neport ドキュメント目次

| 知りたいこと | 最初に読む | 必要に応じて読む |
| --- | --- | --- |
| Neportを試したい | [クイックスタート](../README.md#クイックスタート) | [ローカル環境ガイド](LOCAL_SETUP.md) — 初期設定、更新、起動トラブル |
| Neportの目的と今後の方針を知りたい | [プロダクト方針](PRODUCT.md) | [要求台帳](REQUIREMENTS.md) — 合意済みの要求、現状、オーナー判断 |
| 次に取り組む作業を探したい | [GitHubの進捗チェックリスト](https://github.com/jo7ueb/neport/issues/2) | [ロードマップ](ROADMAP.md) — 優先順位と着手・完了条件 |
| 現行コードの構成や改修箇所を知りたい | [アーキテクチャ](ARCHITECTURE.md) | [技術引き継ぎ](ENGINEERING_HANDOFF.md) — 調査済みの箇所、リスク、未検証の項目 |
| 開発・動作確認をしたい | [開発の入口](../AGENTS.md) | [手動試験書](MANUAL_TESTING.md) — ブラウザでの確認項目 |
| デプロイ・復元をしたい | [Cloud Runとバックアップ](CLOUD_RUN_DEPLOY.md) | [ローカル環境ガイド](LOCAL_SETUP.md) — Docker Composeでの運用 |
| REST APIやMCPを使いたい | [外部連携ガイド](INTEGRATIONS.md) | [MCP利用手順](../mcp/README.md) |

## 設計判断と仕様案

- [ADR 0001　チケット進捗の一元管理](adr/0001-ticket-progress.md) — 合意済みの方針と、その適用範囲。
- [横断カンバンと個人の注目リスト](specs/boards-and-focus.md) — 仕様案。表示・更新・権限の受け入れ例。
- [作業タイマーと工数訂正](specs/work-timer.md) — 仕様案。切替・重複防止・締め解除の受け入れ例。

## 履歴資料

- [初期の要求メモ](../requirements.md) — 開発当初の背景と要求。
- [Phase 1の進捗記録](PHASE1_PROGRESS.md) — 当時の実装と動作確認。
