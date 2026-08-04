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
    bodyVersion: "本体 {version} / {channel}",
    notInstalledVersion: "未インストール",
    launcherVersion: "Launcher {version}",
    downloading: "本体をダウンロード中",
    extracting: "本体を展開・検証中",
    verifying: "ダウンロードしたファイルを検証中",
    applying: "更新を適用中",
    restarting: "新しいランチャーを起動中",
    progressFiles: "{done} / {total} ファイル"
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
    bodyVersion: "Body {version} / {channel}",
    notInstalledVersion: "Not installed",
    launcherVersion: "Launcher {version}",
    downloading: "Downloading game files",
    extracting: "Extracting and verifying game files",
    verifying: "Verifying downloaded files",
    applying: "Applying update",
    restarting: "Starting the updated launcher",
    progressFiles: "{done} / {total} files"
  }
};

let language = localStorage.getItem("bmsir-launcher-language") === "en" ? "en" : "ja";
let state = null;
let update = null;
let checking = true;
let updateUnavailable = false;
let installingUpdate = false;
let updateProgress = null;
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
}

function setStatus(text, kind = "neutral") {
  byId("update-status").textContent = text;
  byId("status-mark").dataset.kind = kind;
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

function renderSafeMarkdown(markdown) {
  const target = byId("release-notes");
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

function localizedReleaseNotes() {
  if (!update) return "";
  const localized = language === "ja"
    ? update.release_notes_markdown_ja
    : update.release_notes_markdown_en;
  return localized || update.release_notes_markdown || "";
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

function renderUpdate() {
  if (!state) return;
  const installed = Boolean(state.installation_ready);
  const blocked = updateBlocksLaunch();
  byId("installation-status").textContent = installed ? tr("ready") : tr("notInstalled");
  byId("play").disabled = !canLaunch();
  byId("configure").disabled = !canLaunch();
  byId("check").disabled = checking;
  const version = installed ? state.installed_version : tr("notInstalledVersion");
  byId("version").textContent = tr("bodyVersion")
    .replace("{version}", version)
    .replace("{channel}", state.channel);
  byId("launcher-version").textContent = tr("launcherVersion")
    .replace("{version}", state.launcher_version);
  byId("update-panel").hidden = !update || update.status === "current";
  renderAnnouncements();
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
  byId("update-launch").textContent = installing ? tr("installLaunch") : tr("updateLaunch");
  renderSafeMarkdown(localizedReleaseNotes());
  byId("launch-current").disabled = blocked || !installed;
  byId("play").disabled = blocked || !installed;
  byId("configure").disabled = blocked || !installed;
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
  try {
    await invoke("launch_game", {configuration});
  } catch (error) {
    showError(error);
  }
}

async function installAndLaunch() {
  installingUpdate = true;
  updateProgress = {
    phase: "downloading",
    bytes_done: 0,
    bytes_total: Array.isArray(update?.artifacts)
      ? update.artifacts.reduce((total, artifact) => total + Number(artifact.size || 0), 0)
      : 0,
    files_done: 0,
    files_total: Array.isArray(update?.artifacts) ? update.artifacts.length : 0
  };
  renderProgress();
  setStatus(tr(update?.status === "install_required" ? "installing" : "updating"), "available");
  byId("update-launch").disabled = true;
  try {
    await invoke("install_online_update", {launchAfter: true});
  } catch (error) {
    installingUpdate = false;
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
