export const QUICK_TOOL_KINDS = Object.freeze(["bmsir", "jukebox", "skin", "tables"]);

const definitions = Object.freeze({
  ja: Object.freeze({
    bmsir: Object.freeze({
      title: "BMS-IR 固有設定",
      description: "アカウントと普段使うIR設定をランチャー内で確認する想定です。",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "select", label: "BMS-IR アカウント", help: "ログイン・ログアウト・アカウント変更をここに集約します。", options: ["未接続（モック）", "BMSIR_PLAYER / 190000（例）"]}),
        Object.freeze({type: "toggle", label: "起動前にIR接続を確認", help: "プレイ前にアカウント、プラグイン、Arena接続を確認する想定です。", checked: true}),
        Object.freeze({type: "select", label: "ランキング表示範囲", help: "詳細項目は本体コンフィグへ残し、よく使う選択だけを置きます。", options: ["BMS-IR Standard", "New IR Only", "LR2 Family"]})
      ])
    }),
    jukebox: Object.freeze({
      title: "ジュークボックス",
      description: "プレイ前に使う選曲セットをランチャーから切り替える想定です。",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "select", label: "起動時のジュークボックス", help: "config.json の選択値へ接続する前のUIモックです。", options: ["使用しない", "お気に入り", "最近プレイした曲"]}),
        Object.freeze({type: "toggle", label: "前回の選択を使用", help: "プロファイルごとの保存方法は実装時に決めます。", checked: true})
      ])
    }),
    skin: Object.freeze({
      title: "スキン設定",
      description: "よく切り替えるモードとスキンだけを短い導線で選ぶ想定です。",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "select", label: "対象モード", help: "細かなスキン設定は従来の本体コンフィグに残します。", options: ["選曲", "7KEYS プレイ", "リザルト"]}),
        Object.freeze({type: "select", label: "使用するスキン", help: "インストール済みスキンを読み込む想定のサンプルです。", options: ["Default", "Endless Dream", "BMS-IR Arena"]})
      ])
    }),
    tables: Object.freeze({
      title: "難易度表の管理",
      description: "登録・更新・並び替えを本体コンフィグから分離する想定です。",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "toggle", label: "GENOSIDE 2018", help: "更新確認済み · 7KEYS", checked: true}),
        Object.freeze({type: "toggle", label: "Satellite", help: "更新確認済み · 7KEYS", checked: true}),
        Object.freeze({type: "toggle", label: "Stella", help: "未選択 · 7KEYS", checked: false})
      ])
    })
  }),
  en: Object.freeze({
    bmsir: Object.freeze({
      title: "BMS-IR settings",
      description: "A launcher-level home for account and frequently used IR settings.",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "select", label: "BMS-IR account", help: "Sign in, sign out, and account switching would live here.", options: ["Not connected (mock)", "BMSIR_PLAYER / 190000 (example)"]}),
        Object.freeze({type: "toggle", label: "Check IR connectivity before launch", help: "The final version would check the account, plugin, and Arena connection.", checked: true}),
        Object.freeze({type: "select", label: "Ranking view", help: "Only frequent choices live here; advanced settings stay in game configuration.", options: ["BMS-IR Standard", "New IR Only", "LR2 Family"]})
      ])
    }),
    jukebox: Object.freeze({
      title: "Jukebox",
      description: "Choose the song set to use before launching the game.",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "select", label: "Jukebox on launch", help: "UI mock before this is connected to the config.json value.", options: ["Disabled", "Favorites", "Recently played"]}),
        Object.freeze({type: "toggle", label: "Use the previous choice", help: "Per-profile persistence will be decided during implementation.", checked: true})
      ])
    }),
    skin: Object.freeze({
      title: "Skin settings",
      description: "Choose only the mode and skin that users switch frequently.",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "select", label: "Target mode", help: "Detailed skin configuration remains in the full game configuration.", options: ["Music select", "7KEYS play", "Result"]}),
        Object.freeze({type: "select", label: "Active skin", help: "Example values stand in for installed-skin discovery.", options: ["Default", "Endless Dream", "BMS-IR Arena"]})
      ])
    }),
    tables: Object.freeze({
      title: "Difficulty tables",
      description: "A dedicated place to add, update, and reorder tables.",
      persistent: false,
      fields: Object.freeze([
        Object.freeze({type: "toggle", label: "GENOSIDE 2018", help: "Up to date · 7KEYS", checked: true}),
        Object.freeze({type: "toggle", label: "Satellite", help: "Up to date · 7KEYS", checked: true}),
        Object.freeze({type: "toggle", label: "Stella", help: "Not selected · 7KEYS", checked: false})
      ])
    })
  })
});

export function getQuickToolDefinition(language, kind) {
  const localized = definitions[language] || definitions.ja;
  return localized[kind] || null;
}
