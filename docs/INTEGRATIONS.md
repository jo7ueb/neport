# REST API・MCP・公開サイト

## REST API

HTTP APIは `/api/v1` 以下にある。チケットの状態は `PATCH /api/v1/tickets/:id` で更新する。ページング対応の一覧では `page` と `per_page` を指定でき、レスポンスは `items`、`total`、`page`、`per_page` を返す。[APIリファレンス（英語）](https://jo7ueb.github.io/neport/en/developer/api/)・[OpenAPI定義](../backend/src/api_docs.rs)・[ページング定義](../backend/src/pagination.rs)

## MCP

`neport-mcp` はNeportのHTTP APIを利用するstdioサーバー。接続先URLとAPIトークンを設定して利用する。英語のパッケージREADMEにインストール、トークン発行、クライアント設定をまとめている。[MCPの利用手順](../mcp/README.md)

リリースのワークフローは `v*` タグのpushまたは手動実行で、MCPのwheel・zipとリポジトリのtar.gzを用意する。[リリース設定](../.github/workflows/release.yml)

## 公開サイトを更新する

Rust、Hugo、Node.js 22以降を用意し、リポジトリ直下で `make site` を実行する。サイト本文は `site/content/ja/` と `site/content/en/` を対応させ、APIのパスや型の変更はOpenAPI定義にも反映する。公開ジョブはmasterへのpushまたは手動実行で動く。[公開サイト](https://jo7ueb.github.io/neport/en/)・[Makefile](../Makefile)・[Pagesワークフロー](../.github/workflows/pages.yml)
