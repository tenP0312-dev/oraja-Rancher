const invoke = window.__TAURI__?.core?.invoke;
const openDialog = window.__TAURI__?.dialog?.open;
const dictionary = {
  ja: {
    title: "Arena Launcher",
    lead: "署名を検証して、安全に更新・設定・起動します。",
    installation: "本体",
    rootLabel: "BMS-IR Arena oraja フォルダ",
    javaLabel: "Java 17（自動検出できない場合）",
    browse: "参照",
    inspect: "確認",
    useJava: "使用",
    actions: "起動",
    configure: "設定を開く",
    play: "ゲームを起動",
    javaHint: "同梱Java 17を優先し、見つからない場合は指定されたJava 17を使用します。",
    update: "オフライン更新",
    updateText: "更新は内蔵Ed25519公開鍵と全ファイルのSHA-256が一致した場合だけ適用されます。失敗時はバックアップから復元します。",
    stagingLabel: "更新ファイルのフォルダ",
    manifestLabel: "署名済みmanifest.json",
    inspectUpdate: "内容を確認",
    applyUpdate: "検証して適用",
    selfUpdate: "ランチャーを更新",
    signing: "正式配布版はWindows AuthenticodeとmacOS Developer ID公証済みである必要があります。",
    ini: "INI設定",
    iniHint: "未知の項目・コメント・順序を保ったまま、指定した項目だけ更新します。",
    iniKey: "SECTION.key",
    iniValue: "value",
    save: "保存",
    ready: "本体とJava 17を確認しました。",
    missingJava: "本体は見つかりましたが、Java 17を指定してください。",
    invalid: "本体フォルダを確認できませんでした。",
    javaReady: "Java 17を確認しました。",
    updateDone: "更新を適用しました",
    selfUpdateStarted: "ランチャーを終了し、安全な自己更新を開始します。",
    iniDone: "INIを保存しました。"
  },
  en: {
    title: "Arena Launcher",
    lead: "Verify signatures, then update, configure, and launch safely.",
    installation: "Installation",
    rootLabel: "BMS-IR Arena oraja folder",
    javaLabel: "Java 17 (when automatic detection fails)",
    browse: "Browse",
    inspect: "Inspect",
    useJava: "Use",
    actions: "Launch",
    configure: "Open configuration",
    play: "Launch game",
    javaHint: "Bundled Java 17 is preferred; otherwise select a Java 17 executable.",
    update: "Offline update",
    updateText: "Updates are applied only after the built-in Ed25519 key and every SHA-256 match. A failed install is rolled back.",
    stagingLabel: "Update files folder",
    manifestLabel: "Signed manifest.json",
    inspectUpdate: "Review",
    applyUpdate: "Verify and apply",
    selfUpdate: "Update launcher",
    signing: "Official releases require Windows Authenticode and notarized macOS Developer ID signatures.",
    ini: "INI settings",
    iniHint: "Update only the requested value while preserving unknown keys, comments, and ordering.",
    iniKey: "SECTION.key",
    iniValue: "value",
    save: "Save",
    ready: "The game and Java 17 are ready.",
    missingJava: "The game was found, but Java 17 must be selected.",
    invalid: "The installation folder could not be verified.",
    javaReady: "Java 17 verified.",
    updateDone: "Update applied",
    selfUpdateStarted: "The launcher will exit and begin its verified self-update.",
    iniDone: "INI saved."
  }
};

let language = "ja";
let installation = null;

const byId = id => document.getElementById(id);
const applyLanguage = () => {
  document.documentElement.lang = language;
  document.querySelectorAll("[data-i18n]").forEach(element => {
    element.textContent = dictionary[language][element.dataset.i18n];
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach(element => {
    element.placeholder = dictionary[language][element.dataset.i18nPlaceholder];
  });
  byId("language").textContent = language === "ja" ? "EN" : "日本語";
};
const setLaunchEnabled = enabled => {
  byId("play").disabled = !enabled;
  byId("configure").disabled = !enabled;
  byId("apply-update").disabled = !installation
    || !byId("staging").value
    || !byId("manifest").value;
  byId("inspect-update").disabled = !byId("manifest").value;
  byId("self-update").disabled = !byId("staging").value
    || !byId("manifest").value;
};
const choose = async (id, options) => {
  if (!openDialog) return;
  const selected = await openDialog({...options, multiple: false});
  if (typeof selected === "string") {
    byId(id).value = selected;
    setLaunchEnabled(Boolean(installation?.java_runtime));
  }
};

byId("language").addEventListener("click", () => {
  language = language === "ja" ? "en" : "ja";
  applyLanguage();
});
byId("browse-root").addEventListener("click", () =>
  choose("root", {directory: true})
);
byId("browse-staging").addEventListener("click", () =>
  choose("staging", {directory: true})
);
byId("browse-java").addEventListener("click", () =>
  choose("java", {directory: false})
);
byId("browse-manifest").addEventListener("click", () =>
  choose("manifest", {directory: false, filters: [{name: "JSON", extensions: ["json"]}]})
);
byId("browse-ini").addEventListener("click", () =>
  choose("ini-path", {directory: false, filters: [{name: "INI", extensions: ["ini"]}]})
);

byId("inspect").addEventListener("click", async () => {
  const status = byId("installation-status");
  try {
    if (!invoke) throw new Error("Tauri unavailable");
    installation = await invoke("inspect_installation", {path: byId("root").value});
    if (!installation.game_jar) throw new Error("game missing");
    if (installation.java_runtime) {
      byId("java").value = installation.java_runtime;
      status.textContent = dictionary[language].ready;
      setLaunchEnabled(true);
    } else {
      status.textContent = dictionary[language].missingJava;
      setLaunchEnabled(false);
    }
  } catch (error) {
    installation = null;
    status.textContent = `${dictionary[language].invalid} ${error}`;
    setLaunchEnabled(false);
  }
});

byId("use-java").addEventListener("click", async () => {
  try {
    await invoke("inspect_java", {path: byId("java").value});
    if (!installation) throw new Error("inspect the game folder first");
    installation.java_runtime = byId("java").value;
    installation.java_source = "manual";
    installation.java_version = 17;
    byId("installation-status").textContent = dictionary[language].javaReady;
    setLaunchEnabled(true);
  } catch (error) {
    byId("installation-status").textContent = String(error);
    setLaunchEnabled(false);
  }
});

const launch = async configuration => {
  if (!installation || !installation.java_runtime) return;
  await invoke("launch_game", {
    root: installation.root,
    java: installation.java_runtime,
    gameJar: installation.game_jar,
    configuration
  });
};
byId("play").addEventListener("click", () => launch(false));
byId("configure").addEventListener("click", () => launch(true));

["staging", "manifest"].forEach(id =>
  byId(id).addEventListener("input", () =>
    setLaunchEnabled(Boolean(installation?.java_runtime))
  )
);

const renderSafeMarkdown = markdown => {
  const target = byId("release-notes");
  target.replaceChildren();
  let list = null;
  String(markdown || "").split(/\r?\n/).forEach(raw => {
    const line = raw.trim();
    if (!line) {
      list = null;
      return;
    }
    let element;
    if (line.startsWith("### ")) {
      element = document.createElement("h4");
      element.textContent = line.slice(4);
      list = null;
    } else if (line.startsWith("## ")) {
      element = document.createElement("h3");
      element.textContent = line.slice(3);
      list = null;
    } else if (line.startsWith("# ")) {
      element = document.createElement("h2");
      element.textContent = line.slice(2);
      list = null;
    } else if (/^[-*]\s+/.test(line)) {
      if (!list) {
        list = document.createElement("ul");
        target.append(list);
      }
      element = document.createElement("li");
      element.textContent = line.replace(/^[-*]\s+/, "");
      list.append(element);
      return;
    } else {
      element = document.createElement("p");
      element.textContent = line;
      list = null;
    }
    target.append(element);
  });
};

byId("inspect-update").addEventListener("click", async () => {
  const status = byId("update-status");
  try {
    const release = await invoke("inspect_update_manifest", {
      manifestPath: byId("manifest").value
    });
    renderSafeMarkdown(release.release_notes_markdown);
    status.textContent = `${release.version} (${release.channel})`;
  } catch (error) {
    renderSafeMarkdown("");
    status.textContent = String(error);
  }
});

byId("apply-update").addEventListener("click", async () => {
  const status = byId("update-status");
  try {
    const release = await invoke("apply_offline_update", {
      root: installation.root,
      staging: byId("staging").value,
      manifestPath: byId("manifest").value
    });
    renderSafeMarkdown(release.release_notes_markdown);
    status.textContent = `${dictionary[language].updateDone}: ${release.version}`;
    await byId("inspect").click();
  } catch (error) {
    status.textContent = String(error);
  }
});

byId("self-update").addEventListener("click", async () => {
  const status = byId("update-status");
  try {
    status.textContent = dictionary[language].selfUpdateStarted;
    await invoke("begin_self_update", {
      staging: byId("staging").value,
      manifestPath: byId("manifest").value
    });
  } catch (error) {
    status.textContent = String(error);
  }
});

byId("save-ini").addEventListener("click", async () => {
  const status = byId("ini-status");
  const key = byId("ini-key").value.trim();
  try {
    if (!key) throw new Error("key is required");
    await invoke("update_ini", {
      path: byId("ini-path").value,
      updates: {[key]: byId("ini-value").value}
    });
    status.textContent = dictionary[language].iniDone;
  } catch (error) {
    status.textContent = String(error);
  }
});

applyLanguage();
setLaunchEnabled(false);
