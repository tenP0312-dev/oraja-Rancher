const invoke = window.__TAURI__?.core?.invoke;

const dictionary = {
  ja: {
    checking: "更新を確認しています",
    current: "最新です",
    unavailable: "更新を確認できませんでした。現在の本体は起動できます",
    invalid: "本体JARまたはJava 21以上を確認できません",
    ready: "本体とJavaを確認しました",
    play: "Arenaを起動",
    configure: "起動前コンフィグ",
    check: "更新を確認",
    updateLaunch: "更新して起動",
    launchCurrent: "現在の版を起動",
    available: "BMS-IR Arena {version} が利用できます",
    mandatory: "この更新は必須です",
    revoked: "現在の版は停止されています。更新が必要です",
    launcherOld: "ランチャーの更新が必要です",
    updating: "更新ファイルを検証して適用しています",
    details: "詳細"
  },
  en: {
    checking: "Checking for updates",
    current: "Up to date",
    unavailable: "Could not check for updates. The installed version can still launch",
    invalid: "The game JAR or Java 21+ could not be found",
    ready: "Game and Java are ready",
    play: "Launch Arena",
    configure: "Pre-launch configuration",
    check: "Check for updates",
    updateLaunch: "Update and launch",
    launchCurrent: "Launch installed version",
    available: "BMS-IR Arena {version} is available",
    mandatory: "This update is required",
    revoked: "The installed version has been revoked and must be updated",
    launcherOld: "The launcher must be updated",
    updating: "Verifying and applying the update",
    details: "Details"
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
  byId("language").textContent = language === "ja" ? "EN" : "日本語";
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
  return Boolean(state?.installation?.game_jar && state?.installation?.java_runtime);
}

function renderUpdate() {
  if (!state) return;
  const ready = canLaunch();
  byId("installation-status").textContent = ready ? tr("ready") : tr("invalid");
  byId("play").disabled = !ready;
  byId("configure").disabled = !ready;
  byId("version").textContent = `${state.installed_version}  /  ${state.channel}`;
  byId("update-panel").hidden = !update || update.status === "current";
  if (!update) return;

  if (update.status === "current") {
    setStatus(tr("current"), "ok");
    return;
  }
  byId("available-title").textContent = tr("available").replace("{version}", update.available_version);
  renderSafeMarkdown(update.release_notes_markdown);
  const blocked = update.mandatory || update.status === "revoked" || update.status === "launcher_too_old";
  byId("launch-current").disabled = blocked || !ready;
  byId("play").disabled = blocked || !ready;
  byId("configure").disabled = blocked || !ready;
  if (update.status === "revoked") {
    setStatus(tr("revoked"), "error");
  } else if (update.status === "launcher_too_old") {
    setStatus(tr("launcherOld"), "error");
  } else if (update.mandatory) {
    setStatus(tr("mandatory"), "warning");
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
  setStatus(tr("checking"), "neutral");
  byId("check").disabled = true;
  try {
    update = await invoke("check_online_update");
    hideError();
    renderUpdate();
  } catch (error) {
    update = null;
    setStatus(tr("unavailable"), "warning");
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
  setStatus(tr("updating"), "available");
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
if (canLaunch()) await checkUpdate();
