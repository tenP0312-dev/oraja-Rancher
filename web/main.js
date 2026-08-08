const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;

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
    launching: "Arenaを起動しています",
    launchFailed: "Arenaが異常終了しました",
    launchEndedEarly: "Arenaがすぐに終了しました",
    launchDiagnostic: "Arenaの起動診断を確認してください",
    exitCode: "終了コード",
    exitCodeUnavailable: "取得できません",
    launchLog: "診断ログ",
    deprecatedToggle: "非推奨版から選択",
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
    currentRelease: "現在の本体 {version}",
    noReleaseNotes: "この版のリリースノートはありません",
    statusBody: "本体",
    statusLauncher: "ランチャー",
    statusChecking: "確認中",
    statusUpToDate: "最新",
    statusSetupNeeded: "セットアップ必要",
    statusUpdateAvailable: "更新あり",
    statusMandatory: "必須更新",
    statusRevoked: "利用停止中",
    statusLauncherTooOld: "ランチャー要更新",
    pluginToggle: "Arenaプラグインの更新・旧版",
    pluginCurrent: "プラグイン更新あり: {version}",
    pluginCurrentOk: "プラグインは最新です",
    pluginInstall: "このプラグインを適用",
    pluginDeprecated: "旧プラグイン版",
    pluginReleaseVersion: "プラグイン {plugin} / 本体リリース {release}"
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
    launching: "Launching Arena",
    launchFailed: "Arena exited with an error",
    launchEndedEarly: "Arena exited shortly after launch",
    launchDiagnostic: "Check the Arena launch diagnostic",
    exitCode: "Exit code",
    exitCodeUnavailable: "Unavailable",
    launchLog: "Diagnostic log",
    deprecatedToggle: "Choose a deprecated version",
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
    currentRelease: "Installed body {version}",
    noReleaseNotes: "No release notes are available for this version",
    statusBody: "Body",
    statusLauncher: "Launcher",
    statusChecking: "Checking",
    statusUpToDate: "Up to date",
    statusSetupNeeded: "Setup needed",
    statusUpdateAvailable: "Update available",
    statusMandatory: "Required update",
    statusRevoked: "Revoked",
    statusLauncherTooOld: "Launcher needs updating",
    pluginToggle: "Arena plugin updates and older versions",
    pluginCurrent: "Plugin update available: {version}",
    pluginCurrentOk: "The plugin is up to date",
    pluginInstall: "Apply this plugin",
    pluginDeprecated: "Older plugin release",
    pluginReleaseVersion: "Plugin {plugin} / body release {release}"
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
let pluginUpdate = null;
const deprecatedNotesCache = {};
const byId = id => document.getElementById(id);
const tr = key => dictionary[language][key];

function applyLanguage() {
  document.documentElement.lang = language;
  document.querySelectorAll("[data-i18n]").forEach(element => {
    element.textContent = tr(element.dataset.i18n);
  });
  byId("language-label").textContent = language === "ja" ? "日本語" : "English";
  byId("language").setAttribute("aria-label", tr("switchLanguage"));
  byId("language").title = tr("switchLanguage");
  renderUpdate();
  renderProgress();
  renderDeprecated();
  renderPlugins();
}

function renderPlugins() {
  const toggle = byId("plugin-toggle");
  const container = byId("plugin-list-container");
  if (!toggle || !container) return;
  toggle.setAttribute("aria-expanded", String(pluginVisible));
  container.hidden = !pluginVisible;
  if (!pluginVisible) return;
  byId("plugin-current").textContent = pluginUpdate
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
      [pluginUpdate, pluginVersions] = await Promise.all([
        invoke("check_plugin_update"), invoke("list_deprecated_plugin_versions")
      ]);
    } catch (error) { showError(error); pluginVersions = []; }
  }
  renderPlugins();
}

async function installPlugin(version) {
  installingUpdate = true; updateProgress = {phase:"downloading", bytes_done:0, bytes_total:0, files_done:0, files_total:1};
  renderUpdate(); renderProgress();
  try {
    await invoke("install_plugin_version", {version});
    [pluginUpdate, pluginVersions] = await Promise.all([
      invoke("check_plugin_update"), invoke("list_deprecated_plugin_versions")
    ]);
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
    || (update.status === "available" && update.mandatory);
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
  setBadge(byId("launcher-badge"), "ok", tr("statusUpToDate"));
}

function renderReleasePanel() {
  const panel = byId("update-panel");
  if (!update) {
    panel.hidden = true;
    return;
  }
  const current = update.status === "current";
  panel.hidden = false;
  byId("available-title").textContent = current
    ? tr("currentRelease").replace("{version}", update.available_version)
    : tr(update.status === "install_required" ? "installAvailable" : "available")
      .replace("{version}", update.available_version);
  const publishedAt = formatPublishedAt(update.available_published_at);
  byId("available-published-at").textContent = publishedAt
    ? tr("publishedAt").replace("{datetime}", publishedAt)
    : "";
  byId("update-actions").hidden = current;
  renderSafeMarkdown(localizedReleaseNotes());
}

function renderUpdate() {
  if (!state) return;
  const installed = Boolean(state.installation_ready);
  const blocked = updateBlocksLaunch();
  byId("installation-status").textContent = installed ? tr("ready") : tr("notInstalled");
  byId("play").disabled = !canLaunch();
  byId("configure").disabled = !canLaunch();
  byId("check").disabled = checking || installingUpdate || launching;
  byId("launch-current").disabled = blocked || !installed || installingUpdate || launching;
  renderStatusCards();
  renderReleasePanel();
  renderAnnouncements();
  renderDeprecated();
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

  if (update.status === "current") {
    setStatus(tr(updateUnavailable ? "unavailable" : "current"), updateUnavailable ? "warning" : "ok");
    return;
  }
  const installing = update.status === "install_required";
  byId("update-launch").textContent = installing ? tr("installLaunch") : tr("updateLaunch");
  byId("launch-current").disabled = blocked || !installed || installingUpdate || launching;
  byId("play").disabled = blocked || !installed || launching;
  byId("configure").disabled = blocked || !installed || launching;
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

async function installAndLaunch() {
  installingUpdate = true;
  launching = true;
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
  byId("update-launch").disabled = true;
  try {
    const result = await invoke("install_online_update", {launchAfter: true});
    await loadState();
    installingUpdate = false;
    updateProgress = null;
    renderProgress();
    renderUpdate();
    if (result?.diagnostic) showError(result.diagnostic);
    reportLaunchExit(latestLaunchExit);
  } catch (error) {
    installingUpdate = false;
    launching = false;
    updateProgress = null;
    renderProgress();
    byId("update-launch").disabled = false;
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
byId("update-launch").addEventListener("click", installAndLaunch);
byId("launch-current").addEventListener("click", () => launch(false));
byId("deprecated-toggle").addEventListener("click", toggleDeprecated);
byId("plugin-toggle").addEventListener("click", togglePlugins);

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
}

applyLanguage();
await loadState();
if (state) await checkUpdate();
