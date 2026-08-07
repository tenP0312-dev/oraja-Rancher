const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;
const openUrl = window.__TAURI__?.opener?.openUrl;

const dictionary = {
  ja: {
    checking: "更新を確認しています",
    current: "最新です",
    unavailable: "更新を確認できませんでした。現在の本体は起動できます",
    unavailableNoInstall: "セットアップファイルを取得できませんでした",
    invalid: "本体JAR、Java 21以上、またはArenaプラグインを確認できません",
    notInstalled: "本体はまだインストールされていません",
    ready: "本体、Java、Arenaプラグインを確認しました",
    play: "Arenaを起動",
    configure: "起動前コンフィグ",
    check: "更新を確認",
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
    information: "お知らせ",
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
    deprecatedToggle: "非推奨版から選択",
    deprecatedHide: "非推奨版一覧を閉じる",
    deprecatedLoading: "読み込んでいます…",
    deprecatedEmpty: "非推奨版はありません",
    deprecatedLoadError: "非推奨版の一覧を取得できませんでした",
    deprecatedMoreLink: "もっと古い版はこちら(GitHub)",
    downgradeButton: "ダウングレード",
    downgradeConfirm: "本体を {version}（配信日時: {datetime}）にダウングレードします。Java、プラグイン、設定、スキン、スコアデータは変更されません。よろしいですか？",
    downgradeSuccess: "ダウングレードが完了しました",
    deprecatedKindTest: "(旧テストビルド)",
    deprecatedKindStable: "(旧安定版)",
    statusBody: "本体",
    statusLauncher: "ランチャー",
    statusChecking: "確認中",
    statusUpToDate: "最新",
    statusSetupNeeded: "セットアップ必要",
    statusUpdateAvailable: "更新あり",
    statusMandatory: "必須更新",
    statusRevoked: "利用停止中",
    statusLauncherTooOld: "ランチャー要更新",
    updateOnly: "更新のみ",
    releaseNotesToggle: "リリースノートを見る",
    availableSize: "ダウンロードサイズ: 約{size}",
    settingsOpen: "設定",
    settingsTitle: "設定",
    settingsResident: "常駐(トレイ)を有効にする",
    settingsResidentHint: "閉じてもトレイに残り続けます",
    settingsAutostart: "ログイン時に自動起動",
    settingsAutostartHint: "常駐が必要です",
    settingsBackgroundCheck: "バックグラウンド自動チェック",
    settingsBackgroundCheckHint: "1日1回、自動で更新を確認し、見つかったら通知します",
    settingsSaveError: "設定を保存できませんでした",
    notificationUpdateTitle: "BMS-IR Arena {version} が利用できます",
    notificationUpdateBody: "ランチャーを開いて更新してください"
  },
  en: {
    checking: "Checking for updates",
    current: "Up to date",
    unavailable: "Could not check for updates. The installed version can still launch",
    unavailableNoInstall: "Could not download the setup information",
    invalid: "The game JAR, Java 21+, or Arena plugin could not be found",
    notInstalled: "The game is not installed yet",
    ready: "Game, Java, and Arena plugin are ready",
    play: "Launch Arena",
    configure: "Pre-launch configuration",
    check: "Check for updates",
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
    information: "Information",
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
    deprecatedToggle: "Choose a deprecated version",
    deprecatedHide: "Hide deprecated versions",
    deprecatedLoading: "Loading…",
    deprecatedEmpty: "No deprecated versions are available",
    deprecatedLoadError: "Could not load the deprecated version list",
    deprecatedMoreLink: "Older releases (GitHub)",
    downgradeButton: "Downgrade",
    downgradeConfirm: "Downgrade the game to {version} (published {datetime}). Java, the plugin, settings, skins, and score data will not be touched. Continue?",
    downgradeSuccess: "Downgrade complete",
    deprecatedKindTest: "(older test build)",
    deprecatedKindStable: "(older stable build)",
    statusBody: "Body",
    statusLauncher: "Launcher",
    statusChecking: "Checking",
    statusUpToDate: "Up to date",
    statusSetupNeeded: "Setup needed",
    statusUpdateAvailable: "Update available",
    statusMandatory: "Required update",
    statusRevoked: "Revoked",
    statusLauncherTooOld: "Launcher needs updating",
    updateOnly: "Update only",
    releaseNotesToggle: "View release notes",
    availableSize: "Download size: about {size}",
    settingsOpen: "Settings",
    settingsTitle: "Settings",
    settingsResident: "Enable tray residency",
    settingsResidentHint: "Keeps running in the tray after you close the window",
    settingsAutostart: "Launch at login",
    settingsAutostartHint: "Requires tray residency",
    settingsBackgroundCheck: "Automatic background check",
    settingsBackgroundCheckHint: "Checks for updates once a day and notifies you if one is found",
    settingsSaveError: "Could not save settings",
    notificationUpdateTitle: "BMS-IR Arena {version} is available",
    notificationUpdateBody: "Open the launcher to update"
  }
};

// Only the launcher's own recent history is offered as a one-click
// downgrade; anything older is still on GitHub and linked from the panel
// instead of being kept in the signed publication tree indefinitely.
const RELEASES_PAGE_URL = "https://github.com/tenP0312-dev/bms-ir-arena-patch-server/releases";

let language = localStorage.getItem("bmsir-launcher-language") === "en" ? "en" : "ja";
let state = null;
let update = null;
let checking = true;
let updateUnavailable = false;
let installingUpdate = false;
let updateProgress = null;
let deprecatedVersions = null;
let deprecatedVisible = false;
let deprecatedLoading = false;
let downgradingVersion = null;
let settingsVisible = false;
let launcherSettings = null;
const deprecatedNotesCache = {};
const byId = id => document.getElementById(id);
const tr = key => dictionary[language][key];

function applyLanguage() {
  document.documentElement.lang = language;
  document.querySelectorAll("[data-i18n]").forEach(element => {
    element.textContent = tr(element.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach(element => {
    const label = tr(element.dataset.i18nAriaLabel);
    element.setAttribute("aria-label", label);
    element.title = label;
  });
  byId("language-label").textContent = language === "ja" ? "日本語" : "English";
  byId("language").setAttribute("aria-label", tr("switchLanguage"));
  byId("language").title = tr("switchLanguage");
  renderUpdate();
  renderProgress();
  renderDeprecated();
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
  byId("deprecated-more-link").href = RELEASES_PAGE_URL;
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

function canLaunch() {
  return Boolean(state?.installation_ready)
    && !state?.cached_policy_invalid
    && !checking
    && !updateBlocksLaunch();
}

function updateBlocksLaunch() {
  if (!update) return false;
  return update.status === "install_required"
    || update.status === "revoked"
    || update.status === "launcher_too_old"
    || (update.status === "available" && update.mandatory);
}

function pickLocalizedNotes(notes) {
  if (!notes) return "";
  const localized = language === "ja"
    ? notes.release_notes_markdown_ja
    : notes.release_notes_markdown_en;
  return localized || notes.release_notes_markdown || "";
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

function renderStatusCards() {
  if (!state) return;
  const installed = Boolean(state.installation_ready);
  const installedVersion = installed ? state.installed_version : tr("notInstalledVersion");
  const bodyLine = byId("body-version-line");
  const bodyBadge = byId("body-badge");

  if (checking) {
    bodyLine.textContent = installedVersion;
    setBadge(bodyBadge, "neutral", tr("statusChecking"));
  } else if (!update || update.status === "current") {
    bodyLine.textContent = installedVersion;
    setBadge(bodyBadge, installed ? "ok" : "warning", installed ? tr("statusUpToDate") : tr("statusSetupNeeded"));
  } else {
    bodyLine.textContent = installed
      ? `${installedVersion} → ${update.available_version}`
      : `→ ${update.available_version}`;
    if (update.status === "revoked") {
      setBadge(bodyBadge, "error", tr("statusRevoked"));
    } else if (update.status === "launcher_too_old") {
      setBadge(bodyBadge, "error", tr("statusLauncherTooOld"));
    } else if (update.status === "install_required") {
      setBadge(bodyBadge, "warning", tr("statusSetupNeeded"));
    } else if (update.mandatory) {
      setBadge(bodyBadge, "warning", tr("statusMandatory"));
    } else {
      setBadge(bodyBadge, "available", tr("statusUpdateAvailable"));
    }
  }

  byId("launcher-version-line").textContent = state.launcher_version;
  // The launcher does not yet publish a "newer launcher available" signal
  // separate from the body (see the deprecated-version-downgrade PR notes:
  // launcher self-update is wired but no manifest has shipped a launcher
  // artifact yet), so this always reads as up to date for now.
  setBadge(byId("launcher-badge"), "ok", tr("statusUpToDate"));
}

function renderUpdate() {
  if (!state) return;
  const installed = Boolean(state.installation_ready);
  const blocked = updateBlocksLaunch();
  byId("installation-status").textContent = installed ? tr("ready") : tr("notInstalled");
  byId("play").disabled = !canLaunch();
  byId("configure").disabled = !canLaunch();
  byId("check").disabled = checking;
  renderStatusCards();
  byId("update-panel").hidden = !update || update.status === "current";
  renderAnnouncements();
  renderDeprecated();
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

  if (update.status === "current") {
    setStatus(tr(updateUnavailable ? "unavailable" : "current"), updateUnavailable ? "warning" : "ok");
    return;
  }
  const installing = update.status === "install_required";
  const title = installing ? tr("installAvailable") : tr("available");
  byId("available-title").textContent = title.replace("{version}", update.available_version);
  const publishedAt = formatPublishedAt(update.available_published_at);
  byId("available-published-at").textContent = publishedAt
    ? tr("publishedAt").replace("{datetime}", publishedAt)
    : "";
  const totalBytes = Number(update.total_artifact_bytes) || 0;
  byId("available-size").textContent = totalBytes > 0
    ? tr("availableSize").replace("{size}", formatBytes(totalBytes))
    : "";
  byId("update-launch").textContent = installing ? tr("installLaunch") : tr("updateLaunch");
  byId("update-only").hidden = installing;
  renderSafeMarkdown(localizedReleaseNotes());
  byId("launch-current").disabled = blocked || !installed;
  byId("play").disabled = blocked || !installed;
  byId("configure").disabled = blocked || !installed;
  byId("update-only").disabled = blocked;
  if (update.status === "revoked") {
    setStatus(tr("revoked"), "error");
  } else if (update.status === "launcher_too_old") {
    setStatus(tr("launcherOld"), "error");
  } else if (update.mandatory) {
    setStatus(tr("mandatory"), "warning");
  } else if (installing) {
    setStatus(byId("available-title").textContent, "available");
  } else {
    setStatus(byId("available-title").textContent, "available");
  }
}

function renderSettings() {
  byId("settings-view").hidden = !settingsVisible;
  byId("main-view").hidden = settingsVisible;
  if (!settingsVisible || !launcherSettings) return;
  byId("setting-resident").checked = launcherSettings.resident;
  byId("setting-autostart").checked = launcherSettings.autostart;
  byId("setting-background-check").checked = launcherSettings.background_check;
  // Autostart only makes sense while the launcher can actually stay
  // resident to be woken back up; keep it visibly tied to that toggle
  // instead of silently ignoring it.
  byId("setting-autostart").disabled = !launcherSettings.resident;
}

async function toggleSettings() {
  settingsVisible = !settingsVisible;
  renderSettings();
  if (settingsVisible && !launcherSettings && invoke) {
    try {
      launcherSettings = await invoke("get_launcher_settings");
    } catch (error) {
      launcherSettings = {resident: false, autostart: false, background_check: false};
      showError(error);
    }
    renderSettings();
  }
}

async function updateLauncherSetting(key, value) {
  if (!launcherSettings) return;
  const previous = launcherSettings[key];
  launcherSettings = {...launcherSettings, [key]: value};
  if (key === "resident" && !value) {
    // Autostart without residency would just reopen a normal window on
    // login with nothing keeping it running; turn it off alongside.
    launcherSettings.autostart = false;
  }
  renderSettings();
  try {
    await invoke("set_launcher_settings", {settings: launcherSettings});
    hideError();
  } catch (error) {
    launcherSettings = {...launcherSettings, [key]: previous};
    renderSettings();
    showError(error);
    setStatus(tr("settingsSaveError"), "error");
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
  renderUpdate();
  try {
    update = await invoke("check_online_update");
    state.cached_update = update;
    state.cached_policy_invalid = false;
    hideError();
  } catch (error) {
    update = state.cached_update || update;
    updateUnavailable = true;
    showError(error);
  } finally {
    checking = false;
    renderUpdate();
  }
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
  try {
    await invoke("launch_game", {configuration});
  } catch (error) {
    showError(error);
  }
}

async function installAndLaunch(launchAfter) {
  installingUpdate = true;
  updateProgress = {
    phase: "downloading",
    bytes_done: 0,
    bytes_total: Number(update?.total_artifact_bytes) || 0,
    files_done: 0,
    files_total: 0
  };
  renderProgress();
  setStatus(tr(update?.status === "install_required" ? "installing" : "updating"), "available");
  byId("update-launch").disabled = true;
  byId("update-only").disabled = true;
  try {
    await invoke("install_online_update", {launchAfter});
    if (!launchAfter) {
      hideError();
      await loadState();
      if (state) await checkUpdate();
    }
  } catch (error) {
    showError(error);
    renderUpdate();
  } finally {
    installingUpdate = false;
    updateProgress = null;
    renderProgress();
    byId("update-launch").disabled = false;
    byId("update-only").disabled = false;
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
byId("update-launch").addEventListener("click", () => installAndLaunch(true));
byId("update-only").addEventListener("click", () => installAndLaunch(false));
byId("settings-open").addEventListener("click", toggleSettings);
byId("setting-resident").addEventListener("change", event => updateLauncherSetting("resident", event.target.checked));
byId("setting-autostart").addEventListener("change", event => updateLauncherSetting("autostart", event.target.checked));
byId("setting-background-check").addEventListener("change", event => updateLauncherSetting("background_check", event.target.checked));
byId("launch-current").addEventListener("click", () => launch(false));
byId("deprecated-toggle").addEventListener("click", toggleDeprecated);
byId("deprecated-more-link").addEventListener("click", event => {
  // A plain <a target="_blank"> silently does nothing in the Tauri
  // webview (external navigation is not the OS browser); open it through
  // the opener plugin instead.
  event.preventDefault();
  if (openUrl) {
    openUrl(RELEASES_PAGE_URL).catch(error => showError(error));
  }
});

if (listen) {
  await listen("arena-update-progress", event => {
    if (!installingUpdate) return;
    updateProgress = event.payload;
    renderProgress();
  });
}

applyLanguage();
await loadState();
if (state) await checkUpdate();
