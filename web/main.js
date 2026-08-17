import {getQuickToolDefinition} from "./quick-tools-mock.mjs";

const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;

const dictionary = {
  ja: {
    checking: "更新を確認しています",
    current: "すべて最新です",
    allCurrentDescription: "本体、ランチャー、プラグインは最新の状態です",
    updatesAvailable: "更新があります",
    unavailable: "更新を確認できませんでした。現在の本体は起動できます",
    unavailableNoInstall: "セットアップファイルを取得できませんでした",
    invalid: "本体JAR、Java 21以上、またはArenaプラグインを確認できません",
    notInstalled: "本体はまだインストールされていません",
    ready: "本体、Java、Arenaプラグインを確認しました",
    play: "Arenaを起動",
    configure: "本体設定",
    check: "更新を確認",
    updateAll: "すべて更新",
    updateBody: "本体を更新",
    updateLauncher: "ランチャーを更新",
    updatePlugin: "プラグインを更新",
    updateLaunch: "更新して起動",
    installLaunch: "ダウンロードして起動",
    launchCurrent: "現在の版を起動",
    available: "BMS-IR Arena {version} が利用できます",
    installAvailable: "BMS-IR Arena {version} をセットアップできます",
    publishedAt: "配信日時: {datetime}",
    mandatory: "この更新は必須です",
    revoked: "現在の版は停止されています。更新が必要です",
    launcherOld: "ランチャーの更新が必要です",
    updating: "更新ファイルを検証して適用しています",
    installing: "本体ファイルを検証してセットアップしています",
    details: "詳細",
    information: "アップデート・イベント情報",
    noInformation: "現在のお知らせはありません",
    policyInvalid: "保存された更新判定を検証できません。オンライン更新確認が必要です",
    updateUnconfigured: "このランチャーには更新先が設定されていません",
    switchLanguage: "英語に切り替え",
    notInstalledVersion: "未インストール",
    downloading: "本体をダウンロード中",
    extracting: "本体を展開・検証中",
    verifying: "ダウンロードしたファイルを検証中",
    applying: "更新を適用中",
    restarting: "新しいランチャーを起動中",
    progressFiles: "{done} / {total} ファイル",
    launching: "Arenaを起動しています",
    launchFailed: "Arenaが異常終了しました",
    launchEndedEarly: "Arenaがすぐに終了しました",
    launchDiagnostic: "Arenaの起動診断を確認してください",
    exitCode: "終了コード",
    exitCodeUnavailable: "取得できません",
    launchLog: "診断ログ",
    deprecatedToggle: "非推奨版をダウンロード",
    deprecatedHide: "非推奨版一覧を閉じる",
    deprecatedLoading: "読み込んでいます…",
    deprecatedEmpty: "非推奨版はありません",
    deprecatedLoadError: "非推奨版の一覧を取得できませんでした",
    downgradeButton: "ダウングレード",
    downgradeConfirm: "本体を {version}（配信日時: {datetime}）にダウングレードします。Java、プラグイン、設定、スキン、リプレイ、スコアデータは変更されません。よろしいですか？",
    downgradeSuccess: "ダウングレードが完了しました",
    deprecatedKindTest: "(旧テストビルド)",
    deprecatedKindStable: "(旧安定版)",
    releaseNotesToggle: "リリースノートを見る",
    releaseNotesOpen: "更新内容を見る",
    currentRelease: "現在の本体 {version}",
    noReleaseNotes: "この版のリリースノートはありません",
    statusBody: "本体",
    statusLauncher: "ランチャー",
    statusPlugin: "プラグイン",
    statusChecking: "確認中",
    statusUpToDate: "最新",
    statusSetupNeeded: "セットアップ必要",
    statusUpdateAvailable: "更新あり",
    statusMandatory: "必須更新",
    statusRevoked: "利用停止中",
    statusLauncherTooOld: "ランチャー要更新",
    statusUnavailable: "確認できません",
    pluginCheckUnavailable: "プラグインの更新を確認できません",
    pluginToggle: "Arenaプラグインの更新・旧版",
    pluginCurrent: "プラグイン更新あり: {version}",
    pluginCurrentOk: "プラグインは最新です",
    pluginInstall: "このプラグインを適用",
    pluginDeprecated: "旧プラグイン版",
    pluginReleaseVersion: "プラグイン {plugin} / 本体リリース {release}",
    subtitle: "プレイ・更新・管理",
    close: "閉じる",
    accountMenu: "アカウント",
    launchEyebrow: "PLAY / UPDATE",
    launchTitle: "Arenaをはじめる",
    primaryActionsLabel: "主な操作",
    playHint: "準備済みの本体ですぐにプレイ",
    updateAllHint: "必要なコンポーネントをまとめて更新",
    configureHint: "詳細なゲームコンフィグを開く",
    bmsirSettings: "BMS-IR設定",
    bmsirSettingsHint: "アカウントとIR固有設定",
    quickToolsEyebrow: "QUICK SETTINGS",
    quickToolsTitle: "よく使う設定",
    jukeboxTitle: "ジュークボックス",
    jukeboxHint: "選曲セットを切り替える",
    skinTitle: "スキン",
    skinHint: "使用スキンを選ぶ",
    tablesTitle: "難易度表",
    tablesHint: "テーブルを追加・整理する",
    mockFootnote: "この段階では画面構成だけのモックです。config.jsonへの書き込みや設定保存は行いません。",
    informationEyebrow: "EVENT LOG",
    systemEyebrow: "SYSTEM",
    systemTitle: "バージョン状態",
    latestRelease: "最新リリース",
    advancedEyebrow: "ADVANCED",
    advancedTitle: "詳細管理",
    quickToolMockNotice: "操作感を確認するためのモックです。入力内容は送信・保存されず、config.jsonも変更しません。",
    mockDone: "閉じる",
    residentOn: "常駐 ON",
    residentOff: "常駐 OFF",
    settingsOpen: "設定",
    settingsTitle: "ランチャー設定",
    settingsDescription: "変更はすぐに保存されます",
    settingsResident: "常駐（トレイ）",
    settingsResidentHint: "Arena起動後やウィンドウを閉じた後も常駐します",
    settingsBackgroundCheck: "バックグラウンド更新確認",
    settingsBackgroundCheckHint: "1日1回確認し、トレイに更新を表示します",
    settingsAutostart: "ログイン時に起動",
    settingsAutostartHint: "常駐がONのときに利用できます",
    settingsSaveError: "設定を保存できませんでした",
    deprecatedWarning: "非推奨版はサポート対象外です。本体だけを切り替え、設定やスコアは変更しません。"
  },
  en: {
    checking: "Checking for updates",
    current: "Everything is up to date",
    allCurrentDescription: "The game, launcher, and plugin are all up to date",
    updatesAvailable: "Updates are available",
    unavailable: "Could not check for updates. The installed version can still launch",
    unavailableNoInstall: "Could not download the setup information",
    invalid: "The game JAR, Java 21+, or Arena plugin could not be found",
    notInstalled: "The game is not installed yet",
    ready: "Game, Java, and Arena plugin are ready",
    play: "Launch Arena",
    configure: "Game settings",
    check: "Check for updates",
    updateAll: "Update all",
    updateBody: "Update game",
    updateLauncher: "Update launcher",
    updatePlugin: "Update plugin",
    updateLaunch: "Update and launch",
    installLaunch: "Download and launch",
    launchCurrent: "Launch installed version",
    available: "BMS-IR Arena {version} is available",
    installAvailable: "BMS-IR Arena {version} is ready to install",
    publishedAt: "Published: {datetime}",
    mandatory: "This update is required",
    revoked: "The installed version has been revoked and must be updated",
    launcherOld: "The launcher must be updated",
    updating: "Verifying and applying the update",
    installing: "Verifying and installing the game files",
    details: "Details",
    information: "Updates and event information",
    noInformation: "There are no current announcements",
    policyInvalid: "The saved update policy is invalid. An online update check is required",
    updateUnconfigured: "This launcher has no update endpoint configured",
    switchLanguage: "Switch to Japanese",
    notInstalledVersion: "Not installed",
    downloading: "Downloading game files",
    extracting: "Extracting and verifying game files",
    verifying: "Verifying downloaded files",
    applying: "Applying update",
    restarting: "Starting the updated launcher",
    progressFiles: "{done} / {total} files",
    launching: "Launching Arena",
    launchFailed: "Arena exited with an error",
    launchEndedEarly: "Arena exited shortly after launch",
    launchDiagnostic: "Check the Arena launch diagnostic",
    exitCode: "Exit code",
    exitCodeUnavailable: "Unavailable",
    launchLog: "Diagnostic log",
    deprecatedToggle: "Download a deprecated version",
    deprecatedHide: "Hide deprecated versions",
    deprecatedLoading: "Loading…",
    deprecatedEmpty: "No deprecated versions are available",
    deprecatedLoadError: "Could not load the deprecated version list",
    downgradeButton: "Downgrade",
    downgradeConfirm: "Downgrade the game to {version} (published {datetime}). Java, the plugin, settings, skins, replays, and score data will not be touched. Continue?",
    downgradeSuccess: "Downgrade complete",
    deprecatedKindTest: "(older test build)",
    deprecatedKindStable: "(older stable build)",
    releaseNotesToggle: "View release notes",
    releaseNotesOpen: "View update details",
    currentRelease: "Installed body {version}",
    noReleaseNotes: "No release notes are available for this version",
    statusBody: "Body",
    statusLauncher: "Launcher",
    statusPlugin: "Plugin",
    statusChecking: "Checking",
    statusUpToDate: "Up to date",
    statusSetupNeeded: "Setup needed",
    statusUpdateAvailable: "Update available",
    statusMandatory: "Required update",
    statusRevoked: "Revoked",
    statusLauncherTooOld: "Launcher needs updating",
    statusUnavailable: "Unavailable",
    pluginCheckUnavailable: "Could not check for plugin updates",
    pluginToggle: "Arena plugin updates and older versions",
    pluginCurrent: "Plugin update available: {version}",
    pluginCurrentOk: "The plugin is up to date",
    pluginInstall: "Apply this plugin",
    pluginDeprecated: "Older plugin release",
    pluginReleaseVersion: "Plugin {plugin} / body release {release}",
    subtitle: "Play, update, and manage",
    close: "Close",
    accountMenu: "Account",
    launchEyebrow: "PLAY / UPDATE",
    launchTitle: "Start Arena",
    primaryActionsLabel: "Primary actions",
    playHint: "Launch the verified installed body",
    updateAllHint: "Update every component that needs it",
    configureHint: "Open the full game configuration",
    bmsirSettings: "BMS-IR settings",
    bmsirSettingsHint: "Account and IR-specific settings",
    quickToolsEyebrow: "QUICK SETTINGS",
    quickToolsTitle: "Frequently used settings",
    jukeboxTitle: "Jukebox",
    jukeboxHint: "Choose a song set",
    skinTitle: "Skin",
    skinHint: "Select the active skin",
    tablesTitle: "Difficulty tables",
    tablesHint: "Add and organize tables",
    mockFootnote: "This phase is a layout mock only. It does not write config.json or save settings.",
    informationEyebrow: "EVENT LOG",
    systemEyebrow: "SYSTEM",
    systemTitle: "Version status",
    latestRelease: "Latest release",
    advancedEyebrow: "ADVANCED",
    advancedTitle: "Advanced management",
    quickToolMockNotice: "This interaction mock does not send or save input and never changes config.json.",
    mockDone: "Close",
    residentOn: "Resident ON",
    residentOff: "Resident OFF",
    settingsOpen: "Settings",
    settingsTitle: "Launcher settings",
    settingsDescription: "Changes are saved immediately",
    settingsResident: "Tray residency",
    settingsResidentHint: "Keep running after Arena launches or the window closes",
    settingsBackgroundCheck: "Background update checks",
    settingsBackgroundCheckHint: "Check daily and show updates in the tray",
    settingsAutostart: "Launch at login",
    settingsAutostartHint: "Available while tray residency is enabled",
    settingsSaveError: "Could not save settings",
    deprecatedWarning: "Deprecated versions are unsupported. Only the game body changes; settings and scores are preserved."
  }
};

let language = localStorage.getItem("bmsir-launcher-language") === "en" ? "en" : "ja";
let state = null;
let update = null;
let checking = true;
let updateUnavailable = false;
let installingUpdate = false;
let updateProgress = null;
let launching = false;
let latestLaunchExit = null;
let deprecatedVersions = null;
let deprecatedVisible = false;
let deprecatedLoading = false;
let downgradingVersion = null;
let pluginVisible = false;
let pluginVersions = null;
let pluginStatus = null;
let pluginUnavailable = false;
let launcherSettings = null;
let activeQuickTool = null;
const deprecatedNotesCache = {};
const byId = id => document.getElementById(id);
const tr = key => dictionary[language][key];

function applyLanguage() {
  document.documentElement.lang = language;
  document.querySelectorAll("[data-i18n]").forEach(element => {
    element.textContent = tr(element.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach(element => {
    element.setAttribute("aria-label", tr(element.dataset.i18nAriaLabel));
    element.title = tr(element.dataset.i18nAriaLabel);
  });
  byId("language-label").textContent = language === "ja" ? "日本語" : "English";
  byId("language").setAttribute("aria-label", tr("switchLanguage"));
  byId("language").title = tr("switchLanguage");
  renderUpdate();
  renderProgress();
  renderDeprecated();
  renderPlugins();
  renderSettings();
  renderQuickToolDialog();
}

function renderSettings() {
  const resident = Boolean(launcherSettings?.resident);
  byId("resident-label").textContent = tr(resident ? "residentOn" : "residentOff");
  byId("resident-state").setAttribute("aria-pressed", String(resident));
  byId("resident-state").dataset.enabled = String(resident);
  byId("setting-resident").checked = resident;
  byId("setting-background-check").checked = Boolean(launcherSettings?.background_check);
  byId("setting-autostart").checked = Boolean(launcherSettings?.autostart);
  byId("setting-autostart").disabled = !resident;
}

async function loadSettings() {
  try {
    launcherSettings = await invoke("get_launcher_settings");
  } catch (error) {
    launcherSettings = {resident: false, autostart: false, background_check: false};
    showError(error);
  }
  renderSettings();
}

async function saveSetting(key, value) {
  if (!launcherSettings) return;
  launcherSettings[key] = value;
  if (key === "resident" && !value) launcherSettings.autostart = false;
  renderSettings();
  try {
    launcherSettings = await invoke("set_launcher_settings", {settings: launcherSettings});
    hideError();
  } catch (error) {
    showError(`${tr("settingsSaveError")}\n${error}`);
    await loadSettings();
  }
  renderSettings();
}

function openDialog(dialog) {
  if (!dialog.open) dialog.showModal();
}

function renderQuickToolDialog() {
  if (!activeQuickTool) return;
  const definition = getQuickToolDefinition(language, activeQuickTool);
  if (!definition || definition.persistent) return;
  byId("quick-tool-dialog-title").textContent = definition.title;
  byId("quick-tool-dialog-description").textContent = definition.description;
  const fields = byId("quick-tool-fields");
  fields.replaceChildren();
  definition.fields.forEach((field, index) => {
    const row = document.createElement("label");
    if (field.type === "select") {
      row.className = "mock-field";
      const label = document.createElement("span");
      label.textContent = field.label;
      const help = document.createElement("small");
      help.textContent = field.help;
      const select = document.createElement("select");
      select.setAttribute("aria-label", field.label);
      field.options.forEach(option => {
        const element = document.createElement("option");
        element.textContent = option;
        select.append(element);
      });
      row.append(label, help, select);
    } else {
      row.className = "mock-toggle";
      const copy = document.createElement("span");
      const label = document.createElement("span");
      label.textContent = field.label;
      const help = document.createElement("small");
      help.textContent = field.help;
      copy.append(label, help);
      const input = document.createElement("input");
      input.type = "checkbox";
      input.checked = Boolean(field.checked);
      input.setAttribute("aria-label", field.label);
      row.append(copy, input);
    }
    row.dataset.mockField = `${activeQuickTool}-${index}`;
    fields.append(row);
  });
}

function openQuickTool(kind) {
  const definition = getQuickToolDefinition(language, kind);
  if (!definition || definition.persistent) return;
  activeQuickTool = kind;
  renderQuickToolDialog();
  openDialog(byId("quick-tool-dialog"));
}

function closeQuickTool() {
  byId("quick-tool-dialog").close();
  activeQuickTool = null;
  byId("quick-tool-fields").replaceChildren();
}

function renderPlugins() {
  const toggle = byId("plugin-toggle");
  const container = byId("plugin-list-container");
  if (!toggle || !container) return;
  toggle.setAttribute("aria-expanded", String(pluginVisible));
  container.hidden = !pluginVisible;
  if (!pluginVisible) return;
  const pluginUpdate = pluginStatus?.update_available ? pluginStatus.available : null;
  byId("plugin-current").textContent = pluginUnavailable
    ? tr("pluginCheckUnavailable")
    : pluginUpdate
      ? tr("pluginCurrent").replace("{version}", pluginVersionLabel(pluginUpdate))
      : tr("pluginCurrentOk");
  const list = byId("plugin-list");
  list.replaceChildren();
  if (pluginUpdate) {
    appendPluginRelease(list, pluginUpdate, false);
  }
  (pluginVersions || []).forEach(entry => {
    appendPluginRelease(list, entry, true);
  });
}

function pluginVersionLabel(entry) {
  const filename = String(entry?.artifact_path || "").split("/").pop() || "";
  const versionMatch = filename.match(/_(\d+(?:\.\d+)+(?:[-+][A-Za-z0-9.-]+)?)\.jar$/i);
  return versionMatch ? versionMatch[1] : filename || String(entry?.version || "");
}

function appendPluginRelease(list, entry, deprecated) {
  const item = document.createElement("li");
  const row = document.createElement("div");
  row.className = "deprecated-row";
  const label = document.createElement("span");
  const publishedAt = formatPublishedAt(entry.published_at) || entry.published_at;
  const releaseLabel = tr("pluginReleaseVersion")
    .replace("{plugin}", pluginVersionLabel(entry))
    .replace("{release}", entry.version);
  label.textContent = `${releaseLabel} · ${publishedAt}${deprecated ? ` (${tr("pluginDeprecated")})` : ""}`;
  const button = document.createElement("button");
  button.type = "button";
  button.className = deprecated ? "quiet" : "primary";
  button.textContent = tr("pluginInstall");
  button.disabled = installingUpdate;
  button.addEventListener("click", () => installPlugin(entry.version));
  row.append(label, button);

  const details = document.createElement("details");
  const summary = document.createElement("summary");
  summary.textContent = tr("releaseNotesToggle");
  const notes = document.createElement("div");
  notes.className = "notes";
  details.append(summary, notes);
  details.addEventListener("toggle", () => {
    if (details.open) loadDeprecatedNotes(entry.version, notes);
  });
  item.append(row, details);
  list.append(item);
}

async function togglePlugins() {
  pluginVisible = !pluginVisible;
  if (pluginVisible && pluginVersions === null) {
    try {
      [pluginStatus, pluginVersions] = await Promise.all([
        pluginStatus ? Promise.resolve(pluginStatus) : invoke("check_plugin_update"),
        invoke("list_deprecated_plugin_versions")
      ]);
      pluginUnavailable = false;
    } catch (error) { showError(error); pluginVersions = []; }
  }
  renderPlugins();
}

async function installPlugin(version) {
  installingUpdate = true; updateProgress = {phase:"downloading", bytes_done:0, bytes_total:0, files_done:0, files_total:1};
  renderUpdate(); renderProgress();
  try {
    await invoke("install_plugin_version", {version});
    [pluginStatus, pluginVersions] = await Promise.all([
      invoke("check_plugin_update"), invoke("list_deprecated_plugin_versions")
    ]);
    pluginUnavailable = false;
    hideError();
  } catch (error) { showError(error); }
  finally { installingUpdate = false; updateProgress = null; renderProgress(); renderPlugins(); renderUpdate(); }
}

function setStatus(text, kind = "neutral") {
  byId("update-status").textContent = text;
  byId("status-mark").dataset.kind = kind;
}

function formatPublishedAt(value) {
  if (!value) return "";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return "";
  return new Intl.DateTimeFormat(language === "ja" ? "ja-JP" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(parsed);
}

function formatBytes(bytes) {
  const value = Number(bytes) || 0;
  if (value >= 1024 * 1024 * 1024) return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  if (value >= 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${value} B`;
}

function renderProgress() {
  const container = byId("update-progress");
  if (!container || !installingUpdate || !updateProgress) {
    if (container) container.hidden = true;
    return;
  }
  const phase = dictionary[language][updateProgress.phase] ? updateProgress.phase : "downloading";
  const bytesDone = Number(updateProgress.bytes_done) || 0;
  const bytesTotal = Number(updateProgress.bytes_total) || 0;
  const filesDone = Number(updateProgress.files_done) || 0;
  const filesTotal = Number(updateProgress.files_total) || 0;
  const percent = bytesTotal > 0
    ? Math.min(100, Math.floor(bytesDone * 100 / bytesTotal))
    : (phase === "downloading" ? 0 : 100);
  const files = tr("progressFiles")
    .replace("{done}", filesDone)
    .replace("{total}", filesTotal);
  byId("progress-label").textContent = tr(phase);
  byId("progress-percent").textContent = `${percent}%`;
  byId("progress-bar").value = percent;
  byId("progress-bar").textContent = `${percent}%`;
  byId("progress-detail").textContent = `${formatBytes(bytesDone)} / ${formatBytes(bytesTotal)}  ·  ${files}`;
  container.hidden = false;
}

function deprecatedVersionKind() {
  // Every entry in the deprecated list is, by construction, older than the
  // channel's currently published version (list_deprecated_versions_from
  // excludes both the installed and the published version). Labeling each
  // one by channel makes it unambiguous at a glance that it is an older
  // build, not a newer one under test.
  return state?.channel === "test" ? tr("deprecatedKindTest") : tr("deprecatedKindStable");
}

function renderDeprecated() {
  const toggle = byId("deprecated-toggle");
  const container = byId("deprecated-list-container");
  const loading = byId("deprecated-loading");
  const empty = byId("deprecated-empty");
  const list = byId("deprecated-list");
  toggle.textContent = tr(deprecatedVisible ? "deprecatedHide" : "deprecatedToggle");
  toggle.setAttribute("aria-expanded", String(deprecatedVisible));
  toggle.disabled = checking || installingUpdate;
  container.hidden = !deprecatedVisible;
  if (!deprecatedVisible) return;
  loading.hidden = !deprecatedLoading;
  list.replaceChildren();
  if (deprecatedLoading) {
    empty.hidden = true;
    return;
  }
  const versions = Array.isArray(deprecatedVersions) ? deprecatedVersions : [];
  empty.hidden = versions.length > 0;
  versions.forEach(entry => {
    const item = document.createElement("li");
    const row = document.createElement("div");
    row.className = "deprecated-row";
    const label = document.createElement("span");
    const publishedAt = formatPublishedAt(entry.published_at);
    const versionLabel = `${entry.version} ${deprecatedVersionKind()}`;
    label.textContent = publishedAt ? `${versionLabel}  ·  ${publishedAt}` : versionLabel;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "quiet";
    button.textContent = downgradingVersion === entry.version ? tr("applying") : tr("downgradeButton");
    button.disabled = Boolean(downgradingVersion) || installingUpdate || checking;
    button.addEventListener("click", () => confirmAndDowngrade(entry));
    row.append(label, button);

    const details = document.createElement("details");
    const summary = document.createElement("summary");
    summary.textContent = tr("releaseNotesToggle");
    const notes = document.createElement("div");
    notes.className = "notes";
    details.append(summary, notes);
    details.addEventListener("toggle", () => {
      if (details.open) loadDeprecatedNotes(entry.version, notes);
    });

    item.append(row, details);
    list.append(item);
  });
}

async function loadDeprecatedNotes(version, container) {
  const cached = deprecatedNotesCache[version];
  if (cached) {
    renderSafeMarkdown(pickLocalizedNotes(cached), container);
    return;
  }
  container.textContent = tr("deprecatedLoading");
  try {
    const notes = await invoke("fetch_deprecated_version_notes", {version});
    deprecatedNotesCache[version] = notes;
    renderSafeMarkdown(pickLocalizedNotes(notes), container);
  } catch (error) {
    container.textContent = tr("deprecatedLoadError");
  }
}

async function toggleDeprecated() {
  deprecatedVisible = !deprecatedVisible;
  if (deprecatedVisible && deprecatedVersions === null && !deprecatedLoading) {
    deprecatedLoading = true;
    renderDeprecated();
    try {
      deprecatedVersions = await invoke("list_deprecated_versions");
      hideError();
    } catch (error) {
      deprecatedVersions = [];
      showError(error);
      setStatus(tr("deprecatedLoadError"), "error");
    } finally {
      deprecatedLoading = false;
      renderDeprecated();
    }
    return;
  }
  renderDeprecated();
}

function confirmAndDowngrade(entry) {
  const datetime = formatPublishedAt(entry.published_at) || entry.published_at || "";
  const message = tr("downgradeConfirm")
    .replace("{version}", entry.version)
    .replace("{datetime}", datetime);
  if (!window.confirm(message)) return;
  downgrade(entry.version);
}

async function downgrade(version) {
  downgradingVersion = version;
  installingUpdate = true;
  updateProgress = {phase: "downloading", bytes_done: 0, bytes_total: 0, files_done: 0, files_total: 1};
  renderProgress();
  renderDeprecated();
  renderUpdate();
  setStatus(tr("applying"), "available");
  try {
    await invoke("downgrade_to_version", {version});
    hideError();
    deprecatedVersions = null;
    deprecatedVisible = false;
    await loadState();
    if (state) await checkUpdate();
  } catch (error) {
    showError(error);
  } finally {
    downgradingVersion = null;
    installingUpdate = false;
    updateProgress = null;
    renderProgress();
    renderDeprecated();
    renderUpdate();
  }
}

function renderSafeMarkdown(markdown, target = byId("release-notes")) {
  target.replaceChildren();
  if (!String(markdown || "").trim()) {
    const empty = document.createElement("p");
    empty.textContent = tr("noReleaseNotes");
    target.append(empty);
    return;
  }
  let list = null;
  String(markdown || "").split(/\r?\n/).forEach(raw => {
    const line = raw.trim();
    if (!line) {
      list = null;
      return;
    }
    if (/^[-*]\s+/.test(line)) {
      if (!list) {
        list = document.createElement("ul");
        target.append(list);
      }
      const item = document.createElement("li");
      item.textContent = line.replace(/^[-*]\s+/, "");
      list.append(item);
      return;
    }
    const element = document.createElement(line.startsWith("#") ? "h3" : "p");
    element.textContent = line.replace(/^#{1,3}\s+/, "");
    target.append(element);
  });
}

function pickLocalizedNotes(source) {
  if (!source) return "";
  const localized = language === "ja"
    ? source.release_notes_markdown_ja
    : source.release_notes_markdown_en;
  return localized || source.release_notes_markdown || "";
}

function canLaunch() {
  return Boolean(state?.installation_ready)
    && !state?.cached_policy_invalid
    && !checking
    && !installingUpdate
    && !launching
    && !updateBlocksLaunch();
}

function updateBlocksLaunch() {
  if (!update) return false;
  return update.status === "install_required"
    || update.status === "revoked"
    || update.status === "launcher_too_old"
    || ((update.body_update_available || update.launcher_update_available) && update.mandatory);
}

function localizedReleaseNotes() {
  return pickLocalizedNotes(update);
}

function renderAnnouncements() {
  const target = byId("announcements");
  target.replaceChildren();
  const announcements = Array.isArray(update?.announcements) ? update.announcements : [];
  if (!announcements.length) {
    const empty = document.createElement("li");
    empty.className = "empty-information";
    empty.textContent = tr("noInformation");
    target.append(empty);
    return;
  }
  announcements.forEach(announcement => {
    const item = document.createElement("li");
    const date = document.createElement("time");
    date.dateTime = announcement.date;
    date.textContent = announcement.date.replaceAll("-", ".");
    const title = document.createElement("span");
    title.textContent = language === "ja" ? announcement.title_ja : announcement.title_en;
    item.append(date, title);
    target.append(item);
  });
}

function setBadge(element, kind, text) {
  element.textContent = text;
  element.dataset.kind = kind;
}

function installedPluginVersion() {
  const path = pluginStatus?.installed_artifact_path
    || state?.installation?.plugin_jars?.[0]
    || "";
  return path ? pluginVersionLabel({artifact_path: path}) : tr("notInstalledVersion");
}

function renderStatusCards() {
  if (!state) return;
  const installed = Boolean(state.installation_ready);
  const installedVersion = installed ? state.installed_version : tr("notInstalledVersion");
  const bodyLine = byId("body-version-line");
  const bodyBadge = byId("body-badge");
  const bodyUpdate = Boolean(update?.body_update_available);
  const launcherUpdate = Boolean(update?.launcher_update_available);
  const pluginUpdate = Boolean(pluginStatus?.update_available);

  if (checking) {
    bodyLine.textContent = installedVersion;
    setBadge(bodyBadge, "neutral", tr("statusChecking"));
  } else if (!bodyUpdate) {
    bodyLine.textContent = installedVersion;
    setBadge(bodyBadge, installed ? "ok" : "warning", installed ? tr("statusUpToDate") : tr("statusSetupNeeded"));
  } else {
    bodyLine.textContent = installed
      ? `${installedVersion} → ${update.available_version}`
      : `→ ${update.available_version}`;
    if (update.status === "revoked") {
      setBadge(bodyBadge, "error", tr("statusRevoked"));
    } else if (update.status === "install_required") {
      setBadge(bodyBadge, "warning", tr("statusSetupNeeded"));
    } else if (update.mandatory) {
      setBadge(bodyBadge, "warning", tr("statusMandatory"));
    } else {
      setBadge(bodyBadge, "available", tr("statusUpdateAvailable"));
    }
  }

  const installedLauncher = update?.installed_launcher_version || state.launcher_version;
  const availableLauncher = update?.available_launcher_version || installedLauncher;
  byId("launcher-version-line").textContent = launcherUpdate
    ? `${installedLauncher} → ${availableLauncher}`
    : installedLauncher;
  if (checking) {
    setBadge(byId("launcher-badge"), "neutral", tr("statusChecking"));
  } else if (launcherUpdate) {
    setBadge(
      byId("launcher-badge"),
      update?.status === "launcher_too_old" ? "error" : "available",
      update?.status === "launcher_too_old" ? tr("statusMandatory") : tr("statusUpdateAvailable")
    );
  } else {
    setBadge(byId("launcher-badge"), "ok", tr("statusUpToDate"));
  }

  const installedPlugin = installedPluginVersion();
  const availablePlugin = pluginStatus ? pluginVersionLabel(pluginStatus.available) : "";
  if (checking) {
    byId("plugin-version-line").textContent = installedPlugin;
    setBadge(byId("plugin-badge"), "neutral", tr("statusChecking"));
  } else if (pluginUnavailable || !pluginStatus) {
    byId("plugin-version-line").textContent = installedPlugin;
    setBadge(byId("plugin-badge"), "warning", tr("statusUnavailable"));
  } else if (pluginUpdate) {
    byId("plugin-version-line").textContent = `${installedPlugin} → ${availablePlugin}`;
    setBadge(byId("plugin-badge"), "available", tr("statusUpdateAvailable"));
  } else {
    byId("plugin-version-line").textContent = availablePlugin;
    setBadge(byId("plugin-badge"), "ok", tr("statusUpToDate"));
  }
  const partialBlocked = updateBlocksLaunch() || update?.status === "install_required";
  byId("body-update").hidden = !bodyUpdate || !installed || partialBlocked;
  byId("launcher-update").hidden = !launcherUpdate || partialBlocked;
  byId("plugin-update").hidden = !pluginUpdate;
  byId("body-update").disabled = checking || installingUpdate;
  byId("launcher-update").disabled = checking || installingUpdate;
  byId("plugin-update").disabled = checking || installingUpdate;
}

function renderReleasePanel() {
  const current = !update?.body_update_available && !update?.launcher_update_available;
  byId("release-notes-open").disabled = !update;
  if (!update) {
    byId("latest-release-time").textContent = "—";
    return;
  }
  byId("release-dialog-title").textContent = current
    ? tr("currentRelease").replace("{version}", update.available_version)
    : tr(update.status === "install_required" ? "installAvailable" : "available")
      .replace("{version}", update.available_version);
  const publishedAt = formatPublishedAt(update.available_published_at);
  byId("latest-release-time").textContent = publishedAt || "—";
  byId("available-published-at").textContent = publishedAt
    ? tr("publishedAt").replace("{datetime}", publishedAt)
    : "";
  renderSafeMarkdown(localizedReleaseNotes());
}

function renderUpdate() {
  if (!state) return;
  const installed = Boolean(state.installation_ready);
  const blocked = updateBlocksLaunch();
  const mainUpdates = Boolean(update?.body_update_available || update?.launcher_update_available);
  const pluginUpdate = Boolean(pluginStatus?.update_available);
  const hasUpdates = mainUpdates || pluginUpdate;
  const allCurrent = Boolean(update && pluginStatus) && !hasUpdates && !pluginUnavailable;
  byId("installation-status").textContent = pluginUnavailable && installed
    ? tr("pluginCheckUnavailable")
    : allCurrent
      ? tr("allCurrentDescription")
      : (installed ? tr("ready") : tr("notInstalled"));
  byId("play").disabled = !canLaunch();
  byId("configure").disabled = !canLaunch();
  byId("check").disabled = checking || installingUpdate || launching;
  byId("play").hidden = hasUpdates;
  byId("configure").hidden = hasUpdates;
  byId("update-all").hidden = !hasUpdates;
  byId("update-all").disabled = checking || installingUpdate;
  byId("launch-current").hidden = !hasUpdates || blocked || !installed;
  byId("launch-current").disabled = blocked || !installed || installingUpdate || launching;
  renderStatusCards();
  renderReleasePanel();
  renderAnnouncements();
  renderDeprecated();
  renderPlugins();
  if (installingUpdate) {
    setStatus(tr(update?.status === "install_required" ? "installing" : "updating"), "available");
    return;
  }
  if (launching) {
    setStatus(tr("launching"), "neutral");
    return;
  }
  if (checking) {
    setStatus(tr("checking"), "neutral");
    return;
  }
  if (!update) {
    if (state.cached_policy_invalid) {
      setStatus(tr("policyInvalid"), "error");
      return;
    }
    setStatus(tr(updateUnavailable ? (installed ? "unavailable" : "unavailableNoInstall") : "current"), updateUnavailable ? "warning" : "ok");
    return;
  }

  if (pluginUnavailable && !mainUpdates) {
    setStatus(tr("pluginCheckUnavailable"), "warning");
    return;
  }

  if (!hasUpdates) {
    setStatus(tr(updateUnavailable ? "unavailable" : "current"), updateUnavailable ? "warning" : "ok");
    return;
  }
  const installing = update.status === "install_required";
  if (update.status === "revoked") {
    setStatus(tr("revoked"), "error");
  } else if (update.status === "launcher_too_old") {
    setStatus(tr("launcherOld"), "error");
  } else if (update.mandatory) {
    setStatus(tr("mandatory"), "warning");
  } else {
    setStatus(
      installing
        ? tr("installAvailable").replace("{version}", update.available_version)
        : tr("updatesAvailable"),
      "available"
    );
  }
}

async function loadState() {
  try {
    if (!invoke) throw new Error("Tauri unavailable");
    state = await invoke("launcher_state");
    update = state.cached_update || null;
    renderUpdate();
  } catch (error) {
    state = null;
    setStatus(tr("invalid"), "error");
    showError(error);
  }
}

async function checkUpdate() {
  if (state?.update_configuration !== "BMSIR_ARENA_UPDATE_CONFIGURED_V1") {
    update = null;
    checking = false;
    updateUnavailable = true;
    renderUpdate();
    setStatus(tr("updateUnconfigured"), "warning");
    showError(tr("updateUnconfigured"));
    return;
  }
  checking = true;
  updateUnavailable = false;
  pluginUnavailable = false;
  renderUpdate();
  const [updateResult, pluginResult] = await Promise.allSettled([
    invoke("check_online_update"),
    invoke("check_plugin_update")
  ]);
  const errors = [];
  if (updateResult.status === "fulfilled") {
    update = updateResult.value;
    state.cached_update = update;
    state.cached_policy_invalid = false;
  } else {
    update = state.cached_update || update;
    updateUnavailable = true;
    errors.push(updateResult.reason);
  }
  if (pluginResult.status === "fulfilled") {
    pluginStatus = pluginResult.value;
  } else {
    pluginUnavailable = true;
    errors.push(pluginResult.reason);
  }
  if (errors.length) showError(errors.join("\n"));
  else hideError();
  checking = false;
  renderUpdate();
}

function showError(error) {
  byId("error-detail").textContent = String(error);
  byId("details").hidden = false;
}

function hideError() {
  byId("error-detail").textContent = "";
  byId("details").hidden = true;
}

async function launch(configuration = false) {
  if (launching) return;
  launching = true;
  latestLaunchExit = null;
  renderUpdate();
  hideError();
  try {
    const result = await invoke("launch_game", {configuration});
    if (result?.diagnostic) showError(result.diagnostic);
  } catch (error) {
    launching = false;
    renderUpdate();
    showError(error);
  }
}

function launchExitDetails(result) {
  const code = result?.exit_code == null ? tr("exitCodeUnavailable") : result.exit_code;
  const detail = [
    `${tr("exitCode")}: ${code}`,
    result?.log_path ? `${tr("launchLog")}: ${result.log_path}` : ""
  ];
  if (result?.diagnostic) detail.push(String(result.diagnostic));
  return detail.filter(Boolean).join("\n");
}

function reportLaunchExit(result) {
  if (!result) return;
  if (!result.success || result.short_lived || result.diagnostic) {
    const failed = !result.success;
    const status = failed
      ? "launchFailed"
      : (result.short_lived ? "launchEndedEarly" : "launchDiagnostic");
    setStatus(tr(status), failed ? "error" : "warning");
    showError(launchExitDetails(result));
  }
}

async function installSelected(target) {
  if (installingUpdate) return;
  installingUpdate = true;
  latestLaunchExit = null;
  updateProgress = {
    phase: "downloading",
    bytes_done: 0,
    bytes_total: Array.isArray(update?.artifacts)
      ? update.artifacts.reduce((total, artifact) => total + Number(artifact.size || 0), 0)
      : 0,
    files_done: 0,
    files_total: Array.isArray(update?.artifacts) ? update.artifacts.length : 0
  };
  renderUpdate();
  renderProgress();
  setStatus(tr(update?.status === "install_required" ? "installing" : "updating"), "available");
  try {
    const mainUpdateAvailable = Boolean(update?.body_update_available || update?.launcher_update_available);
    if (target === "all" && pluginStatus?.update_available) {
      await invoke("install_plugin_version", {version: pluginStatus.available.version});
    }
    if (mainUpdateAvailable || target !== "all") {
      await invoke("install_online_update", {target, launchAfter: false});
    }
    await loadState();
    if (state) await checkUpdate();
    installingUpdate = false;
    updateProgress = null;
    renderProgress();
    renderUpdate();
    hideError();
  } catch (error) {
    installingUpdate = false;
    updateProgress = null;
    renderProgress();
    showError(error);
    renderUpdate();
  }
}

byId("language").addEventListener("click", () => {
  language = language === "ja" ? "en" : "ja";
  localStorage.setItem("bmsir-launcher-language", language);
  applyLanguage();
});
byId("play").addEventListener("click", () => launch(false));
byId("configure").addEventListener("click", () => launch(true));
byId("check").addEventListener("click", checkUpdate);
byId("update-all").addEventListener("click", () => installSelected("all"));
byId("body-update").addEventListener("click", () => installSelected("body"));
byId("launcher-update").addEventListener("click", () => installSelected("launcher"));
byId("plugin-update").addEventListener("click", () => {
  if (pluginStatus?.available?.version) installPlugin(pluginStatus.available.version);
});
byId("launch-current").addEventListener("click", () => launch(false));
byId("deprecated-toggle").addEventListener("click", toggleDeprecated);
byId("plugin-toggle").addEventListener("click", togglePlugins);
byId("release-notes-open").addEventListener("click", () => openDialog(byId("release-dialog")));
byId("release-dialog-close").addEventListener("click", () => byId("release-dialog").close());
byId("settings-open").addEventListener("click", () => openDialog(byId("settings-dialog")));
byId("settings-close").addEventListener("click", () => byId("settings-dialog").close());
byId("bmsir-account-open").addEventListener("click", () => openQuickTool("bmsir"));
byId("bmsir-settings-open").addEventListener("click", () => openQuickTool("bmsir"));
document.querySelectorAll("[data-quick-tool]").forEach(button => {
  button.addEventListener("click", () => openQuickTool(button.dataset.quickTool));
});
byId("quick-tool-dialog-close").addEventListener("click", closeQuickTool);
byId("quick-tool-dialog-done").addEventListener("click", closeQuickTool);
byId("resident-state").addEventListener("click", () => saveSetting("resident", !launcherSettings?.resident));
byId("setting-resident").addEventListener("change", event => saveSetting("resident", event.target.checked));
byId("setting-background-check").addEventListener("change", event => saveSetting("background_check", event.target.checked));
byId("setting-autostart").addEventListener("change", event => saveSetting("autostart", event.target.checked));
[byId("release-dialog"), byId("settings-dialog"), byId("quick-tool-dialog")].forEach(dialog => {
  dialog.addEventListener("click", event => {
    const bounds = dialog.getBoundingClientRect();
    const inside = event.clientX >= bounds.left && event.clientX <= bounds.right
      && event.clientY >= bounds.top && event.clientY <= bounds.bottom;
    if (!inside) {
      if (dialog.id === "quick-tool-dialog") closeQuickTool();
      else dialog.close();
    }
  });
});
byId("quick-tool-dialog").addEventListener("close", () => {
  activeQuickTool = null;
  byId("quick-tool-fields").replaceChildren();
});

if (listen) {
  await listen("arena-update-progress", event => {
    if (!installingUpdate) return;
    updateProgress = event.payload;
    renderProgress();
  });
  await listen("arena-launch-exit", event => {
    const result = event.payload;
    latestLaunchExit = result;
    launching = false;
    renderUpdate();
    reportLaunchExit(result);
  });
  await listen("arena-tray-check-requested", checkUpdate);
  await listen("arena-background-update-available", event => {
    update = event.payload;
    if (state) state.cached_update = update;
    renderUpdate();
  });
}

applyLanguage();
await loadSettings();
await loadState();
if (state) await checkUpdate();
