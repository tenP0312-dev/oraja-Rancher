const invoke = window.__TAURI__?.core?.invoke;

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
    updateUnconfigured: "このランチャーには更新先が設定されていません",
    switchLanguage: "英語に切り替え"
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
    updateUnconfigured: "This launcher has no update endpoint configured",
    switchLanguage: "Switch to Japanese"
  }
};

let language = localStorage.getItem("bmsir-launcher-language") === "en" ? "en" : "ja";
let state = null;
let update = null;
const byId = id => document.getElementById(id);
const tr = key => dictionary[language][key];

function applyLanguage() {
  document.documentElement.lang = language;
  document.querySelectorAll("[data-i18n]").forEach(element => {
    element.textContent = tr(element.dataset.i18n);
  });
  byId("language").textContent = language === "ja" ? "日本語" : "EN";
  byId("language").setAttribute("aria-label", tr("switchLanguage"));
  byId("language").title = tr("switchLanguage");
  renderUpdate();
}

function setStatus(text, kind = "neutral") {
  byId("update-status").textContent = text;
  byId("status-mark").dataset.kind = kind;
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
  return Boolean(state?.installation_ready);
}

function renderUpdate() {
  if (!state) return;
  const ready = canLaunch();
  byId("installation-status").textContent = ready ? tr("ready") : tr("notInstalled");
  byId("play").disabled = !ready;
  byId("configure").disabled = !ready;
  const version = ready ? state.installed_version : tr("notInstalled");
  byId("version").textContent = `${version}  /  ${state.channel}`;
  byId("update-panel").hidden = !update || update.status === "current";
  if (!update) return;

  if (update.status === "current") {
    setStatus(tr("current"), "ok");
    return;
  }
  const installing = update.status === "install_required";
  const title = installing ? tr("installAvailable") : tr("available");
  byId("available-title").textContent = title.replace("{version}", update.available_version);
  byId("update-launch").textContent = installing ? tr("installLaunch") : tr("updateLaunch");
  renderSafeMarkdown(update.release_notes_markdown);
  const blocked = installing || update.mandatory || update.status === "revoked" || update.status === "launcher_too_old";
  byId("launch-current").disabled = blocked || !ready;
  byId("play").disabled = blocked || !ready;
  byId("configure").disabled = blocked || !ready;
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
    setStatus(tr("updateUnconfigured"), "warning");
    showError(tr("updateUnconfigured"));
    return;
  }
  setStatus(tr("checking"), "neutral");
  byId("check").disabled = true;
  try {
    update = await invoke("check_online_update");
    hideError();
    renderUpdate();
  } catch (error) {
    update = null;
    setStatus(tr(canLaunch() ? "unavailable" : "unavailableNoInstall"), "warning");
    showError(error);
  } finally {
    byId("check").disabled = false;
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
  setStatus(tr(update?.status === "install_required" ? "installing" : "updating"), "available");
  byId("update-launch").disabled = true;
  try {
    await invoke("install_online_update", {launchAfter: true});
  } catch (error) {
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

applyLanguage();
await loadState();
if (state) await checkUpdate();
