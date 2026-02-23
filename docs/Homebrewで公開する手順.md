# Homebrewで公開する手順

## 1. バイナリのバージョンを更新する

`Cargo.toml` の `package.version` をリリース対象バージョンに更新します。

例: `0.1.0` から `0.1.1` に更新する

```toml
[package]
version = "0.1.1"
```

`Cargo.toml` の更新をコミットして push してから、次の手順に進んでください。

## 2. タグを作成して push する

例: `v0.1.1` タグを作成して push する

```bash
git tag v0.1.1
git push origin v0.1.1
```

※ タグのバージョンは `Cargo.toml` の `version` と同じ値にしてください（例: `0.1.1` → `v0.1.1`）。

## 3. Homebrew tapリポジトリ

<https://github.com/somei-san/homebrew-tools>

## 4. Formulaを生成する

このリポジトリで以下を実行:

```bash
./scripts/homebrew/generate_formula.sh somei-san 0.1.1 ./Formula/cliip-show.rb
```

※ バージョンは `0.1.1` のように `v` なしで指定してください（タグは内部で `v0.1.1` として参照されます）。

生成された `Formula/cliip-show.rb` を [tap リポジトリ](https://github.com/somei-san/homebrew-tools)の `Formula/cliip-show.rb` としてコミットして push してください。

テンプレートは `packaging/homebrew/cliip-show.rb.template` にあります。

## 5. ユーザーのインストール手順

[TapリポジトリのREADME参照](https://github.com/somei-san/homebrew-tools/blob/main/README.md)

## 補足

`cliip-show` はGUI（AppKit）アプリのため、ユーザーログインセッションで動かしてください。
